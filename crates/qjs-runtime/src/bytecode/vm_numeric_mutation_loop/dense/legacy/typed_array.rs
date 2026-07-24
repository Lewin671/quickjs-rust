//! Fixed Number TypedArray backing leases for local-only dense loops.
//!
//! The lease resolves every view once, rejects observable/resizable storage,
//! and keeps each distinct ordinary ArrayBuffer borrowed for the whole numeric
//! region. The translated body cannot call JavaScript, so cached geometry stays
//! authoritative until the lease is dropped before any header replay.

use std::cell::RefMut;

use crate::bytecode::vm::Vm;
use crate::{
    NativeFunction, ObjectRef, Value, array_buffer, to_int32_number, to_uint32_number, typed_array,
};

use super::super::{
    DenseNumericMutationLoopRun, DynamicProgramRun, array_index_from_number,
    descending_counter_is_valid, record_iteration, record_typed_array_dense_attempt,
    record_typed_array_dense_path_hit, record_typed_array_dense_suppression,
    record_typed_constant_prefix_loads, record_typed_executor_steps,
    record_typed_local_prefix_loads, record_typed_logical_operations, record_writable_path_hit,
    set_local_number,
};
use super::{
    ArraySource, INLINE_DENSE_OPS, LegacyDynamicDensePlan, LocalControl, LocalLimit,
    MAX_DENSE_LOCALS, local_number,
};

mod bundle_executor;
mod program;

use program::{TypedInstruction, TypedProgram};

#[cfg(test)]
use super::super::record_read_only_iteration;

#[derive(Clone, Copy)]
enum NumberViewKind {
    Uint8,
    Int8,
    Uint8Clamped,
    Uint16,
    Int16,
    Uint32,
    Int32,
    Float32,
    Float64,
}

impl NumberViewKind {
    fn from_native(kind: NativeFunction) -> Option<Self> {
        Some(match kind {
            NativeFunction::Uint8Array => Self::Uint8,
            NativeFunction::Int8Array => Self::Int8,
            NativeFunction::Uint8ClampedArray => Self::Uint8Clamped,
            NativeFunction::Uint16Array => Self::Uint16,
            NativeFunction::Int16Array => Self::Int16,
            NativeFunction::Uint32Array => Self::Uint32,
            NativeFunction::Int32Array => Self::Int32,
            NativeFunction::Float32Array => Self::Float32,
            NativeFunction::Float64Array => Self::Float64,
            _ => return None,
        })
    }

    const fn element_shift(self) -> u32 {
        match self {
            Self::Uint8 | Self::Int8 | Self::Uint8Clamped => 0,
            Self::Uint16 | Self::Int16 => 1,
            Self::Uint32 | Self::Int32 | Self::Float32 => 2,
            Self::Float64 => 3,
        }
    }

    const fn element_size(self) -> usize {
        1 << self.element_shift()
    }
}

#[derive(Clone, Copy)]
struct ViewGeometry {
    kind: NumberViewKind,
    byte_offset: usize,
    length: usize,
}

#[derive(Clone, Copy)]
enum EncodedNumber {
    Byte(u8),
    Two([u8; 2]),
    Four([u8; 4]),
    Eight([u8; 8]),
}

#[derive(Clone, Copy)]
struct PendingStore {
    receiver: usize,
    index: usize,
    byte_index: usize,
    value: f64,
    encoded: EncodedNumber,
}

struct TypedDenseAccess<'a, 'bytes> {
    views: &'a [ViewGeometry],
    bytes: &'a mut [RefMut<'bytes, Vec<u8>>],
    pending: Vec<PendingStore>,
}

impl TypedDenseAccess<'_, '_> {
    #[inline(always)]
    fn byte_index<const SHIFT: u32>(&self, receiver: usize, index: usize) -> Option<usize> {
        let view = self.views[receiver];
        if index >= view.length {
            return None;
        }
        // Region entry proved `byte_offset + length * element_size` is inside
        // the leased backing. Therefore this arithmetic cannot overflow for an
        // admitted index, and the direct offset is authoritative until leases
        // are released.
        Some(view.byte_offset + (index << SHIFT))
    }

    #[inline(always)]
    fn reset_iteration(&mut self) {
        self.pending.clear();
    }

    #[inline(always)]
    fn forwarded(&self, receiver: usize, index: usize) -> Option<f64> {
        self.pending
            .iter()
            .rev()
            .find(|store| store.receiver == receiver && store.index == index)
            .map(|store| store.value)
    }

