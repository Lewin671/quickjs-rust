use std::rc::Rc;

use qjs_ast::{
    BindingPattern, CallArgument, ClassBody, ClassElement, ClassMemberKey, Expr, FunctionParams,
    MethodKind, Span, Stmt,
};

use crate::{RuntimeError, function::collect_function_local_names};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::ir::{
    ClassConstructorDef, ClassElementDef, ClassFieldDef, ClassFieldInitializerDef,
    ClassMemberKeyDef, ClassMethodDef, ClassMethodKind, ClassPrivateElementDef,
    ClassStaticBlockDef, Op,
};

impl Compiler {
    /// Compiles a class declaration or expression into a `NewClass` op that
    /// builds the constructor function object at runtime. The class name (when
    /// present) is used for the constructor `name` property and the bindable
    /// inner name.
    ///
    /// Computed member keys are evaluated, in class-definition order, before
    /// the `NewClass` op and left on the stack; the op consumes them.
    pub(super) fn compile_class(
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

        // Evaluate computed keys (methods and fields) in source order so their
        // side effects run in class-definition order, ahead of building the
        // constructor.
        let mut computed_key_count = 0usize;
        for element in &body.elements {
            let key = match element {
                ClassElement::Method(member) => &member.key,
                ClassElement::Field(field) => &field.key,
                // Static blocks have no key.
                ClassElement::StaticBlock(_) => continue,
            };
            if let ClassMemberKey::Computed(expr) = key {
                self.compile_expr(expr)?;
                computed_key_count += 1;
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
                    let bytecode = compile_function_body_with_strict(params, body, true)?;
                    let local_names = collect_function_local_names(None, params, body, true);

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
                        private_elements.push(match member.kind {
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
                        });
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
                        compile_field_initializer(field.initializer.as_ref(), &field.key)?;
                    if let ClassMemberKey::Private(private_name) = &field.key {
                        private_elements.push(ClassPrivateElementDef::Field {
                            name: private_name.clone(),
                            is_static: field.is_static,
                            initializer,
                        });
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
                    elements.push(ClassElementDef::StaticBlock(compile_static_block(block)?));
                }
            }
        }

        let constructor = constructor.unwrap_or_else(|| default_constructor(name, has_heritage));

        self.emit(Op::NewClass {
            name: name.map(str::to_owned),
            constructor,
            elements,
            private_elements,
            computed_key_count,
            has_heritage,
        });
        Ok(())
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

/// Compiles a field initializer as a parameterless strict-mode thunk whose body
/// returns the initializer value. A field without an initializer compiles to
/// no thunk and installs `undefined`. When the field has a statically known
/// name (a literal or private key), an anonymous function/class initializer
/// takes that name via NamedEvaluation; computed-key fields keep the empty
/// name.
fn compile_field_initializer(
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
    let bytecode = match inferred_name {
        Some(name) => super::compiler::compile_named_field_initializer(expr, &name)?,
        None => compile_function_body_with_strict(&params, &body, true)?,
    };
    let local_names = collect_function_local_names(None, &params, &body, true);
    Ok(Some(ClassFieldInitializerDef {
        local_names,
        bytecode: Rc::new(bytecode),
    }))
}

/// Compiles a `static { ... }` block into a parameterless strict thunk run at
/// class definition with `this` = the constructor.
fn compile_static_block(block: &qjs_ast::StaticBlock) -> Result<ClassStaticBlockDef, RuntimeError> {
    let params = FunctionParams::positional(Vec::new());
    let bytecode = compile_function_body_with_strict(&params, &block.body, true)?;
    let local_names = collect_function_local_names(None, &params, &block.body, true);
    Ok(ClassStaticBlockDef {
        local_names,
        bytecode: Rc::new(bytecode),
    })
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
    let zero = Span::new(0, 0);
    let params = FunctionParams::new(
        Vec::new(),
        Some(BindingPattern::Identifier {
            name: "args".to_owned(),
            span: zero,
        }),
    );
    let body = vec![Stmt::Expr(Expr::Call {
        callee: Box::new(Expr::Super { span: zero }),
        arguments: vec![CallArgument::Spread(Expr::Identifier {
            name: "args".to_owned(),
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
