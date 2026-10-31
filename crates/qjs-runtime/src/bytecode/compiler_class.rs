//! Class lowering: `class` declarations and expressions become one
//! a class-definition region. Computed keys execute in the enclosing IR;
//! only methods, field initializers and static blocks have nested bodies.

use std::rc::Rc;

use qjs_ast::{
    BindingPattern, ClassBody, ClassElement, ClassMemberKey, Expr, FunctionParams, MethodKind,
    Span, Stmt,
};

use crate::{
    RuntimeError,
    function::{collect_function_local_names, rest_parameter_binding_name},
};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::compiler_lexical::LexicalCapture;
use super::ir::{
    Bytecode, ClassConstructorDef, ClassDefinition, ClassElementDef, ClassFieldDef,
    ClassFieldInitializerDef, ClassMemberKeyDef, ClassMethodDef, ClassMethodKind,
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

        self.with_scope_kind(crate::binding::ScopeKind::ClassName, |compiler| {
            let storage_name = format!("\0class_expr_inner:{}:{}", name, compiler.locals.len());
            let slot = compiler.declare_lexical_slot_with_storage_name(name, &storage_name, false);
            compiler.emit(Op::ClearLocal(slot));
            compiler.compile_bound_class(name, slot, body)?;
            Ok(())
        })
    }

    /// Anonymous classes may have an inferred display name without introducing
    /// a lexical binding for that name.
    pub(super) fn compile_class(
        &mut self,
        name: Option<&str>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        self.compile_class_region(name, None, body)
    }

    pub(super) fn compile_bound_class(
        &mut self,
        name: &str,
        slot: usize,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        self.compile_class_region(Some(name), Some(slot), body)
    }

    fn compile_class_region(
        &mut self,
        display_name: Option<&str>,
        name_slot: Option<usize>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        let previous_strict = self.strict;
        self.strict = true;
        let result = self.compile_class_strict(display_name, name_slot, body);
        self.strict = previous_strict;
        result
    }

    fn compile_class_strict(
        &mut self,
        display_name: Option<&str>,
        name_slot: Option<usize>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
        let name = name_slot.and(display_name);
        let has_heritage = body.heritage.is_some();
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
                            name: display_name.map(str::to_owned),
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

        let constructor =
            constructor.unwrap_or_else(|| default_constructor(display_name, has_heritage));

        self.emit(Op::NewClass {
            definition: Rc::new(ClassDefinition {
                name: name.map(str::to_owned),
                name_slot,
                constructor,
                elements,
                private_elements,
                has_heritage,
            }),
        });
        if let Some(heritage) = &body.heritage {
            self.compile_expr(heritage)?;
        }
        self.emit(Op::BeginClass);
        let mut index = 0;
        for element in &body.elements {
            let key = match element {
                ClassElement::Method(member) if member.kind == MethodKind::Constructor => continue,
                ClassElement::Method(member) => Some(&member.key),
                ClassElement::Field(field) => Some(&field.key),
                ClassElement::StaticBlock(_) => None,
            };
            if let Some(ClassMemberKey::Computed(expr)) = key {
                self.compile_expr(expr)?;
                self.emit(Op::ToPropertyKey);
            }
            self.emit(Op::DefineClassElement { index });
            index += 1;
        }
        self.emit(Op::FinishClass);
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
            is_generator && is_async,
            true,
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
                is_generator && is_async,
                true,
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
    async_generator: bool,
    arguments_owner: bool,
    captured_lexicals: &[(&str, &str, bool)],
) -> Result<super::ir::Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
    compiler.async_generator_body = async_generator;
    compiler.arguments_owner = arguments_owner;
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
    compiler.arguments_owner = false;
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
    Ok(Bytecode::from_compiler(compiler, None, Vec::new()))
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
    compiler.scope_plan.scopes[0].kind = crate::binding::ScopeKind::Parameters;
    Ok(Bytecode::from_compiler(
        compiler,
        Some((Vec::new(), params)),
        Vec::new(),
    ))
}
