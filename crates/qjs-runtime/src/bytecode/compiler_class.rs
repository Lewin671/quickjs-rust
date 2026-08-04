use std::rc::Rc;

use qjs_ast::{
    ArrayElement, AssignmentTarget, AssignmentTargetPropertyKey, BindingPattern, CallArgument,
    ClassBody, ClassElement, ClassMemberKey, Expr, FunctionParams, MemberProperty, MethodKind,
    ObjectPropertyKey, Span, Stmt,
};

use crate::{
    RuntimeError,
    function::{collect_function_local_names, rest_parameter_binding_name},
};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::compiler_lexical::LexicalCapture;
use super::ir::{
    Bytecode, ClassComputedKeyDef, ClassConstructorDef, ClassDefinition, ClassElementDef,
    ClassFieldDef, ClassFieldInitializerDef, ClassMemberKeyDef, ClassMethodDef, ClassMethodKind,
    ClassPrivateElementDef, ClassStaticBlockDef, Op,
};

impl Compiler {
    pub(super) fn compile_class_expression(
        &mut self,
        name: Option<&str>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        let Some(name) = name else {
            return self.compile_class(None, body);
        };

        self.with_lexical_scope(|compiler| {
            let storage_name = format!("\0class_expr_inner:{}:{}", name, compiler.locals.len());
            let slot = compiler.declare_lexical_slot_with_storage_name(name, &storage_name, false);
            compiler.emit(Op::ClearLocal(slot));
            compiler.compile_class(Some(name), body)?;
            compiler.emit(Op::Dup);
            compiler.emit(Op::StoreLocal(slot));
            Ok(())
        })
    }

    /// Compiles a class declaration or expression into a `NewClass` op that
    /// builds the constructor function object at runtime. The class name (when
    /// present) is used for the constructor `name` property and the bindable
    /// inner name.
    ///
    /// Computed member keys are compiled into thunks and evaluated, in
    /// class-definition order, by `NewClass` after private names are bound.
    pub(super) fn compile_class(
        &mut self,
        name: Option<&str>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        let previous_strict = self.strict;
        self.strict = true;
        let result = self.compile_class_strict(name, body);
        self.strict = previous_strict;
        result
    }

