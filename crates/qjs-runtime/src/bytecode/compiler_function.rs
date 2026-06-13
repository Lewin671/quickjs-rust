use qjs_ast::{FunctionParams, Stmt};

use crate::RuntimeError;

use super::compiler::{Compiler, compile_function_body_with_strict_generator};
use super::ir::Bytecode;

fn compile_with_captured_lexicals(
    params: &FunctionParams,
    body: &[Stmt],
    is_strict: bool,
    is_generator: bool,
    is_async: bool,
    captured_lexicals: &[(String, bool)],
) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler::function_compiler(is_strict, is_generator && is_async);
    for (name, mutable) in captured_lexicals {
        compiler.declare_captured_lexical_slot(name, *mutable);
    }
    compiler.compile_function(params, body)
}

impl Compiler {
    pub(super) fn compile_nested_function_body(
        &self,
        params: &FunctionParams,
        body: &[Stmt],
        is_strict: bool,
        is_generator: bool,
        is_async: bool,
        local_names: &[String],
    ) -> Result<(Bytecode, Vec<(String, usize)>), RuntimeError> {
        let mut bytecode = compile_function_body_with_strict_generator(
            params,
            body,
            is_strict,
            is_generator,
            is_async,
        )?;
        let mut lexical_captures = self.active_lexical_captures(&bytecode, local_names);
        if !lexical_captures.is_empty() {
            let captured_lexicals = lexical_captures
                .iter()
                .map(|(name, slot)| (name.clone(), self.locals[*slot].mutable))
                .collect::<Vec<_>>();
            bytecode = compile_with_captured_lexicals(
                params,
                body,
                is_strict,
                is_generator,
                is_async,
                &captured_lexicals,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, local_names);
        }
        Ok((bytecode, lexical_captures))
    }
}
