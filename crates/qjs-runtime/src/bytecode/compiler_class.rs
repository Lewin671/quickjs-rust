use std::rc::Rc;

use qjs_ast::{
    BindingPattern, CallArgument, ClassBody, ClassMemberKey, Expr, FunctionParams, MethodKind,
    Span, Stmt,
};

use crate::{RuntimeError, function::collect_function_local_names};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::ir::{ClassConstructorDef, ClassMemberKeyDef, ClassMethodDef, ClassMethodKind, Op};

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

        // Evaluate computed keys in source order so their side effects run in
        // class-definition order, ahead of building the constructor.
        let mut computed_key_count = 0usize;
        for member in &body.members {
            if let ClassMemberKey::Computed(expr) = &member.key {
                self.compile_expr(expr)?;
                computed_key_count += 1;
            }
        }

        let mut constructor = None;
        let mut methods = Vec::new();

        for member in &body.members {
            let Expr::Function { params, body, .. } = &member.value else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "class member is not a method".to_owned(),
                });
            };
            // Class bodies are strict mode code, so every method and the
            // constructor compile with strict semantics regardless of context.
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
            let (key, name) = match &member.key {
                ClassMemberKey::Literal(key) => {
                    (ClassMemberKeyDef::Literal(key.clone()), Some(key.clone()))
                }
                ClassMemberKey::Computed(_) => (ClassMemberKeyDef::Computed, None),
            };
            methods.push(ClassMethodDef {
                key,
                method_kind,
                is_static: member.is_static,
                name,
                params: params.clone(),
                local_names,
                bytecode: Rc::new(bytecode),
            });
        }

        let constructor = constructor.unwrap_or_else(|| default_constructor(name, has_heritage));

        self.emit(Op::NewClass {
            name: name.map(str::to_owned),
            constructor,
            methods,
            computed_key_count,
            has_heritage,
        });
        Ok(())
    }
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