    fn compile_class_strict(
        &mut self,
        name: Option<&str>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        // Evaluate the heritage expression first so the parent constructor is
        // beneath the computed keys when `NewClass` runs.
        let has_heritage = body.heritage.is_some();
        if let Some(heritage) = &body.heritage {
            self.compile_expr(heritage)?;
        }

        let mut computed_keys = Vec::new();
        for element in &body.elements {
            let key = match element {
                ClassElement::Method(member) => &member.key,
                ClassElement::Field(field) => &field.key,
                // Static blocks have no key.
                ClassElement::StaticBlock(_) => continue,
            };
            if let ClassMemberKey::Computed(expr) = key {
                if expr_contains_private_name(expr) {
                    computed_keys.push(self.compile_computed_key(name, expr)?);
                } else {
                    self.compile_expr(expr)?;
                    self.emit(Op::ToPropertyKey);
                    computed_keys.push(ClassComputedKeyDef::Precomputed);
                }
            }
        }

        let mut constructor = None;
        let mut elements = Vec::new();
        let mut private_elements = Vec::new();

        for element in &body.elements {
            match element {
                ClassElement::Method(member) => {
                    let Expr::Function {
                        params,
                        body,
                        is_generator,
                        is_async,
                        ..
                    } = &member.value
                    else {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "class member is not a method".to_owned(),
                        });
                    };
                    let is_generator = *is_generator;
                    let is_async = *is_async;
                    // Class bodies are strict mode code, so every method and the
                    // constructor compile with strict semantics regardless of
                    // context.
                    let local_names = collect_function_local_names(None, params, body, true);

                    if member.kind == MethodKind::Constructor {
                        let (bytecode, lexical_captures) = self.compile_class_function_body(
                            name,
                            params,
                            body,
                            &local_names,
                            is_generator,
                            is_async,
                        )?;
                        constructor = Some(ClassConstructorDef {
                            name: name.map(str::to_owned),
                            params: params.clone(),
                            local_names,
                            lexical_captures,
                            bytecode: Rc::new(bytecode),
                        });
                        continue;
                    }
                    let (bytecode, lexical_captures) = self.compile_class_function_body(
                        name,
                        params,
                        body,
                        &local_names,
                        is_generator,
                        is_async,
                    )?;

                    let method_kind = match member.kind {
                        MethodKind::Method => ClassMethodKind::Method,
                        MethodKind::Getter => ClassMethodKind::Getter,
                        MethodKind::Setter => ClassMethodKind::Setter,
                        MethodKind::Constructor => unreachable!("handled above"),
                    };
                    let source_text = self.class_method_source_text(member.span, member.is_static);

                    // Private methods and accessors are not ordinary properties:
                    // route them to the private-element list keyed by name.
                    if let ClassMemberKey::Private(private_name) = &member.key {
                        let def = ClassMethodDef {
                            key: ClassMemberKeyDef::Computed,
                            method_kind,
                            is_static: member.is_static,
                            name: Some(format!("#{private_name}")),
                            params: params.clone(),
                            local_names,
                            lexical_captures,
                            bytecode: Rc::new(bytecode),
                            source_text,
                            is_generator,
                            is_async,
                        };
                        let private_element = match member.kind {
                            MethodKind::Getter => ClassPrivateElementDef::Getter {
                                name: private_name.clone(),
                                is_static: member.is_static,
                                def,
                            },
                            MethodKind::Setter => ClassPrivateElementDef::Setter {
                                name: private_name.clone(),
                                is_static: member.is_static,
                                def,
                            },
                            _ => ClassPrivateElementDef::Method {
                                name: private_name.clone(),
                                is_static: member.is_static,
                                def,
                            },
                        };
                        elements.push(ClassElementDef::Private(private_element.clone()));
                        private_elements.push(private_element);
                        continue;
                    }

                    let (key, method_name) = compile_member_key(&member.key);
                    elements.push(ClassElementDef::Method(ClassMethodDef {
                        key,
                        method_kind,
                        is_static: member.is_static,
                        name: method_name,
                        params: params.clone(),
                        local_names,
                        lexical_captures,
                        bytecode: Rc::new(bytecode),
                        source_text,
                        is_generator,
                        is_async,
                    }));
                }
                ClassElement::Field(field) => {
                    let initializer = self.compile_field_initializer(
                        name,
                        field.initializer.as_ref(),
                        &field.key,
                    )?;
                    if let ClassMemberKey::Private(private_name) = &field.key {
                        let private_element = ClassPrivateElementDef::Field {
                            name: private_name.clone(),
                            is_static: field.is_static,
                            initializer,
                        };
                        elements.push(ClassElementDef::Private(private_element.clone()));
                        private_elements.push(private_element);
                        continue;
                    }
                    let (key, _) = compile_member_key(&field.key);
                    elements.push(ClassElementDef::Field(ClassFieldDef {
                        key,
                        is_static: field.is_static,
                        initializer,
                    }));
                }
                ClassElement::StaticBlock(block) => {
                    elements.push(ClassElementDef::StaticBlock(
                        self.compile_static_block(name, block)?,
                    ));
                }
            }
        }

        let constructor = constructor.unwrap_or_else(|| default_constructor(name, has_heritage));

        self.emit(Op::NewClass {
            definition: Rc::new(ClassDefinition {
                name: name.map(str::to_owned),
                constructor,
                elements,
                private_elements,
                computed_keys,
                has_heritage,
            }),
        });
        Ok(())
    }

    fn class_method_source_text(&self, span: Span, is_static: bool) -> Option<Rc<str>> {
        let source = self.source.get(span.start..span.end)?;
        let mut offset = skip_js_trivia(source, 0);
        if is_static && source[offset..].starts_with("static") {
            offset = skip_js_trivia(source, offset + "static".len());
        }
        self.source
            .get((span.start + offset)..span.end)
            .map(|source| self.canonical_function_source(source))
    }
}