    #[inline(always)]
    fn stage<const SHIFT: u32>(
        &mut self,
        receiver: usize,
        index: usize,
        value: f64,
        encoded: EncodedNumber,
    ) -> bool {
        let Some(byte_index) = self.byte_index::<SHIFT>(receiver, index) else {
            return false;
        };
        self.pending.push(PendingStore {
            receiver,
            index,
            byte_index,
            value,
            encoded,
        });
        true
    }

    #[inline(always)]
    fn load_u8(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<0>(receiver, index)?;
        Some(f64::from(self.bytes[receiver][byte_index]))
    }

    #[inline(always)]
    fn load_i8(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<0>(receiver, index)?;
        Some(f64::from(self.bytes[receiver][byte_index] as i8))
    }

    #[inline(always)]
    fn load_u16(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<1>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 2];
        Some(f64::from(u16::from_le_bytes([bytes[0], bytes[1]])))
    }

    #[inline(always)]
    fn load_i16(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<1>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 2];
        Some(f64::from(i16::from_le_bytes([bytes[0], bytes[1]])))
    }

    #[inline(always)]
    fn load_u32(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<2>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 4];
        Some(f64::from(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    #[inline(always)]
    fn load_i32(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<2>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 4];
        Some(f64::from(i32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    #[inline(always)]
    fn load_f32(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<2>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 4];
        Some(f64::from(f32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ])))
    }

    #[inline(always)]
    fn load_f64(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(value) = self.forwarded(receiver, index) {
            return Some(value);
        }
        let byte_index = self.byte_index::<3>(receiver, index)?;
        let bytes = &self.bytes[receiver][byte_index..byte_index + 8];
        Some(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    #[inline(always)]
    fn stage_u8(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let converted = modulo_integer(value, 256.0);
        self.stage::<0>(
            receiver,
            index,
            converted,
            EncodedNumber::Byte(converted as u8),
        )
    }

    #[inline(always)]
    fn stage_i8(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let unsigned = modulo_integer(value, 256.0);
        let converted = if unsigned >= 128.0 {
            unsigned - 256.0
        } else {
            unsigned
        };
        self.stage::<0>(
            receiver,
            index,
            converted,
            EncodedNumber::Byte((converted as i8) as u8),
        )
    }

    #[inline(always)]
    fn stage_u8_clamped(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let converted = clamp_uint8(value);
        self.stage::<0>(
            receiver,
            index,
            converted,
            EncodedNumber::Byte(converted as u8),
        )
    }

    #[inline(always)]
    fn stage_u16(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let converted = modulo_integer(value, 65_536.0);
        self.stage::<1>(
            receiver,
            index,
            converted,
            EncodedNumber::Two((converted as u16).to_le_bytes()),
        )
    }

    #[inline(always)]
    fn stage_i16(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let unsigned = modulo_integer(value, 65_536.0);
        let converted = if unsigned >= 32_768.0 {
            unsigned - 65_536.0
        } else {
            unsigned
        };
        self.stage::<1>(
            receiver,
            index,
            converted,
            EncodedNumber::Two((converted as i16).to_le_bytes()),
        )
    }

    #[inline(always)]
    fn stage_u32(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let converted = modulo_integer(value, 4_294_967_296.0);
        self.stage::<2>(
            receiver,
            index,
            converted,
            EncodedNumber::Four((converted as u32).to_le_bytes()),
        )
    }

    #[inline(always)]
    fn stage_i32(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let unsigned = modulo_integer(value, 4_294_967_296.0);
        let converted = if unsigned >= 2_147_483_648.0 {
            unsigned - 4_294_967_296.0
        } else {
            unsigned
        };
        self.stage::<2>(
            receiver,
            index,
            converted,
            EncodedNumber::Four((converted as i32).to_le_bytes()),
        )
    }

    #[inline(always)]
    fn stage_f32(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        let converted = value as f32;
        self.stage::<2>(
            receiver,
            index,
            f64::from(converted),
            EncodedNumber::Four(converted.to_le_bytes()),
        )
    }

    #[inline(always)]
    fn stage_f64(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        self.stage::<3>(
            receiver,
            index,
            value,
            EncodedNumber::Eight(value.to_le_bytes()),
        )
    }

    #[inline(always)]
    fn commit_stores(&mut self) {
        for store in &self.pending {
            let bytes = &mut self.bytes[store.receiver];
            match store.encoded {
                EncodedNumber::Byte(value) => bytes[store.byte_index] = value,
                EncodedNumber::Two(value) => {
                    bytes[store.byte_index..store.byte_index + 2].copy_from_slice(&value);
                }
                EncodedNumber::Four(value) => {
                    bytes[store.byte_index..store.byte_index + 4].copy_from_slice(&value);
                }
                EncodedNumber::Eight(value) => {
                    bytes[store.byte_index..store.byte_index + 8].copy_from_slice(&value);
                }
            }
        }
    }
}

#[inline(always)]
fn modulo_integer(number: f64, modulo: f64) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return 0.0;
    }
    let integer = number.trunc();
    ((integer % modulo) + modulo) % modulo
}

