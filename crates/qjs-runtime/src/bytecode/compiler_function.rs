use qjs_ast::{FunctionParams, Stmt};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::compiler_lexical::LexicalCapture;
use super::ir::Bytecode;

#[allow(clippy::too_many_arguments)]
fn compile_with_captured_lexicals(
    params: &FunctionParams,
    body: &[Stmt],
    is_strict: bool,
    is_generator: bool,
    is_async: bool,
    with_base_depth: usize,
    captured_lexicals: &[(&str, &str, bool)],
    source: &std::rc::Rc<str>,
) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler::function_compiler_with_base_with_depth(
        is_strict,
        is_generator && is_async,
        with_base_depth,
    );
    compiler.source = source.clone();
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
    /// Slices the retained source text for a function whose source span is
    /// `span`, for `Function.prototype.toString`. Returns `None` when no source
    /// is available (synthesized bodies) or the span is out of range, falling
    /// back to the `[native code]` form.
    pub(super) fn function_source_text(&self, span: qjs_ast::Span) -> Option<std::rc::Rc<str>> {
        self.source.get(span.start..span.end).map(std::rc::Rc::from)
    }

    pub(super) fn compile_nested_function_body(
        &self,
        params: &FunctionParams,
        body: &[Stmt],
        is_strict: bool,
        is_generator: bool,
        is_async: bool,
        local_names: &[String],
    ) -> Result<(Bytecode, Vec<(String, usize)>), RuntimeError> {
        let mut bytecode = {
            // Inner functions are compiled by a fresh compiler; carry the source
            // text so their own nested functions can still slice it for
            // `Function.prototype.toString`.
            let mut compiler = if self.inside_with() {
                Compiler::function_compiler_with_base_with_depth(
                    is_strict,
                    is_generator && is_async,
                    self.with_depth,
                )
            } else {
                Compiler::function_compiler(is_strict, is_generator && is_async)
            };
            compiler.source = self.source.clone();
            compiler.compile_function(params, body)?
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
                &self.source,
            )?;
            lexical_captures = self.active_lexical_captures(&bytecode, local_names);
        }
        Ok((bytecode, runtime_lexical_captures(lexical_captures)))
    }
}