fn skip_js_trivia(source: &str, mut index: usize) -> usize {
    let bytes = source.as_bytes();
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            _ => break,
        }
    }
    index
}

impl Compiler {
    fn compile_computed_key(
        &self,
        class_name: Option<&str>,
        expr: &Expr,
    ) -> Result<ClassComputedKeyDef, RuntimeError> {
        let params = FunctionParams::positional(Vec::new());
        let body = vec![Stmt::Return {
            argument: Some(expr.clone()),
            span: expr.span(),
        }];
        let local_names = collect_function_local_names(None, &params, &body, true);
        let mut bytecode = compile_class_function_body_with_captures(
            class_name,
            &params,
            &body,
            &local_names,
            false,
            false,
            &[],
        )?;
        let mut lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
        lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        if !lexical_captures.is_empty() {
            let captured = lexical_captures
                .iter()
                .map(|capture| {
                    (
                        capture.name.as_str(),
                        capture.storage_name.as_str(),
                        self.locals[capture.slot].mutable,
                    )
                })
                .collect::<Vec<_>>();
            bytecode = compile_class_function_body_with_captures(
                class_name,
                &params,
                &body,
                &local_names,
                false,
                false,
                &captured,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
            lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        }
        Ok(ClassComputedKeyDef::Deferred {
            local_names,
            lexical_captures: runtime_lexical_captures(lexical_captures),
            bytecode: Rc::new(bytecode),
        })
    }
}

fn compile_member_key(key: &ClassMemberKey) -> (ClassMemberKeyDef, Option<String>) {
    match key {
        ClassMemberKey::Literal(key) => {
            (ClassMemberKeyDef::Literal(key.clone()), Some(key.clone()))
        }
        ClassMemberKey::Computed(_) => (ClassMemberKeyDef::Computed, None),
        // Private keys are routed to the private-element path before this is
        // ever reached.
        ClassMemberKey::Private(name) => {
            unreachable!("private key #{name} must not reach the ordinary key path")
        }
    }
}

fn expr_contains_private_name(expr: &Expr) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().any(|element| match element {
            ArrayElement::Elision => false,
            ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                expr_contains_private_name(expr)
            }
        }),
        Expr::Object { properties, .. } => properties.iter().any(|property| {
            matches!(&property.key, ObjectPropertyKey::Computed(key) if expr_contains_private_name(key))
                || expr_contains_private_name(&property.value)
        }),
        Expr::Sequence { expressions, .. } | Expr::Template { expressions, .. } => {
            expressions.iter().any(expr_contains_private_name)
        }
        Expr::Unary { argument, .. }
        | Expr::Await { argument, .. }
        | Expr::Yield {
            argument: Some(argument),
            ..
        } => expr_contains_private_name(argument),
        Expr::Binary { left, right, .. } => {
            expr_contains_private_name(left) || expr_contains_private_name(right)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_private_name(tag) || expressions.iter().any(expr_contains_private_name),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_private_name(test)
                || expr_contains_private_name(consequent)
                || expr_contains_private_name(alternate)
        }
        Expr::Assignment { target, value, .. } => {
            assignment_target_contains_private_name(target) || expr_contains_private_name(value)
        }
        Expr::Update { target, .. } => assignment_target_contains_private_name(target),
        Expr::Call {
            callee, arguments, ..
        }
        | Expr::New {
            callee, arguments, ..
        } => {
            expr_contains_private_name(callee)
                || arguments.iter().any(call_argument_contains_private_name)
        }
        Expr::Function { .. } | Expr::Class { .. } => false,
        Expr::Member {
            object, property, ..
        }
        | Expr::OptionalMember {
            object, property, ..
        } => {
            matches!(property, MemberProperty::Private(_))
                || expr_contains_private_name(object)
                || matches!(property, MemberProperty::Computed(key) if expr_contains_private_name(key))
        }
        Expr::OptionalCall {
            callee, arguments, ..
        } => {
            expr_contains_private_name(callee)
                || arguments.iter().any(|arg| match arg {
                    qjs_ast::CallArgument::Expr(e) | qjs_ast::CallArgument::Spread(e) => {
                        expr_contains_private_name(e)
                    }
                })
        }
        Expr::PrivateIn { .. } => true,
        Expr::Literal(_)
        | Expr::Yield { argument: None, .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::Identifier { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. } => false,
        Expr::ImportCall {
            specifier, options, ..
        } => {
            expr_contains_private_name(specifier)
                || options
                    .as_deref()
                    .is_some_and(expr_contains_private_name)
        }
    }
}

