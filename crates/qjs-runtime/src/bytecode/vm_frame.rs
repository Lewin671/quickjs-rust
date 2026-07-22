use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::Value;

use super::{
    ir::Bytecode,
    vm::{FrameState, Slot},
    vm_result::Completion,
    vm_try::TryFrame,
};

// Match Bytecode's ordinary operand-stack pool: common expression stacks are
// worth retaining, while a pathological expression must not stay pinned in
// each of the scheduler's recycled frame slots.
const MAX_RECYCLED_OPERAND_STACK_CAPACITY: usize = 256;
// Locals and their optional cells scale with source declarations. Keeping the
// common small/medium frame while capping both tables prevents one generated
// function from multiplying its peak slot storage across the 64-frame pool.
const MAX_RECYCLED_LOCAL_SLOT_CAPACITY: usize = 256;
const MAX_RECYCLED_LOCAL_UPVALUE_CAPACITY: usize = 256;
// These are cold control/declaration metadata. Sixty-four entries preserve
// normal nested cleanup and sloppy-global workloads without retaining an
// adversarial number of names, try frames, or disposal scopes.
const MAX_RECYCLED_SLOPPY_GLOBAL_CAPACITY: usize = 64;
const MAX_RECYCLED_TRY_FRAME_CAPACITY: usize = 64;
const MAX_RECYCLED_DISPOSABLE_SCOPE_CAPACITY: usize = 64;

#[derive(Clone)]
pub(super) enum BytecodeOwner<'a> {
    Borrowed(&'a Bytecode),
    Shared(Rc<Bytecode>),
}

impl<'a> BytecodeOwner<'a> {
    pub(super) fn borrowed(bytecode: &'a Bytecode) -> Self {
        Self::Borrowed(bytecode)
    }

    pub(super) fn shared(bytecode: Rc<Bytecode>) -> Self {
        Self::Shared(bytecode)
    }
}

impl AsRef<Bytecode> for BytecodeOwner<'_> {
    fn as_ref(&self) -> &Bytecode {
        match self {
            Self::Borrowed(bytecode) => bytecode,
            Self::Shared(bytecode) => bytecode,
        }
    }
}

impl Deref for BytecodeOwner<'_> {
    type Target = Bytecode;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub(super) struct OperandStack<'a> {
    bytecode: BytecodeOwner<'a>,
    values: Vec<Value>,
    recycle_on_drop: bool,
}

impl<'a> OperandStack<'a> {
    pub(super) fn new(bytecode: BytecodeOwner<'a>) -> Self {
        let values = bytecode.take_operand_stack();
        Self {
            bytecode,
            values,
            recycle_on_drop: true,
        }
    }

    pub(super) fn from_recycled(bytecode: BytecodeOwner<'a>, values: Vec<Value>) -> Self {
        debug_assert!(
            values.is_empty(),
            "recycled operand storage must be cleared"
        );
        Self {
            bytecode,
            values,
            recycle_on_drop: true,
        }
    }

    fn take_for_frame_reuse(&mut self) -> Vec<Value> {
        self.recycle_on_drop = false;
        std::mem::take(&mut self.values)
    }

    pub(super) fn take(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.values)
    }

    pub(super) fn replace(&mut self, values: Vec<Value>) {
        let previous = std::mem::replace(&mut self.values, values);
        self.bytecode.recycle_operand_stack(previous);
    }
}

impl Deref for OperandStack<'_> {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for OperandStack<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl Drop for OperandStack<'_> {
    fn drop(&mut self) {
        if self.recycle_on_drop {
            self.bytecode
                .recycle_operand_stack(std::mem::take(&mut self.values));
        }
    }
}

#[derive(Default)]
pub(super) struct FrameBuffers {
    pub(super) stack: Vec<Value>,
    pub(super) locals: Vec<Slot>,
    pub(super) local_upvalues: Vec<Option<crate::function::Upvalue>>,
    pub(super) sloppy_global_names: Vec<String>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) disposable_scopes: Vec<Vec<super::vm_dispose::DisposeResource>>,
}

