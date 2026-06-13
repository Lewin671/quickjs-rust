use std::rc::Rc;

use qjs_ast::{
    BindingPattern, CallArgument, ClassBody, ClassElement, ClassMemberKey, Expr, FunctionParams,
    MethodKind, Span, Stmt,
};

use crate::{RuntimeError, function::collect_function_local_names};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::ir::{
    ClassComputedKeyDef, ClassConstructorDef, ClassElementDef, ClassFieldDef,
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

        self.with_lexical_scope(|compiler| {
            let slot = compiler.declare_lexical_slot(name, false);
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
                computed_keys.push(compile_computed_key(name, expr)?);
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
                    let bytecode = compile_class_function_body(
                        name,
                        params,
                        body,
                        &local_names,
                        is_generator,
                        is_async,
                    )?;

                    if member.kind == MethodKind::Constructor {
                        constructor = Some(ClassConstructorDef {
                            name: name.map(str::to_owned),
                            params: params.clone(),
                            local_names,
                            bytecode: Rc::new(bytecode),
                        });
                        continue;
                    }

                    let method_kind = match member.kind {
                        MethodKind::Method => ClassMethodKind::Method,
                        MethodKind::Getter => ClassMethodKind::Getter,
                        MethodKind::Setter => ClassMethodKind::Setter,
                        MethodKind::Constructor => unreachable!("handled above"),
                    };

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
                            bytecode: Rc::new(bytecode),
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
                        bytecode: Rc::new(bytecode),
                        is_generator,
                        is_async,
                    }));
                }
                ClassElement::Field(field) => {
                    let initializer =
                        compile_field_initializer(name, field.initializer.as_ref(), &field.key)?;
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
                    elements.push(ClassElementDef::StaticBlock(compile_static_block(
                        name, block,
                    )?));
                }
            }
        }

        let constructor = constructor.unwrap_or_else(|| default_constructor(name, has_heritage));

        self.emit(Op::NewClass {
            name: name.map(str::to_owned),
            constructor,
            elements,
            private_elements,
            computed_keys,
            has_heritage,
        });
        Ok(())
    }
}

fn compile_computed_key(
    class_name: Option<&str>,
    expr: &Expr,
) -> Result<ClassComputedKeyDef, RuntimeError> {
    let params = FunctionParams::positional(Vec::new());
    let body = vec![Stmt::Return {
        argument: Some(expr.clone()),
        span: expr.span(),
    }];
    let local_names = collect_function_local_names(None, &params, &body, true);
    let bytecode =
        compile_class_function_body(class_name, &params, &body, &local_names, false, false)?;
    Ok(ClassComputedKeyDef {
        local_names,
        bytecode: Rc::new(bytecode),
    })
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
fn compile_field_initializer(
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
    let bytecode =
        compile_class_field_initializer(class_name, expr, inferred_name.as_deref(), &local_names)?;
    Ok(Some(ClassFieldInitializerDef {
        local_names,
        bytecode: Rc::new(bytecode),
    }))
}

/// Compiles a `static { ... }` block into a parameterless strict thunk run at
/// class definition with `this` = the constructor.
fn compile_static_block(
    class_name: Option<&str>,
    block: &qjs_ast::StaticBlock,
) -> Result<ClassStaticBlockDef, RuntimeError> {
    let params = FunctionParams::positional(Vec::new());
    let local_names = collect_function_local_names(None, &params, &block.body, true);
    let bytecode =
        compile_class_function_body(class_name, &params, &block.body, &local_names, false, false)?;
    Ok(ClassStaticBlockDef {
        local_names,
        bytecode: Rc::new(bytecode),
    })
}

fn compile_class_function_body(
    class_name: Option<&str>,
    params: &FunctionParams,
    body: &[Stmt],
    local_names: &[String],
    is_generator: bool,
    is_async: bool,
) -> Result<super::ir::Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
    compiler.async_generator_body = is_generator && is_async;
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
) -> Result<super::ir::Bytecode, RuntimeError> {
    let mut compiler = Compiler::strict_function_compiler();
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
            bytecode: Rc::new(bytecode),
        };
    }

    // Derived default constructor: `constructor(...args) { super(...args); }`.
    // Use an internal rest binding so the synthetic name cannot shadow a user
    // binding visible to the parent constructor.
    let zero = Span::new(0, 0);
    let args_binding = "\u{0}\u{0}class_default_constructor_args".to_owned();
    let params = FunctionParams::new(
        Vec::new(),
        Some(BindingPattern::Identifier {
            name: args_binding.clone(),
            span: zero,
        }),
    );
    let body = vec![Stmt::Expr(Expr::Call {
        callee: Box::new(Expr::Super { span: zero }),
        arguments: vec![CallArgument::Spread(Expr::Identifier {
            name: args_binding,
            span: zero,
        })],
        span: zero,
    })];
    let bytecode =
        compile_function_body_with_strict(&params, &body, true).expect("derived ctor compiles");
    let local_names = collect_function_local_names(None, &params, &body, true);
    ClassConstructorDef {
        name: name.map(str::to_owned),
        params,
        local_names,
        bytecode: Rc::new(bytecode),
    }
}
