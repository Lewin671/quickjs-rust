use std::rc::Rc;

use qjs_ast::{ClassBody, ClassMemberKey, Expr, FunctionParams, MethodKind};

use crate::{RuntimeError, function::collect_function_local_names};

use super::compiler::{Compiler, compile_function_body_with_strict};
use super::ir::{ClassConstructorDef, ClassMethodDef, Op};

impl Compiler {
    /// Compiles a class declaration or expression into a `NewClass` op that
    /// builds the constructor function object at runtime. The class name (when
    /// present) is used for the constructor `name` property and the bindable
    /// inner name.
    pub(super) fn compile_class(
        &mut self,
        name: Option<&str>,
        body: &ClassBody,
    ) -> Result<(), RuntimeError> {
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

            match member.kind {
                MethodKind::Constructor => {
                    constructor = Some(ClassConstructorDef {
                        name: name.map(str::to_owned),
                        params: params.clone(),
                        local_names,
                        bytecode: Rc::new(bytecode),
                    });
                }
                MethodKind::Method => {
                    let ClassMemberKey::Literal(key) = &member.key;
                    methods.push(ClassMethodDef {
                        key: key.clone(),
                        name: Some(key.clone()),
                        params: params.clone(),
                        local_names,
                        bytecode: Rc::new(bytecode),
                    });
                }
            }
        }

        let constructor = constructor.unwrap_or_else(|| default_constructor(name));

        self.emit(Op::NewClass {
            name: name.map(str::to_owned),
            constructor,
            methods,
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
