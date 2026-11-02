//! Contiguous register/header storage for non-suspendable indexed execution.
//! Drivers take ownership across re-entry; no slice survives stack growth.
//! Prefix and compiler-declared writes are the only owning slots cleared.
use crate::context::{
    ActivationBorrow, ActivationContext, CallRoutes, CallSeed, CallSlotRef, Context, ContextOwner,
    RuntimeServices,
};
use crate::{Bytecode, Function, RuntimeError, Value};
use std::rc::Rc;

pub(in crate::bytecode) const PREFIX: usize = 4;
pub(in crate::bytecode) const CONSTRUCT: u32 = 1;
pub(in crate::bytecode) const TARGET: u32 = 2;
const RECEIVER: u32 = 4;
/// The borrowed Context capability; base fills the header's alignment gap.
#[repr(C)]
pub(crate) struct FrameContext {
    pub(crate) owner: ContextOwner,
    pub(crate) routes: CallRoutes,
    pub(in crate::bytecode) base: u32,
}
#[repr(C)]
pub(in crate::bytecode) struct FrameHeader {
    pub(in crate::bytecode) service: FrameContext,
    pub(in crate::bytecode) program: Option<Rc<Bytecode>>,
    pub(in crate::bytecode) cold: Option<Box<super::vm::ColdFrame>>,
    pub(in crate::bytecode) width: u32,
    pub(in crate::bytecode) written: u32,
    pub(in crate::bytecode) pc: u32,
    pub(in crate::bytecode) dst: u32,
    pub(in crate::bytecode) argc: u32,
    pub(in crate::bytecode) flags: u32,
}
const _: () = {
    assert!(std::mem::size_of::<FrameHeader>() == 64);
    assert!(std::mem::align_of::<FrameHeader>() == 8);
    assert!(std::mem::size_of::<FrameContext>() == 24);
    assert!(std::mem::offset_of!(FrameContext, base) == 20);
    assert!(std::mem::offset_of!(FrameHeader, program) == 24);
    assert!(std::mem::offset_of!(FrameHeader, width) == 40);
    assert!(std::mem::offset_of!(FrameHeader, flags) == 60);
};
impl FrameHeader {
    #[inline(always)]
    pub(in crate::bytecode) fn new(
        base: usize,
        width: usize,
        dst: u16,
        argc: usize,
        flags: u32,
    ) -> Self {
        Self {
            program: None,
            service: FrameContext {
                owner: ContextOwner::new(),
                routes: CallRoutes::default(),
                base: base as u32,
            },
            cold: None,
            width: width as u32,
            written: 0,
            pc: 0,
            dst: dst.into(),
            argc: argc as u32,
            flags,
        }
    }
}
#[derive(Default)]
pub(crate) struct FrameStorage {
    pub(in crate::bytecode) registers: Vec<Value>,
    pub(in crate::bytecode) frames: Vec<FrameHeader>,
    pub(in crate::bytecode) poison: bool,
}
impl FrameStorage {
    pub(crate) fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.registers.capacity() * std::mem::size_of::<Value>()
            + self.frames.capacity() * std::mem::size_of::<FrameHeader>()
    }
    #[inline]
    pub(in crate::bytecode) fn reserve(&mut self, needed: usize) -> Result<(), RuntimeError> {
        if needed > u32::MAX as usize || self.frames.len() >= 200_000 {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: Maximum call stack size exceeded".into(),
            });
        }
        if self.registers.len() < needed {
            self.registers.resize(needed, Value::Undefined);
        }
        Ok(())
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::bytecode) fn push(
        &mut self,
        callee: Value,
        receiver: Value,
        target: Option<Value>,
        width: usize,
        argc: usize,
        dst: u16,
        mut flags: u32,
    ) -> Result<(), RuntimeError> {
        let start = self
            .frames
            .last()
            .map_or(0, |f| f.service.base as usize + f.width as usize);
        let base = start.checked_add(PREFIX).ok_or_else(stack_limit)?;
        self.reserve(base.checked_add(width).ok_or_else(stack_limit)?)?;
        if cfg!(debug_assertions) && self.poison {
            for value in &mut self.registers[start..base + width] {
                assert!(
                    matches!(value, Value::Undefined) || poisoned(value),
                    "released register poison changed"
                );
                *value = Value::Undefined;
            }
        }
        super::compact_fn::execute::store(&mut self.registers[start], callee);
        if matches!(receiver, Value::Undefined) {
            std::mem::forget(receiver);
        } else {
            flags |= RECEIVER;
            super::compact_fn::execute::store(&mut self.registers[start + 1], receiver);
        }
        let flags = if let Some(target) = target {
            super::compact_fn::execute::store(&mut self.registers[start + 2], target);
            flags | TARGET
        } else {
            flags
        };
        self.frames
            .push(FrameHeader::new(base, width, dst, argc, flags));
        let routes = &self.frames.last().expect("pushed frame").service.routes;
        routes.receiver.set(CallSlotRef::PREFIX);
        routes.target.set(if flags & TARGET != 0 {
            CallSlotRef::PREFIX
        } else {
            CallSlotRef::ABSENT
        });
        routes.home.set(CallSlotRef::CALLEE);
        Ok(())
    }
    pub(in crate::bytecode) fn unwind(&mut self) {
        // On abrupt exit any opcode may have staged an owning temporary.
        let end = self
            .frames
            .last()
            .map_or(0, |f| f.service.base as usize + f.width as usize);
        for value in &mut self.registers[..end] {
            crate::value::teardown::clear_value(value);
        }
        self.frames.clear();
    }
}
#[cold]
fn stack_limit() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "RangeError: Maximum call stack size exceeded".into(),
    }
}
fn poison_value() -> Value {
    Value::Number(f64::from_bits(0x7ff8_0000_cafe_f00d))
}
fn poisoned(value: &Value) -> bool {
    matches!(value, Value::Number(n) if n.to_bits() == 0x7ff8_0000_cafe_f00d)
}
#[derive(Default)]
pub(crate) struct StackPool {
    stacks: Vec<FrameStorage>,
    bytes: usize,
}
impl StackPool {
    pub(crate) fn take(&mut self, poison: bool) -> FrameStorage {
        let mut storage = self
            .stacks
            .pop()
            .inspect(|s| {
                self.bytes -= s.retained_bytes();
            })
            .unwrap_or_default();
        if cfg!(debug_assertions) && storage.poison {
            assert!(storage.frames.is_empty());
            for value in &mut storage.registers {
                assert!(poisoned(value), "released stack poison changed");
                *value = Value::Undefined;
            }
        }
        storage.poison = cfg!(debug_assertions) && poison;
        storage
    }
    pub(crate) fn put(&mut self, mut storage: FrameStorage) {
        storage.unwind();
        if cfg!(debug_assertions) && storage.poison {
            for value in &mut storage.registers {
                *value = poison_value();
            }
        }
        let bytes = storage.retained_bytes();
        if bytes <= 16 * 1024
            && self.bytes + bytes + 64 * std::mem::size_of::<FrameStorage>() <= 256 * 1024
            && self.stacks.len() < 64
        {
            self.bytes += bytes;
            self.stacks.push(storage);
        }
    }
}

