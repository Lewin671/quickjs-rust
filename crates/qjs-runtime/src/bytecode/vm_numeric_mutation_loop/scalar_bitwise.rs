use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, function::Upvalue, to_uint32_number, value::OwnDataPropertyWrite};

use super::super::{
    ir::{Bytecode, Op},
    vm::Vm,
    vm_numeric_loop::{LoopWriteTarget, NumericLoopWrite},
    vm_props::RealmGlobalLoopWrite,
};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static SCALAR_BITWISE_LOOP_HITS: Cell<usize> = const { Cell::new(0) };
    static SCALAR_BITWISE_NATIVE_ITERATIONS: Cell<usize> = const { Cell::new(0) };
    static SCALAR_BITWISE_PUBLICATION_COMMITS: Cell<usize> = const { Cell::new(0) };
    static SCALAR_BITWISE_DEOPTS: Cell<usize> = const { Cell::new(0) };
    static SCALAR_BITWISE_PRECOMMIT_REALM_IDENTITY_SWAP: Cell<Option<&'static str>> =
        const { Cell::new(None) };
}

#[derive(Clone, Copy, Debug)]
enum NumericSource {
    Constant(f64),
    Local(usize),
}

impl NumericSource {
    fn compile(bytecode: &Bytecode, op: &Op) -> Option<Self> {
        match op {
            Op::LoadConst(index) => match bytecode.constants.get(*index)? {
                Value::Number(value) => Some(Self::Constant(*value)),
                _ => None,
            },
            Op::LoadLocal(slot) => Some(Self::Local(*slot)),
            _ => None,
        }
    }