fn call_argument_contains_private_name(argument: &CallArgument) -> bool {
    match argument {
        CallArgument::Expr(expr) | CallArgument::Spread(expr) => expr_contains_private_name(expr),
    }
}

fn assignment_target_contains_private_name(target: &AssignmentTarget) -> bool {
    match target {
        AssignmentTarget::Identifier { .. } => false,
        AssignmentTarget::CallExpression { call, .. } => expr_contains_private_name(call),
        AssignmentTarget::Member {
            object, property, ..
        } => {
            matches!(property, MemberProperty::Private(_))
                || expr_contains_private_name(object)
                || matches!(property, MemberProperty::Computed(key) if expr_contains_private_name(key))
        }
        AssignmentTarget::ArrayPattern { elements, rest, .. } => {
            elements.iter().flatten().any(|element| {
                assignment_target_contains_private_name(&element.target)
                    || element
                        .default
                        .as_ref()
                        .is_some_and(expr_contains_private_name)
            }) || rest
                .as_deref()
                .is_some_and(assignment_target_contains_private_name)
        }
        AssignmentTarget::ObjectPattern {
            properties, rest, ..
        } => {
            properties.iter().any(|property| {
                matches!(
                    &property.key,
                    AssignmentTargetPropertyKey::Computed(key)
                        if expr_contains_private_name(key)
                ) || assignment_target_contains_private_name(&property.target)
                    || property
                        .default
                        .as_ref()
                        .is_some_and(expr_contains_private_name)
            }) || rest
                .as_deref()
                .is_some_and(assignment_target_contains_private_name)
        }
    }
}