#[inline(always)]
fn clamp_uint8(number: f64) -> f64 {
    if number.is_nan() || number <= 0.0 {
        0.0
    } else if number >= 255.0 {
        255.0
    } else {
        let floor = number.floor();
        let diff = number - floor;
        if diff < 0.5 {
            floor
        } else if diff > 0.5 {
            floor + 1.0
        } else if (floor as i64) % 2 == 0 {
            floor
        } else {
            floor + 1.0
        }
    }
}

macro_rules! stage_typed_store {
    ($access:expr, $registers:expr, $receiver:expr, $index:expr, $value:expr, $method:ident, $made_progress:expr) => {{
        let Some(index_value) = array_index_from_number($registers[$index]) else {
            return DynamicProgramRun {
                deoptimized: true,
                made_progress: $made_progress,
            };
        };
        let raw = $registers[$value];
        if !$access.$method($receiver, index_value, raw) {
            return DynamicProgramRun {
                deoptimized: true,
                made_progress: $made_progress,
            };
        }
        raw
    }};
}

impl TypedProgram {
    #[inline(always)]
    fn should_continue<const AT_LEAST_ZERO: bool>(counter: f64, limit: f64) -> bool {
        if AT_LEAST_ZERO {
            counter >= 0.0
        } else {
            matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less))
        }
    }

    #[inline(always)]
    fn initialize_constants(&self, registers: &mut [f64]) {
        for (register, operation) in self.operations[..self.constant_count].iter().enumerate() {
            let TypedInstruction::Constant(value) = *operation else {
                unreachable!("typed constant prefix was validated while lowering")
            };
            registers[register] = value;
        }
        record_typed_constant_prefix_loads(self.constant_count);
    }

    #[inline(always)]
    fn copy_entry_locals(&self, locals: &[f64; MAX_DENSE_LOCALS], registers: &mut [f64]) {
        for (offset, operation) in self.operations[self.constant_count..self.dynamic_start]
            .iter()
            .enumerate()
        {
            let TypedInstruction::LoadLocal(local) = *operation else {
                unreachable!("typed local prefix was validated while lowering")
            };
            registers[self.constant_count + offset] = locals[local];
        }
        record_typed_local_prefix_loads(self.dynamic_start - self.constant_count);
    }

    #[inline(always)]
    fn stage_store(
        access: &mut TypedDenseAccess<'_, '_>,
        kind: NumberViewKind,
        receiver: usize,
        index: usize,
        value: f64,
    ) -> bool {
        match kind {
            NumberViewKind::Uint8 => access.stage_u8(receiver, index, value),
            NumberViewKind::Int8 => access.stage_i8(receiver, index, value),
            NumberViewKind::Uint8Clamped => access.stage_u8_clamped(receiver, index, value),
            NumberViewKind::Uint16 => access.stage_u16(receiver, index, value),
            NumberViewKind::Int16 => access.stage_i16(receiver, index, value),
            NumberViewKind::Uint32 => access.stage_u32(receiver, index, value),
            NumberViewKind::Int32 => access.stage_i32(receiver, index, value),
            NumberViewKind::Float32 => access.stage_f32(receiver, index, value),
            NumberViewKind::Float64 => access.stage_f64(receiver, index, value),
        }
    }

    fn run<const AT_LEAST_ZERO: bool>(
        &self,
        plan: &LegacyDynamicDensePlan,
        access: &mut TypedDenseAccess<'_, '_>,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
        limit: f64,
    ) -> DynamicProgramRun {
        if !self.binary_bundles.is_empty() {
            return self.run_bundled::<AT_LEAST_ZERO>(plan, access, locals, registers, limit);
        }
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
            for (offset, operation) in self.operations[self.dynamic_start..].iter().enumerate() {
                record_typed_logical_operations(1);
                record_typed_executor_steps(1);
                let register = self.dynamic_start + offset;
                let value = match *operation {
                    TypedInstruction::Constant(value) => value,
                    TypedInstruction::LoadLocal(local) => locals[local],
                    TypedInstruction::LoadU8 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_u8(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadI8 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_i8(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadU16 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_u16(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadI16 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_i16(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadU32 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_u32(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadI32 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_i32(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadF32 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_f32(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::LoadF64 { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_f64(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    TypedInstruction::StoreU8 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_u8,
                        made_progress
                    ),
                    TypedInstruction::StoreI8 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_i8,
                        made_progress
                    ),
                    TypedInstruction::StoreU8Clamped {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_u8_clamped,
                        made_progress
                    ),
                    TypedInstruction::StoreU16 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_u16,
                        made_progress
                    ),
                    TypedInstruction::StoreI16 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_i16,
                        made_progress
                    ),
                    TypedInstruction::StoreU32 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_u32,
                        made_progress
                    ),
                    TypedInstruction::StoreI32 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_i32,
                        made_progress
                    ),
                    TypedInstruction::StoreF32 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_f32,
                        made_progress
                    ),
                    TypedInstruction::StoreF64 {
                        receiver,
                        index,
                        value,
                    } => stage_typed_store!(
                        access,
                        registers,
                        receiver,
                        index,
                        value,
                        stage_f64,
                        made_progress
                    ),
                    TypedInstruction::Add { left, right } => registers[left] + registers[right],
                    TypedInstruction::Sub { left, right } => registers[left] - registers[right],
                    TypedInstruction::Mul { left, right } => registers[left] * registers[right],
                    TypedInstruction::Div { left, right } => registers[left] / registers[right],
                    TypedInstruction::Rem { left, right } => registers[left] % registers[right],
                    TypedInstruction::Shl { left, right } => f64::from(
                        to_int32_number(registers[left])
                            << (to_uint32_number(registers[right]) & 0x1f),
                    ),
                    TypedInstruction::Shr { left, right } => f64::from(
                        to_int32_number(registers[left])
                            >> (to_uint32_number(registers[right]) & 0x1f),
                    ),
                    TypedInstruction::UShr { left, right } => f64::from(
                        to_uint32_number(registers[left])
                            >> (to_uint32_number(registers[right]) & 0x1f),
                    ),
                    TypedInstruction::BitwiseAnd { left, right } => f64::from(
                        to_int32_number(registers[left]) & to_int32_number(registers[right]),
                    ),
                    TypedInstruction::BitwiseXor { left, right } => f64::from(
                        to_int32_number(registers[left]) ^ to_int32_number(registers[right]),
                    ),
                    TypedInstruction::BitwiseOr { left, right } => f64::from(
                        to_int32_number(registers[left]) | to_int32_number(registers[right]),
                    ),
                    TypedInstruction::Plus { value } => registers[value],
                    TypedInstruction::Minus { value } => -registers[value],
                    TypedInstruction::BitwiseNot { value } => {
                        f64::from(!to_int32_number(registers[value]))
                    }
                    TypedInstruction::Increment { value } => registers[value] + 1.0,
                    TypedInstruction::Decrement { value } => registers[value] - 1.0,
                };
                registers[register] = value;
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

fn receiver_objects(
    plan: &LegacyDynamicDensePlan,
    vm: &Vm<'_>,
) -> Result<Option<Vec<ObjectRef>>, ()> {
    let mut objects = Vec::with_capacity(plan.receiver_sources.len());
    let mut saw_typed_array = false;
    let mut saw_other = false;
    for source in &plan.receiver_sources {
        let value = match source {
            ArraySource::Local(slot) => vm.locals.get(*slot).and_then(Option::as_ref),
            ArraySource::DirectThisOwnData(_) => None,
        };
        match value {
            Some(Value::Object(object)) if typed_array::is_typed_array_object(object) => {
                saw_typed_array = true;
                objects.push(object.clone());
            }
            _ => saw_other = true,
        }
    }
    match (saw_typed_array, saw_other) {
        (false, _) => Ok(None),
        (true, false) => Ok(Some(objects)),
        (true, true) => Err(()),
    }
}

fn typed_limit(
    plan: &LegacyDynamicDensePlan,
    locals: &[f64; MAX_DENSE_LOCALS],
) -> Result<Option<f64>, ()> {
    match plan.control {
        LocalControl::LessThan(LocalLimit::Number(local)) => Ok(Some(locals[local])),
        LocalControl::AtLeastZero => Ok(None),
        // `GetPropNamed("length")` remains observable: an instance own
        // property or a replaced %TypedArray%.prototype accessor must run.
        // Admit this control only after compilation records and validates the
        // native accessor identity.
        LocalControl::LessThan(LocalLimit::ArrayLength(_)) | LocalControl::Countdown => Err(()),
    }
}

/// Attempts the TypedArray specialization. `None` means no receiver is a
/// TypedArray and the ordinary dense-Array executor retains priority. Once any
/// TypedArray participates, every structural rejection suppresses this plan for
/// the current invocation so the backedge does not repeat failed leases.
pub(super) fn try_run(
    plan: &LegacyDynamicDensePlan,
    vm: &mut Vm<'_>,
    exit: usize,
) -> Option<DenseNumericMutationLoopRun> {
    record_typed_array_dense_attempt();
    let objects = match receiver_objects(plan, vm) {
        Ok(None) => return None,
        Ok(Some(objects)) => objects,
        Err(()) => {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        }
    };
    if !matches!(
        plan.control,
        LocalControl::LessThan(LocalLimit::Number(_)) | LocalControl::AtLeastZero
    ) {
        record_typed_array_dense_suppression();
        return Some(DenseNumericMutationLoopRun::Suppress);
    }

    let mut views = Vec::with_capacity(objects.len());
    let mut buffers = Vec::with_capacity(objects.len());
    for object in &objects {
        let Some(view) = typed_array::fixed_number_typed_array_view(object) else {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        };
        let Some(kind) = NumberViewKind::from_native(view.kind) else {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        };
        if buffers
            .iter()
            .any(|buffer: &ObjectRef| buffer.ptr_eq(&view.buffer))
        {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        }
        views.push(ViewGeometry {
            kind,
            byte_offset: view.byte_offset,
            length: view.length,
        });
        buffers.push(view.buffer);
    }
    let Some(program) = TypedProgram::lower(plan, &views) else {
        record_typed_array_dense_suppression();
        return Some(DenseNumericMutationLoopRun::Suppress);
    };

    let mut locals = [0.0; MAX_DENSE_LOCALS];
    for (local, slot) in plan.local_slots.iter().enumerate() {
        let Some(value) = local_number(vm, *slot) else {
            return Some(DenseNumericMutationLoopRun::Declined);
        };
        locals[local] = value;
    }
    if matches!(plan.control, LocalControl::AtLeastZero) {
        let counter = locals[plan.counter_local];
        if !descending_counter_is_valid(counter) {
            return Some(DenseNumericMutationLoopRun::Declined);
        }
    }
    let limit = match typed_limit(plan, &locals) {
        Ok(limit) => limit,
        Err(()) => {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        }
    };

    let mut leased_bytes = Vec::with_capacity(buffers.len());
    for buffer in &buffers {
        let Some(bytes) = array_buffer::try_borrow_fixed_array_buffer_bytes_mut(buffer) else {
            record_typed_array_dense_suppression();
            return Some(DenseNumericMutationLoopRun::Suppress);
        };
        leased_bytes.push(bytes);
    }
    if views.iter().zip(&leased_bytes).any(|(view, bytes)| {
        view.length
            .checked_mul(view.kind.element_size())
            .and_then(|byte_length| view.byte_offset.checked_add(byte_length))
            .is_none_or(|end| end > bytes.len())
    }) {
        record_typed_array_dense_suppression();
        return Some(DenseNumericMutationLoopRun::Suppress);
    }

    let mut inline_registers = [0.0; INLINE_DENSE_OPS];
    let mut large_registers =
        (plan.operations.len() > INLINE_DENSE_OPS).then(|| vec![0.0; plan.operations.len()]);
    let registers = match large_registers.as_mut() {
        Some(registers) => registers.as_mut_slice(),
        None => &mut inline_registers[..plan.operations.len()],
    };
    let run = {
        let mut access = TypedDenseAccess {
            views: &views,
            bytes: &mut leased_bytes,
            pending: Vec::with_capacity(plan.store_count),
        };
        match limit {
            Some(limit) => program.run::<false>(plan, &mut access, &mut locals, registers, limit),
            None => program.run::<true>(plan, &mut access, &mut locals, registers, 0.0),
        }
    };

    // Header replay must run after every backing lease is gone.
    drop(leased_bytes);
    drop(buffers);

    if run.deoptimized && !run.made_progress {
        return Some(DenseNumericMutationLoopRun::Declined);
    }
    if run.made_progress {
        record_typed_array_dense_path_hit();
        if plan.store_count != 0 {
            record_writable_path_hit();
        }
        for (slot, value) in plan.local_slots.iter().copied().zip(locals) {
            set_local_number(vm, slot, value);
        }
    }
    vm.ip = if run.deoptimized {
        plan.header
    } else {
        exit + 1
    };
    Some(DenseNumericMutationLoopRun::Handled)
}
