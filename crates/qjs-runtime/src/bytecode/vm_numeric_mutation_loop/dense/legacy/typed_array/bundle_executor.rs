//! Executor used only by TypedArray plans with validated Binary bundles.

use qjs_ast::BinaryOp;

use crate::{to_int32_number, to_uint32_number};

#[cfg(test)]
use super::super::super::record_read_only_iteration;
use super::super::super::{
    DynamicProgramRun, MAX_DENSE_LOCALS, array_index_from_number, record_iteration,
    record_typed_binary_bundle_iteration, record_typed_binary_bundle_step,
    record_typed_executor_steps, record_typed_logical_operations,
};
use super::super::LegacyDynamicDensePlan;
use super::{
    TypedDenseAccess,
    program::{TypedBinaryBundle, TypedBinaryStep, TypedInstruction, TypedProgram},
};

impl TypedProgram {
    #[inline(always)]
    fn execute_instruction(
        operation: TypedInstruction,
        access: &mut TypedDenseAccess<'_, '_>,
        locals: &[f64; MAX_DENSE_LOCALS],
        registers: &[f64],
    ) -> Option<f64> {
        Some(match operation {
            TypedInstruction::Constant(value) => value,
            TypedInstruction::LoadLocal(local) => *locals.get(local)?,
            TypedInstruction::LoadU8 { receiver, index } => {
                access.load_u8(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadI8 { receiver, index } => {
                access.load_i8(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadU16 { receiver, index } => {
                access.load_u16(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadI16 { receiver, index } => {
                access.load_i16(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadU32 { receiver, index } => {
                access.load_u32(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadI32 { receiver, index } => {
                access.load_i32(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadF32 { receiver, index } => {
                access.load_f32(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::LoadF64 { receiver, index } => {
                access.load_f64(receiver, array_index_from_number(*registers.get(index)?)?)?
            }
            TypedInstruction::StoreU8 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_u8(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreI8 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_i8(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreU8Clamped {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access
                    .stage_u8_clamped(receiver, index, value)
                    .then_some(value)?
            }
            TypedInstruction::StoreU16 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_u16(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreI16 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_i16(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreU32 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_u32(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreI32 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_i32(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreF32 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_f32(receiver, index, value).then_some(value)?
            }
            TypedInstruction::StoreF64 {
                receiver,
                index,
                value,
            } => {
                let index = array_index_from_number(*registers.get(index)?)?;
                let value = *registers.get(value)?;
                access.stage_f64(receiver, index, value).then_some(value)?
            }
            TypedInstruction::Add { left, right } => {
                *registers.get(left)? + *registers.get(right)?
            }
            TypedInstruction::Sub { left, right } => {
                *registers.get(left)? - *registers.get(right)?
            }
            TypedInstruction::Mul { left, right } => {
                *registers.get(left)? * *registers.get(right)?
            }
            TypedInstruction::Div { left, right } => {
                *registers.get(left)? / *registers.get(right)?
            }
            TypedInstruction::Rem { left, right } => {
                *registers.get(left)? % *registers.get(right)?
            }
            TypedInstruction::Shl { left, right } => f64::from(
                to_int32_number(*registers.get(left)?)
                    << (to_uint32_number(*registers.get(right)?) & 0x1f),
            ),
            TypedInstruction::Shr { left, right } => f64::from(
                to_int32_number(*registers.get(left)?)
                    >> (to_uint32_number(*registers.get(right)?) & 0x1f),
            ),
            TypedInstruction::UShr { left, right } => f64::from(
                to_uint32_number(*registers.get(left)?)
                    >> (to_uint32_number(*registers.get(right)?) & 0x1f),
            ),
            TypedInstruction::BitwiseAnd { left, right } => f64::from(
                to_int32_number(*registers.get(left)?) & to_int32_number(*registers.get(right)?),
            ),
            TypedInstruction::BitwiseXor { left, right } => f64::from(
                to_int32_number(*registers.get(left)?) ^ to_int32_number(*registers.get(right)?),
            ),
            TypedInstruction::BitwiseOr { left, right } => f64::from(
                to_int32_number(*registers.get(left)?) | to_int32_number(*registers.get(right)?),
            ),
            TypedInstruction::Plus { value } => *registers.get(value)?,
            TypedInstruction::Minus { value } => -*registers.get(value)?,
            TypedInstruction::BitwiseNot { value } => {
                f64::from(!to_int32_number(*registers.get(value)?))
            }
            TypedInstruction::Increment { value } => *registers.get(value)? + 1.0,
            TypedInstruction::Decrement { value } => *registers.get(value)? - 1.0,
        })
    }

    #[inline(always)]
    fn execute_bundle_step(
        bundle: &TypedBinaryBundle,
        step_index: usize,
        step: &TypedBinaryStep,
        registers: &mut [f64],
    ) {
        macro_rules! execute_lanes {
            ($left:ident, $right:ident, $value:expr) => {{
                for lane in 0..bundle.lane_count {
                    let operands = step.operands[lane];
                    let $left = registers[operands.left];
                    let $right = registers[operands.right];
                    let value = $value;
                    let destination = bundle.start + lane * bundle.chain_length + step_index;
                    registers[destination] = value;
                }
            }};
        }

        match step.operation {
            BinaryOp::Add => execute_lanes!(left, right, left + right),
            BinaryOp::Sub => execute_lanes!(left, right, left - right),
            BinaryOp::Mul => execute_lanes!(left, right, left * right),
            BinaryOp::Div => execute_lanes!(left, right, left / right),
            BinaryOp::Rem => execute_lanes!(left, right, left % right),
            BinaryOp::Shl => execute_lanes!(
                left,
                right,
                f64::from(to_int32_number(left) << (to_uint32_number(right) & 0x1f))
            ),
            BinaryOp::Shr => execute_lanes!(
                left,
                right,
                f64::from(to_int32_number(left) >> (to_uint32_number(right) & 0x1f))
            ),
            BinaryOp::UShr => execute_lanes!(
                left,
                right,
                f64::from(to_uint32_number(left) >> (to_uint32_number(right) & 0x1f))
            ),
            BinaryOp::BitwiseAnd => execute_lanes!(
                left,
                right,
                f64::from(to_int32_number(left) & to_int32_number(right))
            ),
            BinaryOp::BitwiseXor => execute_lanes!(
                left,
                right,
                f64::from(to_int32_number(left) ^ to_int32_number(right))
            ),
            BinaryOp::BitwiseOr => execute_lanes!(
                left,
                right,
                f64::from(to_int32_number(left) | to_int32_number(right))
            ),
            _ => unreachable!("bundle lowering only admits Number binary operations"),
        }
    }

    #[inline(always)]
    fn execute_bundled_operations(
        &self,
        access: &mut TypedDenseAccess<'_, '_>,
        locals: &[f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
    ) -> bool {
        let mut cursor = self.dynamic_start;
        let mut recorded_bundle_iteration = false;
        for bundle in &self.binary_bundles {
            if bundle.start < cursor || bundle.start > self.operations.len() {
                return false;
            }
            for register in cursor..bundle.start {
                record_typed_logical_operations(1);
                record_typed_executor_steps(1);
                let Some(value) =
                    Self::execute_instruction(self.operations[register], access, locals, registers)
                else {
                    return false;
                };
                registers[register] = value;
            }
            if !recorded_bundle_iteration {
                record_typed_binary_bundle_iteration();
                recorded_bundle_iteration = true;
            }
            for (step_index, step) in bundle.steps.iter().enumerate() {
                record_typed_logical_operations(bundle.lane_count);
                record_typed_executor_steps(1);
                record_typed_binary_bundle_step(bundle.lane_count);
                Self::execute_bundle_step(bundle, step_index, step, registers);
            }
            let Some(end) = bundle.end() else {
                return false;
            };
            cursor = end;
        }
        for register in cursor..self.operations.len() {
            record_typed_logical_operations(1);
            record_typed_executor_steps(1);
            let Some(value) =
                Self::execute_instruction(self.operations[register], access, locals, registers)
            else {
                return false;
            };
            registers[register] = value;
        }
        true
    }
    pub(super) fn run_bundled<const AT_LEAST_ZERO: bool>(
        &self,
        plan: &LegacyDynamicDensePlan,
        access: &mut TypedDenseAccess<'_, '_>,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
        limit: f64,
    ) -> DynamicProgramRun {
        let mut made_progress = false;
        if !Self::should_continue::<AT_LEAST_ZERO>(locals[plan.counter_local], limit) {
            return DynamicProgramRun {
                deoptimized: false,
                made_progress,
            };
        }
        self.initialize_constants(registers);
        loop {
            access.reset_iteration();
            self.copy_entry_locals(locals, registers);
            if !self.execute_bundled_operations(access, locals, registers) {
                return DynamicProgramRun {
                    deoptimized: true,
                    made_progress,
                };
            }
            if let Some(store) = self.sunk_store {
                let Some(index) = array_index_from_number(registers[store.index]) else {
                    return DynamicProgramRun {
                        deoptimized: true,
                        made_progress,
                    };
                };
                if !Self::stage_store(
                    access,
                    store.kind,
                    store.receiver,
                    index,
                    registers[store.value],
                ) {
                    return DynamicProgramRun {
                        deoptimized: true,
                        made_progress,
                    };
                }
            }
            debug_assert_eq!(access.pending.len(), plan.store_count);
            access.commit_stores();
            for write in &plan.writes {
                locals[write.local] = registers[write.value];
            }
            #[cfg(test)]
            if plan.store_count == 0 {
                record_read_only_iteration();
            }
            made_progress = true;
            record_iteration();
            if !Self::should_continue::<AT_LEAST_ZERO>(locals[plan.counter_local], limit) {
                return DynamicProgramRun {
                    deoptimized: false,
                    made_progress,
                };
            }
        }
    }
}
