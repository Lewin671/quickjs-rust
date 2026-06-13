//! Function parameter prologue compilation.

use qjs_ast::{BinaryOp, BindingPattern, FunctionParams, VarKind};

use crate::{
    RuntimeError,
    function::{
        parameter_argument_binding_name, parameter_binding_name,
        rest_parameter_argument_binding_name, rest_parameter_binding_name,
    },
};

use super::{compiler::Compiler, ir::Op};

impl Compiler {
    /// Snapshots identifier parameters before clearing their real slots for a
    /// non-simple parameter list. Default initializers then observe the
    /// parameter environment's TDZ while still retaining the original argument
    /// values for ordered binding initialization.
    pub(super) fn snapshot_non_simple_parameter_arguments(
        &mut self,
        params: &FunctionParams,
    ) -> Result<(), RuntimeError> {
        for (index, element) in params.positional.iter().enumerate() {
            let BindingPattern::Identifier { name, .. } = &element.binding else {
                continue;
            };
            let slot = self
                .resolve_local_slot(name)
                .expect("identifier parameter slot should be declared before snapshots");
            let raw_slot = self.local_slot(&parameter_argument_binding_name(index), true);
            self.emit(Op::LoadLocalOrUndefined(slot));
            self.emit(Op::StoreLocal(raw_slot));
            self.emit(Op::ClearLocal(slot));
        }
        if let Some(rest) = &params.rest
            && let BindingPattern::Identifier { name, .. } = rest.as_ref()
        {
            let slot = self
                .resolve_local_slot(name)
                .expect("rest parameter slot should be declared before snapshots");
            let raw_slot = self.local_slot(&rest_parameter_argument_binding_name(), true);
            self.emit(Op::LoadLocalOrUndefined(slot));
            self.emit(Op::StoreLocal(raw_slot));
            self.emit(Op::ClearLocal(slot));
        }
        Ok(())
    }

    pub(super) fn compile_parameter_bindings(
        &mut self,
        params: &FunctionParams,
        non_simple_params: bool,
    ) -> Result<(), RuntimeError> {
        for (index, element) in params.positional.iter().enumerate() {
            if let BindingPattern::Identifier { name, .. } = &element.binding {
                let slot = self
                    .resolve_local_slot(name)
                    .expect("parameter slot should be declared before defaults");
                if non_simple_params {
                    let value_slot = self
                        .resolve_local_slot(&parameter_argument_binding_name(index))
                        .expect("parameter argument snapshot should be declared");
                    match &element.default {
                        Some(default) => {
                            self.emit(Op::LoadLocal(value_slot));
                            self.emit_load_undefined();
                            self.emit(Op::Binary(BinaryOp::StrictEq));
                            let skip_default = self.emit(Op::JumpIfFalse(usize::MAX));
                            self.emit(Op::Pop);
                            self.compile_named_expr(default, name)?;
                            self.emit(Op::StoreLocal(slot));
                            let done = self.emit(Op::Jump(usize::MAX));
                            let skip_target = self.code.len();
                            self.patch_jump(skip_default, skip_target);
                            self.emit(Op::Pop);
                            self.emit(Op::LoadLocal(value_slot));
                            self.emit(Op::StoreLocal(slot));
                            let done_target = self.code.len();
                            self.patch_jump(done, done_target);
                        }
                        None => {
                            self.emit(Op::LoadLocal(value_slot));
                            self.emit(Op::StoreLocal(slot));
                        }
                    }
                } else if let Some(default) = &element.default {
                    self.emit(Op::LoadLocal(slot));
                    self.emit_load_undefined();
                    self.emit(Op::Binary(BinaryOp::StrictEq));
                    let skip_default = self.emit(Op::JumpIfFalse(usize::MAX));
                    self.emit(Op::Pop);
                    self.compile_named_expr(default, name)?;
                    self.emit(Op::StoreLocal(slot));
                    let done = self.emit(Op::Jump(usize::MAX));
                    let skip_target = self.code.len();
                    self.patch_jump(skip_default, skip_target);
                    self.emit(Op::Pop);
                    let done_target = self.code.len();
                    self.patch_jump(done, done_target);
                }
            } else {
                let binding_name = parameter_binding_name(&element.binding, index);
                let slot = self
                    .resolve_local_slot(&binding_name)
                    .expect("parameter pattern slot should be declared before bindings");
                self.emit(Op::LoadLocal(slot));
                self.compile_binding_default(
                    element.default.as_ref(),
                    super::compiler_binding::binding_inferred_name(&element.binding),
                )?;
                self.compile_binding_initializer(&element.binding, VarKind::Var)?;
            }
        }
        if let Some(rest) = &params.rest
            && non_simple_params
            && let BindingPattern::Identifier { name, .. } = rest.as_ref()
        {
            let slot = self
                .resolve_local_slot(name)
                .expect("rest parameter slot should be declared before bindings");
            let raw_slot = self
                .resolve_local_slot(&rest_parameter_argument_binding_name())
                .expect("rest parameter argument snapshot should be declared");
            self.emit(Op::LoadLocal(raw_slot));
            self.emit(Op::StoreLocal(slot));
        } else if let Some(rest) = &params.rest
            && !matches!(rest.as_ref(), BindingPattern::Identifier { .. })
        {
            let binding_name = rest_parameter_binding_name(rest);
            let slot = self
                .resolve_local_slot(&binding_name)
                .expect("rest parameter pattern slot should be declared before bindings");
            self.emit(Op::LoadLocal(slot));
            self.compile_binding_initializer(rest, VarKind::Var)?;
        }
        Ok(())
    }
}