    fn prepare(
        self,
        vm: &Vm<'_>,
        counter_slot: usize,
        accumulator_slot: usize,
    ) -> Option<PreparedNumericSource> {
        match self {
            Self::Constant(value) => Some(PreparedNumericSource::Stable {
                value,
                word: to_uint32_number(value),
                realm_cell: None,
            }),
            Self::Local(slot) if slot == counter_slot => Some(PreparedNumericSource::Counter),
            Self::Local(slot) if slot == accumulator_slot => {
                Some(PreparedNumericSource::Accumulator)
            }
            Self::Local(slot) if vm.slot_is_authoritative(slot) => {
                let value = local_number(vm, slot)?;
                Some(PreparedNumericSource::Stable {
                    value,
                    word: to_uint32_number(value),
                    realm_cell: None,
                })
            }
            Self::Local(slot) => {
                let cell = vm.prepare_realm_global_loop_read(slot)?;
                let value = local_number(vm, slot)?;
                Some(PreparedNumericSource::Stable {
                    value,
                    word: to_uint32_number(value),
                    realm_cell: Some(cell),
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PreparedNumericSource {
    Counter,
    Accumulator,
    Stable {
        value: f64,
        word: u32,
        realm_cell: Option<Upvalue>,
    },
}

impl PreparedNumericSource {
    fn stable_value(&self) -> Option<f64> {
        match self {
            Self::Stable { value, .. } => Some(*value),
            Self::Counter | Self::Accumulator => None,
        }
    }

    fn aliases_any(&self, cells: &[Upvalue]) -> bool {
        let Self::Stable {
            realm_cell: Some(cell),
            ..
        } = self
        else {
            return false;
        };
        cells.iter().any(|candidate| candidate.ptr_eq(cell))
    }
}

#[derive(Clone, Debug)]
enum ScalarWrite {
    Indexed(NumericLoopWrite),
    SloppyGlobal { slot: usize, name: String },
}

impl ScalarWrite {
    fn compile_local_or_realm(op: &Op, expected_slot: usize) -> Option<Self> {
        NumericLoopWrite::compile(op, expected_slot).map(Self::Indexed)
    }

    fn compile_accumulator(bytecode: &Bytecode, read: &ScalarRead, op: &Op) -> Option<Self> {
        match read {
            ScalarRead::Local(slot) => Self::compile_local_or_realm(op, *slot),
            ScalarRead::Global(name) => match op {
                Op::StoreLocalOrGlobalSloppy {
                    slot,
                    name: store_name,
                } if store_name == name
                    && bytecode.local_slot(name) == Some(*slot)
                    && bytecode.local_is_sloppy_global_fallback(*slot) =>
                {
                    Some(Self::SloppyGlobal {
                        slot: *slot,
                        name: name.clone(),
                    })
                }
                Op::StoreGlobalSloppy {
                    slot,
                    name: store_name,
                } if store_name == name && bytecode.local_slot(name) == Some(*slot) => {
                    Self::compile_local_or_realm(op, *slot)
                }
                _ => None,
            },
        }
    }

    fn slot(&self) -> usize {
        match self {
            Self::Indexed(write) => write.slot(),
            Self::SloppyGlobal { slot, .. } => *slot,
        }
    }

    fn prepare(&self, vm: &Vm<'_>) -> Option<PreparedScalarWrite> {
        let target = match self {
            Self::Indexed(write) => PreparedScalarWriteTarget::Indexed(write.prepare(vm)?),
            Self::SloppyGlobal { slot, name } => PreparedScalarWriteTarget::SloppyGlobal(
                PreparedSloppyGlobalWrite::prepare(vm, *slot, name)?,
            ),
        };
        Some(PreparedScalarWrite {
            write: self.clone(),
            target,
        })
    }
}

#[derive(Clone, Debug)]
struct PreparedScalarWrite {
    write: ScalarWrite,
    target: PreparedScalarWriteTarget,
}

impl PreparedScalarWrite {
    fn number(&self, vm: &Vm<'_>) -> Option<f64> {
        match &self.target {
            PreparedScalarWriteTarget::Indexed(target) => target.number(vm),
            PreparedScalarWriteTarget::SloppyGlobal(target) => Some(target.number),
        }
    }

    fn realm_cell(&self) -> Option<Upvalue> {
        match &self.target {
            PreparedScalarWriteTarget::Indexed(target) => target.realm_cell(),
            PreparedScalarWriteTarget::SloppyGlobal(target) => Some(target.cell.clone()),
        }
    }

    fn revalidate_sloppy(&self, vm: &Vm<'_>) -> bool {
        let PreparedScalarWriteTarget::SloppyGlobal(expected) = &self.target else {
            return true;
        };
        let ScalarWrite::SloppyGlobal { slot, name } = &self.write else {
            unreachable!("prepared sloppy target retains its source write")
        };
        PreparedSloppyGlobalWrite::prepare(vm, *slot, name)
            .is_some_and(|current| current.same_target(expected))
    }

    fn realm_write(&self, value: f64) -> Option<(RealmGlobalLoopWrite, f64)> {
        match &self.target {
            PreparedScalarWriteTarget::Indexed(LoopWriteTarget::RealmGlobal(target)) => {
                Some((target.clone(), value))
            }
            PreparedScalarWriteTarget::Indexed(LoopWriteTarget::Local { .. })
            | PreparedScalarWriteTarget::SloppyGlobal(_) => None,
        }
    }

    fn commit_non_realm(self, vm: &mut Vm<'_>, value: f64) {
        match self.target {
            PreparedScalarWriteTarget::Indexed(LoopWriteTarget::Local { slot }) => {
                vm.locals[slot] = Some(Value::Number(value));
            }
            PreparedScalarWriteTarget::Indexed(LoopWriteTarget::RealmGlobal(_)) => {}
            PreparedScalarWriteTarget::SloppyGlobal(target) => target.commit(vm, value),
        }
    }
}

#[derive(Clone, Debug)]
enum PreparedScalarWriteTarget {
    Indexed(LoopWriteTarget),
    SloppyGlobal(PreparedSloppyGlobalWrite),
}

#[derive(Clone, Debug)]
struct PreparedSloppyGlobalWrite {
    slot: usize,
    name: String,
    number: f64,
    cell: Upvalue,
    global_this: crate::ObjectRef,
}

impl PreparedSloppyGlobalWrite {
    fn prepare(vm: &Vm<'_>, slot: usize, name: &str) -> Option<Self> {
        if !vm.transactional_realm_globals
            || !vm.bytecode.is_global_scope()
            || !vm.persist_global_lexicals
            || vm.dynamic_code_executed
            || vm.direct_eval_with_stack
            || !vm.with_stack.is_empty()
            || vm.bytecode.contains_direct_eval()
            || vm.bytecode.contains_with()
            || vm.env.deopt_bindings().is_some()
            || vm.env.has_module_imports()
            || vm.env.dynamic_function_realm_global().is_some()
            || vm
                .bytecode
                .global_names()
                .iter()
                .any(|name| matches!(name.as_str(), "eval" | "Function" | "$262"))
            || vm.env.has_local_binding(name)
            || vm.env.has_module_import(name)
            || vm.env.is_global_lexical_binding(name)
            || vm.env.is_immutable_lexical_binding(name)
            || vm.env.is_immutable_function_name(name)
        {
            return None;
        }
        let local = vm.bytecode.locals.get(slot)?;
        if local.name != name
            || !local.mutable
            || !local.sloppy_global_fallback
            || vm.bytecode.local_slot(name) != Some(slot)
        {
            return None;
        }

        let global_this = vm.cached_global_this()?;
        let property = global_this.own_property(name)?;
        if property.is_accessor() || !property.writable {
            return None;
        }
        let Value::Number(number) = vm.locals.get(slot)?.as_ref()?.clone() else {
            return None;
        };
        let expected = Value::Number(number);
        if !property.value.same_value(&expected) {
            return None;
        }
        let cell = vm.env.realm_binding_cell(name)?;
        if !vm.env.is_realm_binding_cell(name, &cell)
            || !cell.get().same_value(&expected)
            || !vm.env.get_realm(name)?.same_value(&expected)
        {
            return None;
        }
        if let Some(local_cell) = vm.local_upvalues.get(slot)?.as_ref()
            && !local_cell.ptr_eq(&cell)
        {
            return None;
        }

        Some(Self {
            slot,
            name: name.to_owned(),
            number,
            cell,
            global_this,
        })
    }

    fn same_target(&self, other: &Self) -> bool {
        self.slot == other.slot
            && self.name == other.name
            && self.cell.ptr_eq(&other.cell)
            && self.global_this.ptr_eq(&other.global_this)
            && Value::Number(self.number).same_value(&Value::Number(other.number))
    }

    fn commit(self, vm: &mut Vm<'_>, number: f64) {
        let value = Value::Number(number);
        match self
            .global_this
            .write_existing_own_data_property(&self.name, &value)
        {
            OwnDataPropertyWrite::Written => {}
            OwnDataPropertyWrite::ReadOnly | OwnDataPropertyWrite::NeedsSlowPath => {
                unreachable!("revalidated sloppy-global loop target changed during commit")
            }
        }
        let replaced =
            vm.env
                .replace_existing_realm_with_cell(&self.name, value.clone(), &self.cell);
        debug_assert!(replaced, "revalidated sloppy-global cell must still exist");
        vm.locals[self.slot] = Some(value);
        vm.sync_marked_dynamic_global(&self.name);
        vm.record_sloppy_global_name(&self.name);
    }
}

#[derive(Clone, Debug)]
enum ScalarRead {
    Local(usize),
    Global(String),
}

impl ScalarRead {
    fn compile(op: &Op) -> Option<Self> {
        match op {
            Op::LoadLocal(slot) => Some(Self::Local(*slot)),
            Op::LoadGlobal(name) => Some(Self::Global(name.clone())),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BitwiseRecurrence {
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    And,
    Xor,
    Or,
}

impl BitwiseRecurrence {
    fn compile(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Shl => Some(Self::ShiftLeft),
            BinaryOp::Shr => Some(Self::ShiftRight),
            BinaryOp::UShr => Some(Self::UnsignedShiftRight),
            BinaryOp::BitwiseAnd => Some(Self::And),
            BinaryOp::BitwiseXor => Some(Self::Xor),
            BinaryOp::BitwiseOr => Some(Self::Or),
            _ => None,
        }
    }

    fn number(self, word: u32) -> f64 {
        match self {
            Self::UnsignedShiftRight => f64::from(word),
            Self::ShiftLeft | Self::ShiftRight | Self::And | Self::Xor | Self::Or => {
                f64::from(word as i32)
            }
        }
    }
}

macro_rules! apply_native_word {
    (shift_left, $left:ident, $right:expr) => {
        $left = (($left as i32) << ($right & 0x1f)) as u32
    };
    (shift_right, $left:ident, $right:expr) => {
        $left = (($left as i32) >> ($right & 0x1f)) as u32
    };
    (unsigned_shift_right, $left:ident, $right:expr) => {
        $left >>= $right & 0x1f
    };
    (and, $left:ident, $right:expr) => {
        $left &= $right
    };
    (xor, $left:ident, $right:expr) => {
        $left ^= $right
    };
    (or, $left:ident, $right:expr) => {
        $left |= $right
    };
}

macro_rules! run_native_kernel {
    ($operation:ident, counter; $initial_accumulator:expr, $initial_counter:expr, $remaining:expr) => {{
        let mut accumulator_word = $initial_accumulator;
        let mut counter_word = $initial_counter;
        for _ in 0..$remaining {
            apply_native_word!($operation, accumulator_word, counter_word);
            counter_word = counter_word.wrapping_add(1);
        }
        accumulator_word
    }};
    ($operation:ident, accumulator; $initial_accumulator:expr, $initial_counter:expr, $remaining:expr) => {{
        let mut accumulator_word = $initial_accumulator;
        let mut counter_word = $initial_counter;
        for _ in 0..$remaining {
            apply_native_word!($operation, accumulator_word, accumulator_word);
            counter_word = counter_word.wrapping_add(1);
        }
        accumulator_word
    }};
    ($operation:ident, stable($operand_word:expr); $initial_accumulator:expr, $initial_counter:expr, $remaining:expr) => {{
        let mut accumulator_word = $initial_accumulator;
        let mut counter_word = $initial_counter;
        for _ in 0..$remaining {
            apply_native_word!($operation, accumulator_word, $operand_word);
            counter_word = counter_word.wrapping_add(1);
        }
        accumulator_word
    }};
}

macro_rules! dispatch_native_recurrence {
    ($recurrence:expr, [$($source:tt)+], $initial_accumulator:expr, $initial_counter:expr, $remaining:expr) => {{
        match $recurrence {
            BitwiseRecurrence::ShiftLeft => run_native_kernel!(
                shift_left, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
            BitwiseRecurrence::ShiftRight => run_native_kernel!(
                shift_right, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
            BitwiseRecurrence::UnsignedShiftRight => run_native_kernel!(
                unsigned_shift_right, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
            BitwiseRecurrence::And => run_native_kernel!(
                and, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
            BitwiseRecurrence::Xor => run_native_kernel!(
                xor, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
            BitwiseRecurrence::Or => run_native_kernel!(
                or, $($source)+; $initial_accumulator, $initial_counter, $remaining
            ),
        }
    }};
}

/// Selects both dynamic dimensions once. Every generated inner loop contains
/// only direct integer operands and one fixed bitwise operation.
#[inline(never)]
fn run_native_bitwise_kernel(
    recurrence: BitwiseRecurrence,
    operand: PreparedNumericSource,
    accumulator_word: u32,
    counter_word: u32,
    remaining: u64,
) -> u32 {
    match operand {
        PreparedNumericSource::Counter => dispatch_native_recurrence!(
            recurrence,
            [counter],
            accumulator_word,
            counter_word,
            remaining
        ),
        PreparedNumericSource::Accumulator => dispatch_native_recurrence!(
            recurrence,
            [accumulator],
            accumulator_word,
            counter_word,
            remaining
        ),
        PreparedNumericSource::Stable { word, .. } => dispatch_native_recurrence!(
            recurrence,
            [stable(word)],
            accumulator_word,
            counter_word,
            remaining
        ),
    }
}

/// A side-effect-free scalar bitwise recurrence carried by the already-paid
/// numeric-mutation loop lookup.
///
/// The fixed opcode order below is the compiler's canonical lowering for
/// `for (counter; counter < limit; counter++) accumulator <bitop>= operand`:
/// test, assignment value, statement completion, then postfix update. It is a
/// semantic shape rather than a source identity: names, slots, constants,
/// iteration counts, and all six number bitwise operators remain variable.
/// Every dynamic binding and value guard runs once before the integer loop.
#[derive(Clone, Debug)]
pub(super) struct ScalarBitwiseLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    accumulator_slot: usize,
    result_slot: usize,
    limit: NumericSource,
    operand: NumericSource,
    recurrence: BitwiseRecurrence,
    accumulator_write: ScalarWrite,
    counter_write: ScalarWrite,
}

impl ScalarBitwiseLoopPlan {
    pub(super) fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(counter_slot),
            limit_op,
            Op::Binary(BinaryOp::Lt),
            Op::JumpIfFalse(exit),
            Op::Pop,
        ) = (
            code.get(header)?,
            code.get(header + 1)?,
            code.get(header + 2)?,
            code.get(header + 3)?,
            code.get(header + 4)?,
        )
        else {
            return None;
        };
        let limit = NumericSource::compile(bytecode, limit_op)?;
        if !matches!(code.get(*exit), Some(Op::Pop)) {
            return None;
        }

        let cursor = header + 5;
        let accumulator_read = ScalarRead::compile(code.get(cursor)?)?;
        let operand = NumericSource::compile(bytecode, code.get(cursor + 1)?)?;
        let Op::Binary(recurrence) = code.get(cursor + 2)? else {
            return None;
        };
        let recurrence = BitwiseRecurrence::compile(*recurrence)?;
        if !matches!(code.get(cursor + 3), Some(Op::Dup)) {
            return None;
        }
        let accumulator_write =
            ScalarWrite::compile_accumulator(bytecode, &accumulator_read, code.get(cursor + 4)?)?;
        let accumulator_slot = accumulator_write.slot();
        if accumulator_slot == *counter_slot
            || matches!(limit, NumericSource::Local(slot) if slot == *counter_slot || slot == accumulator_slot)
        {
            return None;
        }
        let Op::StoreLocal(result_slot) = code.get(cursor + 5)? else {
            return None;
        };

        let tail = cursor + 6;
        if !matches!(code.get(tail), Some(Op::LoadLocal(slot)) if slot == counter_slot)
            || !matches!(code.get(tail + 1), Some(Op::ToNumeric))
            || !matches!(code.get(tail + 2), Some(Op::Dup))
        {
            return None;
        }
        if !matches!(code.get(tail + 3), Some(Op::Update(UpdateOp::Increment))) {
            return None;
        }
        let counter_write =
            ScalarWrite::compile_local_or_realm(code.get(tail + 4)?, *counter_slot)?;
        if !matches!(code.get(tail + 5), Some(Op::Pop))
            || !matches!(code.get(tail + 6), Some(Op::Jump(target)) if target == &header)
            || tail + 6 != backedge
            || *result_slot == *counter_slot
            || *result_slot == accumulator_slot
        {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_slot: *counter_slot,
            accumulator_slot,
            result_slot: *result_slot,
            limit,
            operand,
            recurrence,
            accumulator_write,
            counter_write,
        })
    }

    pub(super) fn exit(&self) -> usize {
        self.exit
    }

    pub(super) fn contains_instruction(&self, ip: usize) -> bool {
        (self.header..=self.backedge).contains(&ip)
    }

    pub(super) fn try_run(&self, vm: &mut Vm<'_>) -> ScalarBitwiseLoopRun {
        if vm.direct_eval_with_stack
            || vm.bytecode.contains_direct_eval()
            || vm.bytecode.contains_with()
            || !vm.slot_is_authoritative(self.result_slot)
        {
            return ScalarBitwiseLoopRun::suppress();
        }
        let Some(counter_write) = self.counter_write.prepare(vm) else {
            return ScalarBitwiseLoopRun::suppress();
        };
        let Some(accumulator_write) = self.accumulator_write.prepare(vm) else {
            return ScalarBitwiseLoopRun::suppress();
        };
        let Some(counter) = counter_write.number(vm) else {
            return ScalarBitwiseLoopRun::suppress();
        };
        let Some(accumulator) = accumulator_write.number(vm) else {
            return ScalarBitwiseLoopRun::suppress();
        };
        let Some(limit) = self
            .limit
            .prepare(vm, self.counter_slot, self.accumulator_slot)
        else {
            return ScalarBitwiseLoopRun::suppress();
        };
        let Some(operand) = self
            .operand
            .prepare(vm, self.counter_slot, self.accumulator_slot)
        else {
            return ScalarBitwiseLoopRun::suppress();
        };

        let target_cells = [counter_write.realm_cell(), accumulator_write.realm_cell()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        if target_cells
            .iter()
            .enumerate()
            .any(|(index, cell)| target_cells[..index].iter().any(|seen| seen.ptr_eq(cell)))
            || limit.aliases_any(&target_cells)
            || operand.aliases_any(&target_cells)
        {
            return ScalarBitwiseLoopRun::suppress();
        }
        let Some(limit) = limit.stable_value() else {
            return ScalarBitwiseLoopRun::suppress();
        };
        if !is_safe_integer(counter) || !is_safe_integer(limit) || counter >= limit {
            return ScalarBitwiseLoopRun::suppress();
        }
        let remaining_number = limit - counter;
        if !is_safe_integer(remaining_number)
            || remaining_number <= 0.0
            || counter + remaining_number != limit
        {
            return ScalarBitwiseLoopRun::suppress();
        }
        let remaining = remaining_number as u64;

        #[cfg(test)]
        {
            SCALAR_BITWISE_LOOP_HITS.with(|hits| hits.set(hits.get() + 1));
            SCALAR_BITWISE_NATIVE_ITERATIONS.with(|iterations| {
                iterations.set(
                    iterations
                        .get()
                        .saturating_add(usize::try_from(remaining).unwrap_or(usize::MAX)),
                );
            });
        }

        let accumulator_word = run_native_bitwise_kernel(
            self.recurrence,
            operand,
            to_uint32_number(accumulator),
            to_uint32_number(counter),
            remaining,
        );
        // Preserve Number addition's signed-zero result. The loop limit can be
        // `-0`, while postfix `-1 + 1` must publish `+0` as the final counter.
        let counter = counter + remaining_number;
        let accumulator = self.recurrence.number(accumulator_word);

        // All dynamic identities are checked before the first publication. The
        // integer loop above cannot execute JS, so a successful recheck also
        // proves the following ordinary data writes remain non-observable.
        #[cfg(test)]
        run_precommit_test_hook(vm);
        if !counter_write.revalidate_sloppy(vm) || !accumulator_write.revalidate_sloppy(vm) {
            return ScalarBitwiseLoopRun::suppress();
        }
        let realm_writes = [
            counter_write.realm_write(counter),
            accumulator_write.realm_write(accumulator),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        if !vm.commit_realm_global_loop_writes(&realm_writes) {
            return ScalarBitwiseLoopRun::suppress();
        }
        counter_write.commit_non_realm(vm, counter);
        accumulator_write.commit_non_realm(vm, accumulator);
        vm.locals[self.result_slot] = Some(Value::Number(accumulator));
        vm.ip = self.exit + 1;
        #[cfg(test)]
        SCALAR_BITWISE_PUBLICATION_COMMITS.with(|commits| commits.set(commits.get() + 1));
        ScalarBitwiseLoopRun::Handled
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScalarBitwiseLoopRun {
    Handled,
    Suppress,
}

impl ScalarBitwiseLoopRun {
    #[inline(always)]
    fn suppress() -> Self {
        #[cfg(test)]
        SCALAR_BITWISE_DEOPTS.with(|deopts| deopts.set(deopts.get() + 1));
        Self::Suppress
    }
}

#[cfg(test)]
fn run_precommit_test_hook(vm: &mut Vm<'_>) {
    SCALAR_BITWISE_PRECOMMIT_REALM_IDENTITY_SWAP.with(|swap| {
        let Some(name) = swap.take() else {
            return;
        };
        let value = vm
            .env
            .remove_realm(name)
            .unwrap_or_else(|| panic!("test realm binding `{name}` should exist"));
        assert!(vm.env.insert_realm(name.to_owned(), value).is_none());
    });
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn is_safe_integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Property, Value, bytecode::compiler, eval};

    fn compile(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        compiler::compile_script(&script).expect("source should compile")
    }

    fn nested_function(source: &str) -> Bytecode {
        compile(source)
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    fn scalar_plan(bytecode: &Bytecode) -> ScalarBitwiseLoopPlan {
        bytecode
            .code
            .iter()
            .enumerate()
            .find_map(|(backedge, op)| match op {
                Op::Jump(header) if *header < backedge => {
                    ScalarBitwiseLoopPlan::compile(bytecode, *header, backedge)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected scalar bitwise plan: {:#?}", bytecode.code))
    }

    fn reset_hits() {
        SCALAR_BITWISE_LOOP_HITS.with(|hits| hits.set(0));
        SCALAR_BITWISE_NATIVE_ITERATIONS.with(|iterations| iterations.set(0));
        SCALAR_BITWISE_PUBLICATION_COMMITS.with(|commits| commits.set(0));
        SCALAR_BITWISE_DEOPTS.with(|deopts| deopts.set(0));
        SCALAR_BITWISE_PRECOMMIT_REALM_IDENTITY_SWAP.with(|swap| swap.set(None));
    }

    fn hits() -> usize {
        SCALAR_BITWISE_LOOP_HITS.with(Cell::get)
    }

    fn native_iterations() -> usize {
        SCALAR_BITWISE_NATIVE_ITERATIONS.with(Cell::get)
    }

    fn publication_commits() -> usize {
        SCALAR_BITWISE_PUBLICATION_COMMITS.with(Cell::get)
    }

    fn deopts() -> usize {
        SCALAR_BITWISE_DEOPTS.with(Cell::get)
    }

    #[test]
    fn recognizes_the_complete_bitwise_family_without_workload_constants() {
        for (operator, recurrence) in [
            ("&", BitwiseRecurrence::And),
            ("|", BitwiseRecurrence::Or),
            ("^", BitwiseRecurrence::Xor),
            ("<<", BitwiseRecurrence::ShiftLeft),
            (">>", BitwiseRecurrence::ShiftRight),
            (">>>", BitwiseRecurrence::UnsignedShiftRight),
        ] {
            let bytecode = nested_function(&format!(
                "function fold(limit, value, operand) {{ for (var index = 0; index < limit; index++) value = value {operator} operand; return value; }}"
            ));
            let plan = scalar_plan(&bytecode);
            assert_eq!(plan.recurrence, recurrence, "{operator}");
            assert!(matches!(plan.limit, NumericSource::Local(_)), "{operator}");
            assert!(
                matches!(plan.operand, NumericSource::Local(_)),
                "{operator}"
            );
        }
    }

    #[test]
    fn source_specialized_kernels_cover_every_recurrence() {
        for recurrence in [
            BitwiseRecurrence::ShiftLeft,
            BitwiseRecurrence::ShiftRight,
            BitwiseRecurrence::UnsignedShiftRight,
            BitwiseRecurrence::And,
            BitwiseRecurrence::Xor,
            BitwiseRecurrence::Or,
        ] {
            for (source, expected) in [
                (PreparedNumericSource::Counter, 2_147_483_645),
                (PreparedNumericSource::Accumulator, 0),
                (
                    PreparedNumericSource::Stable {
                        value: 33.0,
                        word: 33,
                        realm_cell: None,
                    },
                    33,
                ),
            ] {
                let actual = run_native_bitwise_kernel(recurrence, source, u32::MAX, 31, 3);
                let mut reference = u32::MAX;
                let mut counter = 31_u32;
                for _ in 0..3 {
                    let operand = match expected {
                        2_147_483_645 => counter,
                        0 => reference,
                        stable => stable,
                    };
                    reference = match recurrence {
                        BitwiseRecurrence::ShiftLeft => {
                            ((reference as i32) << (operand & 0x1f)) as u32
                        }
                        BitwiseRecurrence::ShiftRight => {
                            ((reference as i32) >> (operand & 0x1f)) as u32
                        }
                        BitwiseRecurrence::UnsignedShiftRight => reference >> (operand & 0x1f),
                        BitwiseRecurrence::And => reference & operand,
                        BitwiseRecurrence::Xor => reference ^ operand,
                        BitwiseRecurrence::Or => reference | operand,
                    };
                    counter = counter.wrapping_add(1);
                }
                assert_eq!(actual, reference, "{recurrence:?}, source {expected}");
            }
        }
    }

    #[test]
    fn scalar_payload_is_outlined_behind_the_existing_special_plan_rc() {
        let bytecode = nested_function(
            "function fold(limit, value) { for (var i = 0; i < limit; i++) value &= i; return value; }",
        );
        let plans = super::super::NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        let super::super::NumericMutationLoopKind::Special(special) = &plans[0].kind else {
            panic!("scalar recurrence must not enlarge the inline plan kind");
        };
        assert!(matches!(
            special.as_ref(),
            super::super::SpecialPlan::ScalarBitwise(_)
        ));
    }

    #[test]
    fn executes_undeclared_sloppy_global_recurrence_with_one_batched_entry() {
        reset_hits();
        assert_eq!(
            eval(
                "bitwiseValue = 4294967296; \
                 for (var arbitraryCounter = 0; arbitraryCounter < 600000; arbitraryCounter++) \
                   bitwiseValue = bitwiseValue & arbitraryCounter; \
                 bitwiseValue + ':' + arbitraryCounter + ':' + globalThis.bitwiseValue;"
            ),
            Ok(Value::String("0:600000:0".to_owned().into()))
        );
        assert_eq!(hits(), 1);
        assert_eq!(native_iterations(), 599_999);
        assert_eq!(publication_commits(), 1);
        assert_eq!(deopts(), 0);
    }

    #[test]
    fn realm_identity_mismatch_suppresses_without_partial_publication() {
        reset_hits();
        let bytecode = compile(
            "identityValue = 7; for (var identityCounter = 0; identityCounter < 4; identityCounter++) identityValue &= identityCounter; identityValue;",
        );
        let plan = scalar_plan(&bytecode);
        let mut vm = Vm::new(&bytecode).expect("script VM should initialize");
        let global_this = vm.cached_global_this().expect("global object should exist");

        for (slot, name, number) in [
            (plan.counter_slot, "identityCounter", 1.0),
            (plan.accumulator_slot, "identityValue", 0.0),
        ] {
            let value = Value::Number(number);
            global_this.define_property(
                name.to_owned(),
                Property::data(value.clone(), true, true, true),
            );
            if !vm.env.replace_existing_realm(name, value.clone()) {
                assert!(
                    vm.env
                        .insert_realm(name.to_owned(), value.clone())
                        .is_none()
                );
            }
            vm.locals[slot] = Some(value);
        }
        let result_before = vm.locals[plan.result_slot].clone();
        let ip_before = vm.ip;
        SCALAR_BITWISE_PRECOMMIT_REALM_IDENTITY_SWAP.with(|swap| swap.set(Some("identityValue")));

        assert_eq!(plan.try_run(&mut vm), ScalarBitwiseLoopRun::Suppress);
        assert_eq!(hits(), 1);
        assert_eq!(native_iterations(), 3);
        assert_eq!(publication_commits(), 0);
        assert_eq!(deopts(), 1);
        assert_eq!(vm.ip, ip_before);
        assert_eq!(vm.locals[plan.result_slot], result_before);
        assert_eq!(vm.locals[plan.counter_slot], Some(Value::Number(1.0)));
        assert_eq!(vm.locals[plan.accumulator_slot], Some(Value::Number(0.0)));
        assert_eq!(
            vm.env.get_realm("identityCounter"),
            Some(Value::Number(1.0))
        );
        assert_eq!(vm.env.get_realm("identityValue"), Some(Value::Number(0.0)));
        assert_eq!(
            global_this
                .own_property("identityCounter")
                .map(|property| property.value),
            Some(Value::Number(1.0))
        );
        assert_eq!(
            global_this
                .own_property("identityValue")
                .map(|property| property.value),
            Some(Value::Number(0.0))
        );
    }

    #[test]
    fn executes_signed_unsigned_shift_and_boolean_recurrences() {
        for (source, expected) in [
            (
                "function run(value) { for (var i = 1; i < 5; i++) value = value & i; return value + ':' + i; } run(-1);",
                "0:5",
            ),
            (
                "function run(value) { for (var i = 1; i < 5; i++) value = value | i; return value + ':' + i; } run(0);",
                "7:5",
            ),
            (
                "function run(value) { for (var i = 1; i < 5; i++) value = value ^ i; return value + ':' + i; } run(0);",
                "4:5",
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value << i; return value + ':' + i; } run(1);",
                "8:3",
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value >> 1; return value + ':' + i; } run(-16);",
                "-2:3",
            ),
            (
                "function run(value) { for (var i = 0; i < 2; i++) value = value >>> 1; return value + ':' + i; } run(-1);",
                "1073741823:2",
            ),
        ] {
            reset_hits();
            assert_eq!(
                eval(source),
                Ok(Value::String(expected.to_owned().into())),
                "{source}"
            );
            assert_eq!(hits(), 1, "{source}");
        }
    }

    #[test]
    fn counter_words_wrap_without_changing_the_safe_integer_loop_counter() {
        reset_hits();
        assert_eq!(
            eval(
                "function run(value) { for (var i = 4294967294; i < 4294967298; i++) value = value ^ i; return value + ':' + i; } run(0);"
            ),
            Ok(Value::String("0:4294967298".to_owned().into()))
        );
        assert_eq!(hits(), 1);
        assert_eq!(native_iterations(), 3);
    }

    #[test]
    fn final_counter_uses_number_addition_instead_of_copying_negative_zero_limit() {
        reset_hits();
        assert_eq!(
            eval(
                "function run(limit, value) { for (var i = -2; i < limit; i++) value ^= i; return Object.is(i, -0) + ':' + value; } run(-0, 0);"
            ),
            Ok(Value::String("false:1".to_owned().into()))
        );
        assert_eq!(hits(), 1);
        assert_eq!(native_iterations(), 1);
    }

    #[test]
    fn preserves_number_to_int32_and_shift_count_boundaries() {
        for (source, expected) in [
            (
                "function run(value, rhs) { for (var i = 0; i < 3; i++) value = value | rhs; return value; } run(NaN, Infinity);",
                Value::Number(0.0),
            ),
            (
                "function run(value, rhs) { for (var i = 0; i < 3; i++) value = value ^ rhs; return Object.is(value, -0); } run(-0, -0);",
                Value::Boolean(false),
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value >>> 0; return value; } run(-1);",
                Value::Number(4_294_967_295.0),
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value << 33; return value; } run(1);",
                Value::Number(8.0),
            ),
            (
                "function run(value, rhs) { for (var i = 0; i < 2; i++) value = value << rhs; return value; } run(1, -1);",
                Value::Number(0.0),
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value << 32; return value; } run(1);",
                Value::Number(1.0),
            ),
            (
                "function run(value) { for (var i = 0; i < 3; i++) value = value >> 1.9; return value; } run(8);",
                Value::Number(1.0),
            ),
        ] {
            reset_hits();
            assert_eq!(eval(source), Ok(expected), "{source}");
            assert_eq!(hits(), 1, "{source}");
        }
    }

    #[test]
    fn compound_assignment_with_unrelated_names_and_count_uses_the_same_shape() {
        reset_hits();
        assert_eq!(
            eval(
                "function scramble(rounds, state, mask) { for (var cursor = 0; cursor < rounds; cursor++) state ^= mask; return state + ':' + cursor; } scramble(17, 123, 42);"
            ),
            Ok(Value::String("81:17".to_owned().into()))
        );
        assert_eq!(hits(), 1);
        assert_eq!(native_iterations(), 16);
    }

    #[test]
    fn object_coercion_and_bigint_values_replay_in_the_interpreter() {
        reset_hits();
        assert_eq!(
            eval(
                "var coercions = 0; var operand = { valueOf: function () { coercions++; return 3; } }; \
                 function run(value) { for (var i = 0; i < 4; i++) value = value & operand; return value + ':' + i; } \
                 run(7) + ':' + coercions;"
            ),
            Ok(Value::String("3:4:4".to_owned().into()))
        );
        assert_eq!(hits(), 0);

        reset_hits();
        assert_eq!(
            eval(
                "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value & rhs; return value === 1n; } run(5n, 3n);"
            ),
            Ok(Value::Boolean(true))
        );
        assert_eq!(hits(), 0);
        assert!(eval(
            "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value & rhs; return value; } run(5, 3n);"
        )
        .is_err());
        assert_eq!(hits(), 0);
        reset_hits();
        assert!(eval(
            "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value >>> rhs; return value; } run(5n, 1n);"
        )
        .is_err());
        assert_eq!(hits(), 0);
    }

    #[test]
    fn accessor_eval_and_capture_guards_prove_fallback_execution() {
        reset_hits();
        assert_eq!(
            eval(
                "guardedValue = 7; var gets = 0; \
                 Object.defineProperty(globalThis, 'guardedValue', { configurable: true, \
                   get: function () { gets++; return 7; } }); \
                 for (var i = 0; i < 4; i++) guardedValue = guardedValue & i; \
                 gets + ':' + i;"
            ),
            Ok(Value::String("4:4".to_owned().into()))
        );
        assert_eq!(hits(), 0);

        reset_hits();
        assert_eq!(
            eval(
                "readOnlyValue = 7; Object.defineProperty(globalThis, 'readOnlyValue', { writable: false }); \
                 for (var i = 0; i < 4; i++) readOnlyValue = readOnlyValue & i; \
                 readOnlyValue + ':' + i;"
            ),
            Ok(Value::String("7:4".to_owned().into()))
        );
        assert_eq!(hits(), 0);

        reset_hits();
        assert_eq!(
            eval(
                "function run(value) { eval('value = value'); for (var i = 0; i < 4; i++) value = value ^ i; return value; } run(0);"
            ),
            Ok(Value::Number(0.0))
        );
        assert_eq!(hits(), 0);

        reset_hits();
        assert_eq!(
            eval(
                "function run(value) { function read() { return value; } for (var i = 0; i < 4; i++) value = value | i; return value + read(); } run(0);"
            ),
            Ok(Value::Number(6.0))
        );
        assert_eq!(hits(), 0);
    }

    #[test]
    fn fractional_limits_and_single_iteration_loops_suppress_without_partial_progress() {
        reset_hits();
        assert_eq!(
            eval(
                "function run(limit, value) { for (var i = 0; i < limit; i++) value = value ^ i; return value + ':' + i; } run(3.5, 0);"
            ),
            Ok(Value::String("0:4".to_owned().into()))
        );
        assert_eq!(hits(), 0);
        assert_eq!(native_iterations(), 0);

        reset_hits();
        assert_eq!(
            eval(
                "function run(value) { for (var i = 0; i < 1; i++) value = value | i; return value + ':' + i; } run(3);"
            ),
            Ok(Value::String("3:1".to_owned().into()))
        );
        assert_eq!(hits(), 0);
        assert_eq!(native_iterations(), 0);
    }

    #[test]
    fn rejects_extra_body_effects_and_noncanonical_updates_statically() {
        for source in [
            "function run(value, other) { for (var i = 0; i < 4; i++) { other++; value = value & i; } return value; }",
            "function run(value) { for (var i = 0; i < 4; i += 2) value = value & i; return value; }",
            "function run(value, test) { for (var i = 0; test(); i++) value = value & i; return value; }",
        ] {
            let bytecode = nested_function(source);
            assert!(
                bytecode
                    .code
                    .iter()
                    .enumerate()
                    .filter_map(|(backedge, op)| match op {
                        Op::Jump(header) if *header < backedge => {
                            ScalarBitwiseLoopPlan::compile(&bytecode, *header, backedge)
                        }
                        _ => None,
                    })
                    .next()
                    .is_none(),
                "{source}: {:#?}",
                bytecode.code
            );
        }
    }
}
