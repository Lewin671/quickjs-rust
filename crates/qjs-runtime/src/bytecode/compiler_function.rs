use qjs_ast::{FunctionParams, Stmt};

use crate::RuntimeError;

use super::compiler::{Compiler, compile_function_body_with_strict_generator};
use super::compiler_lexical::LexicalCapture;
use super::ir::Bytecode;

fn compile_with_captured_lexicals(
    params: &FunctionParams,
    body: &[Stmt],
    is_strict: bool,
    is_generator: bool,
    is_async: bool,
    with_base_depth: usize,
    captured_lexicals: &[(&str, &str, bool)],
) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler::function_compiler_with_base_with_depth(
        is_strict,
        is_generator && is_async,
        with_base_depth,
    );
    for (name, storage_name, mutable) in captured_lexicals {
        compiler.declare_captured_lexical_slot_with_storage_name(name, storage_name, *mutable);
    }
    compiler.compile_function(params, body)
}

fn runtime_lexical_captures(captures: Vec<LexicalCapture>) -> Vec<(String, usize)> {
    captures
        .into_iter()
        .map(|capture| (capture.storage_name, capture.slot))
        .collect()
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
        let mut bytecode = if self.inside_with() {
            let compiler = Compiler::function_compiler_with_base_with_depth(
                is_strict,
                is_generator && is_async,
                self.with_depth,
            );
            compiler.compile_function(params, body)?
        } else {
            compile_function_body_with_strict_generator(
                params,
                body,
                is_strict,
                is_generator,
                is_async,
            )?
        };
        let mut lexical_captures = self.active_lexical_captures(&bytecode, local_names);
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
            bytecode = compile_with_captured_lexicals(
                params,
                body,
                is_strict,
                is_generator,
                is_async,
                self.with_depth,
                &captured_lexicals,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, local_names);
        }
        Ok((bytecode, runtime_lexical_captures(lexical_captures)))
    }
}
