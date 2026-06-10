use std::rc::Rc;

use qjs_ast::{ClassBody, ClassMemberKey, Expr, FunctionParams, MethodKind};

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

        let constructor = constructor.unwrap_or_else(|| default_constructor(name));

        self.emit(Op::NewClass {
            name: name.map(str::to_owned),
            constructor,
            methods,
            computed_key_count,
        });
        Ok(())
    }
}

/// Builds the implicit empty default constructor for a base class.
fn default_constructor(name: Option<&str>) -> ClassConstructorDef {
    let params = FunctionParams::positional(Vec::new());
    let bytecode =
        compile_function_body_with_strict(&params, &[], true).expect("empty body compiles");
    let local_names = collect_function_local_names(None, &params, &[], true);
    ClassConstructorDef {
        name: name.map(str::to_owned),
        params,
        local_names,
        bytecode: Rc::new(bytecode),
    }
}