pub(in crate::bytecode) enum RootActivation<'a> {
    Full(&'a mut ActivationContext),
    Indexed { function: &'a Function },
}
impl RootActivation<'_> {
    pub(in crate::bytecode) fn view<'a>(
        &'a mut self,
        runtime: &'a mut RuntimeServices,
        header: &'a mut FrameHeader,
        prefix: &'a [Value],
        root: bool,
    ) -> Context<'a> {
        if root && let Self::Full(activation) = self {
            return Context {
                runtime,
                activation: ActivationBorrow::Full(activation),
                native: None,
            };
        }
        let function = if root {
            match self {
                Self::Indexed { function, .. } => *function,
                _ => unreachable!("indexed root"),
            }
        } else {
            match &prefix[0] {
                Value::Function(f) => f,
                _ => unreachable!("admitted callee"),
            }
        };
        Context {
            runtime,
            activation: ActivationBorrow::Indexed {
                state: &mut header.service,
                seed: CallSeed {
                    function,
                    prefix: prefix.try_into().expect("four call-prefix slots"),
                },
            },
            native: None,
        }
    }
}

/// Compiler-declared destination ranges; input parameters and TDZ markers count
/// as writes, while received-cell holes do not. No marking runs per opcode.
pub(in crate::bytecode) fn written_ranges(marks: Vec<bool>) -> Box<[std::ops::Range<u16>]> {
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < marks.len() {
        if !marks[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < marks.len() && marks[i] {
            i += 1;
        }
        ranges.push(start as u16..i as u16);
    }
    ranges.into_boxed_slice()
}

#[inline(always)]
pub(in crate::bytecode) fn clear_written(
    window: &mut [Value],
    ranges: &[std::ops::Range<u16>],
    poison: bool,
) {
    for range in ranges {
        for slot in &mut window[range.start as usize..range.end as usize] {
            crate::value::teardown::clear_value(slot);
        }
    }
    if cfg!(debug_assertions) && poison {
        for slot in window {
            assert!(
                matches!(slot, Value::Undefined),
                "unrecorded register write"
            );
            *slot = poison_value();
        }
    }
}
impl FrameStorage {
    #[inline(always)]
    pub(in crate::bytecode) fn pop(&mut self) -> u32 {
        let frame = self.frames.pop().expect("current indexed frame");
        let base = frame.service.base as usize;
        crate::value::teardown::clear_value(&mut self.registers[base - PREFIX]);
        if frame.flags & RECEIVER != 0 {
            crate::value::teardown::clear_value(&mut self.registers[base - PREFIX + 1]);
        }
        if frame.flags & TARGET != 0 {
            crate::value::teardown::clear_value(&mut self.registers[base - PREFIX + 2]);
        }
        if cfg!(debug_assertions) && self.poison {
            for slot in &mut self.registers[base - PREFIX..base] {
                assert!(matches!(slot, Value::Undefined), "unrecorded prefix write");
                *slot = poison_value();
            }
        }
        frame.dst
    }
    #[inline(always)]
    pub(in crate::bytecode) fn move_arguments(
        &mut self,
        callee: &Value,
        source: usize,
        argc: usize,
    ) {
        let Some(body) = (match callee {
            Value::Function(f) => f.bytecode.as_ref(),
            _ => None,
        }) else {
            return;
        };
        let frame = self.frames.last().expect("new indexed callee");
        let (callers, tail) = self.registers.split_at_mut(frame.service.base as usize);
        for index in 0..argc {
            let value = std::mem::replace(&mut callers[source + index], Value::Undefined);
            if let Some(&slot) = body.parameter_slots().get(index)
                && slot < frame.width as usize
            {
                super::compact_fn::execute::store(&mut tail[slot], value);
            } else {
                discard(value);
            }
        }
    }
    #[inline(always)]
    pub(in crate::bytecode) fn install_callee(&mut self, callee: Value) {
        let frame = self.frames.last().expect("entered frame");
        let lexical = matches!(&callee, Value::Function(f) if f.lexical_this);
        if lexical {
            frame.service.routes.receiver.set(CallSlotRef::LEXICAL);
            frame.service.routes.target.set(CallSlotRef::LEXICAL);
            frame.service.routes.home.set(CallSlotRef::LEXICAL);
        }
        super::compact_fn::execute::store(
            &mut self.registers[frame.service.base as usize - PREFIX],
            callee,
        );
    }
}
#[inline(always)]
pub(in crate::bytecode) fn discard(value: Value) {
    if matches!(
        value,
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined
    ) {
        std::mem::forget(value);
    }
}

/// Receiver coercion needs realm metadata, not an eager activation bind.
#[inline(always)]
pub(in crate::bytecode) fn receiver(
    function: &Function,
    value: Option<Value>,
    uses_this: bool,
    runtime: &mut RuntimeServices,
) -> Value {
    if !uses_this || function.lexical_this {
        if let Some(v) = value {
            discard(v);
        }
        return Value::Undefined;
    }
    match value {
        Some(value @ (Value::Array(_) | Value::Function(_))) => value,
        Some(Value::Object(object)) if !crate::symbol::is_symbol_primitive(&object) => {
            Value::Object(object)
        }
        value => coerce_receiver(function, value, runtime),
    }
}
#[cold]
#[inline(never)]
fn coerce_receiver(
    function: &Function,
    value: Option<Value>,
    runtime: &mut RuntimeServices,
) -> Value {
    let mut state = FrameContext {
        owner: ContextOwner::new(),
        routes: CallRoutes::default(),
        base: 0,
    };
    let prefix = std::array::from_fn(|_| Value::Undefined);
    let view = Context {
        runtime,
        activation: ActivationBorrow::Indexed {
            state: &mut state,
            seed: CallSeed {
                function,
                prefix: &prefix,
            },
        },
        native: None,
    };
    crate::function::function_call_this(value, &view, function.is_strict)
}

#[cfg(test)]
impl StackPool {
    pub(in crate::bytecode) fn is_cleared(&self) -> bool {
        self.stacks.iter().all(|s| {
            s.frames.is_empty()
                && s.registers
                    .iter()
                    .all(|v| matches!(v, Value::Undefined) || s.poison && poisoned(v))
        })
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    fn function() -> Function {
        let Value::Function(f) = crate::eval("(function(){'use strict';return 1;})").unwrap()
        else {
            panic!("function")
        };
        f
    }
    #[test]
    fn only_never_exposed_activations_are_released() {
        // Semantic owners are never rebound. Clearing reusable storage releases
        // its owner while retained views keep the old receiver and identity.
        let f = function();
        let mut storage = FrameStorage::default();
        storage
            .push(
                Value::Function(f.clone()),
                Value::Number(1.0),
                None,
                4,
                0,
                0,
                0,
            )
            .unwrap();
        let (mut runtime, _) =
            crate::context::HostRoot::new(f.creation_realm().clone()).into_parts();
        let mut source = RootActivation::Indexed { function: &f };
        let view = {
            let h = storage.frames.last_mut().unwrap();
            let base = h.service.base as usize;
            let mut context = source.view(
                &mut runtime,
                h,
                &storage.registers[base - PREFIX..base],
                true,
            );
            context.activation.retain_continuation()
        };
        let identity = view.variable_identity();
        clear_written(
            &mut storage.registers[PREFIX..PREFIX + 4],
            std::slice::from_ref(&(0..4)),
            false,
        );
        storage.pop();
        storage
            .push(
                Value::Function(f.clone()),
                Value::Undefined,
                None,
                4,
                0,
                0,
                0,
            )
            .unwrap();
        let h = storage.frames.last_mut().unwrap();
        let base = h.service.base as usize;
        let context = source.view(
            &mut runtime,
            h,
            &storage.registers[base - PREFIX..base],
            true,
        );
        assert!(context.activation.strict());
        assert_eq!(context.activation.this_value(), Ok(Value::Undefined));
        assert_ne!(context.caller().variable_identity(), identity);
        assert_eq!(view.this_value(), Ok(Value::Number(1.0)));
    }
    fn poisoned_pool() -> StackPool {
        let mut storage = FrameStorage {
            poison: true,
            ..FrameStorage::default()
        };
        storage
            .push(Value::Undefined, Value::Number(1.0), None, 4, 0, 0, 0)
            .unwrap();
        storage.registers[PREFIX] = Value::Number(42.0);
        clear_written(
            &mut storage.registers[PREFIX..PREFIX + 4],
            std::slice::from_ref(&(0..4)),
            true,
        );
        storage.pop();
        let mut pool = StackPool::default();
        pool.put(storage);
        pool
    }
    #[test]
    fn poisoned_releases_are_checked_and_cleared_on_rebind() {
        let mut pool = poisoned_pool();
        let storage = pool.take(true);
        assert!(storage.frames.is_empty());
        assert!(
            storage
                .registers
                .iter()
                .all(|v| matches!(v, Value::Undefined))
        );
    }
    #[test]
    #[should_panic(expected = "released stack poison changed")]
    fn a_write_to_a_poisoned_release_is_caught_on_rebind() {
        let mut pool = poisoned_pool();
        pool.stacks.last_mut().unwrap().registers[0] = Value::Number(2.0);
        pool.take(true);
    }
    #[test]
    fn retained_stack_count_and_bytes_include_header_capacity() {
        let mut pool = StackPool::default();
        for _ in 0..100 {
            let mut storage = FrameStorage::default();
            storage.reserve(512).unwrap();
            storage.frames.reserve(4);
            pool.put(storage);
        }
        assert!(pool.stacks.len() <= 64);
        assert!(
            pool.bytes + pool.stacks.capacity() * std::mem::size_of::<FrameStorage>() <= 256 * 1024
        );
        assert!(pool.stacks.iter().all(|s| s.retained_bytes() <= 16 * 1024));
    }
}