/// Compiles a field initializer as a parameterless strict-mode thunk whose body
/// returns the initializer value. A field without an initializer compiles to
/// no thunk and installs `undefined`. When the field has a statically known
/// name (a literal or private key), an anonymous function/class initializer
/// takes that name via NamedEvaluation; computed-key fields keep the empty
/// name.
impl Compiler {
    fn compile_field_initializer(
        &self,
        class_name: Option<&str>,
        initializer: Option<&Expr>,
        key: &ClassMemberKey,
    ) -> Result<Option<ClassFieldInitializerDef>, RuntimeError> {
        let Some(expr) = initializer else {
            return Ok(None);
        };
        let params = FunctionParams::positional(Vec::new());
        let inferred_name = match key {
            ClassMemberKey::Literal(name) => Some(name.clone()),
            ClassMemberKey::Private(name) => Some(format!("#{name}")),
            ClassMemberKey::Computed(_) => None,
        };
        let body = vec![Stmt::Return {
            argument: Some(expr.clone()),
            span: expr.span(),
        }];
        let local_names = collect_function_local_names(None, &params, &body, true);
        let mut bytecode = compile_class_field_initializer(
            class_name,
            expr,
            inferred_name.as_deref(),
            &local_names,
            &[],
        )?;
        let mut lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
        lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        if !lexical_captures.is_empty() {
            let captured = lexical_captures
                .iter()
                .map(|capture| {
                    (
                        capture.name.as_str(),
                        capture.storage_name.as_str(),
                        self.locals[capture.slot].mutable,
                    )
                })
                .collect::<Vec<_>>();
            bytecode = compile_class_field_initializer(
                class_name,
                expr,
                inferred_name.as_deref(),
                &local_names,
                &captured,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
            lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        }
        Ok(Some(ClassFieldInitializerDef {
            local_names,
            lexical_captures: runtime_lexical_captures(lexical_captures),
            bytecode: Rc::new(bytecode),
        }))
    }

    /// Compiles a `static { ... }` block into a parameterless strict thunk run at
    /// class definition with `this` = the constructor.
    fn compile_static_block(
        &self,
        class_name: Option<&str>,
        block: &qjs_ast::StaticBlock,
    ) -> Result<ClassStaticBlockDef, RuntimeError> {
        let params = FunctionParams::positional(Vec::new());
        let local_names = collect_function_local_names(None, &params, &block.body, true);
        let mut bytecode = compile_class_function_body_with_captures(
            class_name,
            &params,
            &block.body,
            &local_names,
            false,
            false,
            &[],
        )?;
        let mut lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
        lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        if !lexical_captures.is_empty() {
            let captured = lexical_captures
                .iter()
                .map(|capture| {
                    (
                        capture.name.as_str(),
                        capture.storage_name.as_str(),
                        self.locals[capture.slot].mutable,
                    )
                })
                .collect::<Vec<_>>();
            bytecode = compile_class_function_body_with_captures(
                class_name,
                &params,
                &block.body,
                &local_names,
                false,
                false,
                &captured,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, &local_names);
            lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        }
        Ok(ClassStaticBlockDef {
            local_names,
            lexical_captures: runtime_lexical_captures(lexical_captures),
            bytecode: Rc::new(bytecode),
        })
    }
}

fn runtime_lexical_captures(captures: Vec<LexicalCapture>) -> Vec<(String, usize)> {
    captures
        .into_iter()
        .map(|capture| (capture.storage_name, capture.slot))
        .collect()
}

impl Compiler {
    fn compile_class_function_body(
        &self,
        class_name: Option<&str>,
        params: &FunctionParams,
        body: &[Stmt],
        local_names: &[String],
        is_generator: bool,
        is_async: bool,
    ) -> Result<(super::ir::Bytecode, Vec<(String, usize)>), RuntimeError> {
        let mut bytecode = compile_class_function_body_with_captures(
            class_name,
            params,
            body,
            local_names,
            is_generator,
            is_async,
            &[],
        )?;
        let mut lexical_captures = self.active_lexical_captures(&bytecode, local_names);
        lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        if !lexical_captures.is_empty() {
            let captured_lexicals = lexical_captures
                .iter()
                .map(|capture| {
                    (
                        capture.name.as_str(),
                        capture.storage_name.as_str(),
                        self.locals[capture.slot].mutable,
                    )
                })
                .collect::<Vec<_>>();
            bytecode = compile_class_function_body_with_captures(
                class_name,
                params,
                body,
                local_names,
                is_generator,
                is_async,
                &captured_lexicals,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, local_names);
            lexical_captures.retain(|capture| Some(capture.name.as_str()) != class_name);
        }
        Ok((bytecode, runtime_lexical_captures(lexical_captures)))
    }
}

fn compile_class_function_body_with_captures(
    class_name: Option<&str>,
    params: &FunctionParams,
    body: &[Stmt],
    local_names: &[String],
    is_generator: bool,
    is_async: bool,
    captured_lexicals: &[(&str, &str, bool)],
) -> Result<super::ir::Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
    compiler.async_generator_body = is_generator && is_async;
    for (name, storage_name, mutable) in captured_lexicals {
        compiler.declare_captured_lexical_slot_with_storage_name(name, storage_name, *mutable);
    }
    if let Some(name) = class_name
        && local_names
            .binary_search_by(|local| local.as_str().cmp(name))
            .is_err()
    {
        compiler.declare_captured_lexical_slot(name, false);
    }
    compiler.compile_function(params, body)
}