impl FrameBuffers {
    fn clear_and_bound(mut self) -> Self {
        self.stack = cleared_bounded_vec(self.stack, MAX_RECYCLED_OPERAND_STACK_CAPACITY);
        self.locals = cleared_bounded_vec(self.locals, MAX_RECYCLED_LOCAL_SLOT_CAPACITY);
        self.local_upvalues =
            cleared_bounded_vec(self.local_upvalues, MAX_RECYCLED_LOCAL_UPVALUE_CAPACITY);
        self.sloppy_global_names = cleared_bounded_vec(
            self.sloppy_global_names,
            MAX_RECYCLED_SLOPPY_GLOBAL_CAPACITY,
        );
        self.try_stack = cleared_bounded_vec(self.try_stack, MAX_RECYCLED_TRY_FRAME_CAPACITY);
        self.disposable_scopes = cleared_bounded_vec(
            self.disposable_scopes,
            MAX_RECYCLED_DISPOSABLE_SCOPE_CAPACITY,
        );
        self
    }
}

fn cleared_bounded_vec<T>(mut values: Vec<T>, max_capacity: usize) -> Vec<T> {
    values.clear();
    if values.capacity() > max_capacity {
        Vec::new()
    } else {
        values
    }
}

impl FrameState<'_> {
    pub(super) fn take_recyclable_buffers(&mut self) -> FrameBuffers {
        FrameBuffers {
            stack: self.stack.take_for_frame_reuse(),
            locals: std::mem::take(&mut self.locals),
            local_upvalues: std::mem::take(&mut self.local_upvalues),
            sloppy_global_names: std::mem::take(&mut self.sloppy_global_names),
            try_stack: std::mem::take(&mut self.try_stack),
            disposable_scopes: std::mem::take(&mut self.disposable_scopes),
        }
        .clear_and_bound()
    }
}

pub(super) enum FrameRun {
    Complete(Completion),
    DirectCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FrameExecution {
    Ordinary,
    Compact,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_frame_buffers_are_dropped_before_pooling() {
        let buffers = FrameBuffers {
            stack: Vec::with_capacity(MAX_RECYCLED_OPERAND_STACK_CAPACITY + 1),
            locals: Vec::with_capacity(MAX_RECYCLED_LOCAL_SLOT_CAPACITY + 1),
            local_upvalues: Vec::with_capacity(MAX_RECYCLED_LOCAL_UPVALUE_CAPACITY + 1),
            sloppy_global_names: Vec::with_capacity(MAX_RECYCLED_SLOPPY_GLOBAL_CAPACITY + 1),
            try_stack: Vec::with_capacity(MAX_RECYCLED_TRY_FRAME_CAPACITY + 1),
            disposable_scopes: Vec::with_capacity(MAX_RECYCLED_DISPOSABLE_SCOPE_CAPACITY + 1),
        }
        .clear_and_bound();

        assert_eq!(buffers.stack.capacity(), 0);
        assert_eq!(buffers.locals.capacity(), 0);
        assert_eq!(buffers.local_upvalues.capacity(), 0);
        assert_eq!(buffers.sloppy_global_names.capacity(), 0);
        assert_eq!(buffers.try_stack.capacity(), 0);
        assert_eq!(buffers.disposable_scopes.capacity(), 0);

        let retained_stack = FrameBuffers {
            stack: Vec::with_capacity(MAX_RECYCLED_OPERAND_STACK_CAPACITY),
            ..FrameBuffers::default()
        }
        .clear_and_bound()
        .stack;
        assert_eq!(
            retained_stack.capacity(),
            MAX_RECYCLED_OPERAND_STACK_CAPACITY
        );
    }

    #[test]
    fn direct_call_scheduler_signals_do_not_carry_the_prepared_payload() {
        let small_transition_limit = std::mem::size_of::<Value>() + std::mem::size_of::<usize>();
        assert!(std::mem::size_of::<FrameRun>() <= small_transition_limit);
        assert!(std::mem::size_of::<Option<bool>>() <= small_transition_limit);
    }
}