fn compile_class_field_initializer(
    class_name: Option<&str>,
    init: &Expr,
    inferred_name: Option<&str>,
    local_names: &[String],
    captured_lexicals: &[(&str, &str, bool)],
) -> Result<super::ir::Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
    for (name, storage_name, mutable) in captured_lexicals {
        compiler.declare_captured_lexical_slot_with_storage_name(name, storage_name, *mutable);
    }
    if let Some(name) = class_name
        && local_names
            .binary_search_by(|local| local.as_str().cmp(name))
            .is_err()
    {
        compiler.declare_captured_lexical_slot(name, false);
    }
    match inferred_name {
        Some(name) => compiler.compile_named_expr(init, name)?,
        None => compiler.compile_expr(init)?,
    }
    compiler.emit(Op::Return);
    Ok(super::ir::Bytecode::new(
        compiler.constants,
        compiler.locals,
        compiler.code,
    ))
}

/// Builds the implicit default constructor. A base class gets an empty body;
/// a derived class gets `constructor(...args) { super(...args); }`, forwarding
/// its arguments to the parent constructor.
fn default_constructor(name: Option<&str>, has_heritage: bool) -> ClassConstructorDef {
    if !has_heritage {
        let params = FunctionParams::positional(Vec::new());
        let bytecode =
            compile_function_body_with_strict(&params, &[], true).expect("empty body compiles");
        let local_names = collect_function_local_names(None, &params, &[], true);
        return ClassConstructorDef {
            name: name.map(str::to_owned),
            params,
            local_names,
            lexical_captures: Vec::new(),
            bytecode: Rc::new(bytecode),
        };
    }

    // Derived default constructor: `constructor(...args) { super(...args); }`,
    // but the spec forwards the rest parameter list directly. It must not
    // evaluate spread syntax or consult `Array.prototype[Symbol.iterator]`.
    let zero = Span::new(0, 0);
    let args_binding = "\u{0}\u{0}class_default_constructor_args".to_owned();
    let params = FunctionParams::new(
        Vec::new(),
        Some(BindingPattern::Identifier {
            name: args_binding.clone(),
            span: zero,
        }),
    );
    let bytecode = compile_default_derived_constructor_bytecode(&params, &args_binding)
        .expect("derived ctor compiles");
    let local_names = collect_function_local_names(None, &params, &[], true);
    ClassConstructorDef {
        name: name.map(str::to_owned),
        params,
        local_names,
        lexical_captures: Vec::new(),
        bytecode: Rc::new(bytecode),
    }
}

fn compile_default_derived_constructor_bytecode(
    params: &FunctionParams,
    args_binding: &str,
) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
    for (index, element) in params.positional.iter().enumerate() {
        let binding_name = crate::function::parameter_binding_name(&element.binding, index);
        compiler.parameter_slot(&binding_name);
    }
    if let Some(rest) = &params.rest {
        let binding_name = rest_parameter_binding_name(rest);
        compiler.parameter_slot(&binding_name);
    }
    compiler.compile_parameter_bindings(params, false)?;
    compiler.emit(Op::FunctionPrologueEnd);
    let args_slot = compiler
        .resolve_local_slot(args_binding)
        .expect("default constructor rest parameter should have a slot");
    compiler.emit(Op::LoadLocal(args_slot));
    compiler.emit(Op::SuperCallSpread);
    compiler.emit(Op::Pop);
    compiler.emit_load_undefined();
    compiler.emit(Op::Return);
    Ok(Bytecode::new(
        compiler.constants,
        compiler.locals,
        compiler.code,
    ))
}
