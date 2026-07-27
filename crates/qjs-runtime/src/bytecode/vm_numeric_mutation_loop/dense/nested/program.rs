//! Predecoded numeric instructions for nested dense regions.
//!
//! The nested executor has already proved that every operation is a pure
//! Number operation before it takes an Array lease. Lowering the shared
//! `NumberInstruction` stream here removes the second dynamic dispatch on
//! `BinaryOp` or `UnaryOp` from each hot inner iteration.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::super::{
    DenseAccess, LocalWrite, MAX_DENSE_OPS, NumberInstruction, Register, array_index_from_number,
};
use super::LocalBank;

const MAX_INDEX_CACHES: usize = 8;

#[derive(Clone, Copy, Debug)]
pub(super) enum NestedInstruction {
    Constant(f64),
    LoadLocal(usize),
    DenseLoad {
        receiver: usize,
        index: Register,
        index_cache: Option<u8>,
    },
    DenseStore {
        receiver: usize,
        index: Register,
        value: Register,
        index_cache: Option<u8>,
    },
    StoreAdd {
        receiver: usize,
        index: Register,
        left: Register,
        right: Register,
        index_cache: Option<u8>,
    },
    StoreSub {
        receiver: usize,
        index: Register,
        left: Register,
        right: Register,
        index_cache: Option<u8>,
    },
    /// A dense read whose only consumer is the following arithmetic store.
    ///
    /// The source read must still run before the target index conversion and
    /// staged write, matching the two original instructions exactly.
    StoreAddFromLoad {
        receiver: usize,
        index: Register,
        index_cache: Option<u8>,
        source_receiver: usize,
        source_index: Register,
        source_index_cache: Option<u8>,
        other: Register,
    },
    StoreSubFromLoad {
        receiver: usize,
        index: Register,
        index_cache: Option<u8>,
        source_receiver: usize,
        source_index: Register,
        source_index_cache: Option<u8>,
        other: Register,
        load_on_left: bool,
    },
    /// Two independently rounded products followed by addition.
    MulAdd {
        left_left: Register,
        left_right: Register,
        right_left: Register,
        right_right: Register,
    },
    /// Two independently rounded products followed by subtraction.
    MulSub {
        left_left: Register,
        left_right: Register,
        right_left: Register,
        right_right: Register,
    },
    Add {
        left: Register,
        right: Register,
    },
    Sub {
        left: Register,
        right: Register,
    },
    Mul {
        left: Register,
        right: Register,
    },
    Div {
        left: Register,
        right: Register,
    },
    Rem {
        left: Register,
        right: Register,
    },
    Shl {
        left: Register,
        right: Register,
    },
    Shr {
        left: Register,
        right: Register,
    },
    UShr {
        left: Register,
        right: Register,
    },
    BitwiseAnd {
        left: Register,
        right: Register,
    },
    BitwiseXor {
        left: Register,
        right: Register,
    },
    BitwiseOr {
        left: Register,
        right: Register,
    },
    Plus {
        value: Register,
    },
    Minus {
        value: Register,
    },
    BitwiseNot {
        value: Register,
    },
    Increment {
        value: Register,
    },
    Decrement {
        value: Register,
    },
}

impl NestedInstruction {
    pub(super) fn lower(operation: NumberInstruction) -> Option<Self> {
        Some(match operation {
            NumberInstruction::Constant(value) => Self::Constant(value),
            NumberInstruction::LoadLocal(local) => Self::LoadLocal(local),
            NumberInstruction::DenseLoad { receiver, index } => Self::DenseLoad {
                receiver,
                index,
                index_cache: None,
            },
            NumberInstruction::DenseStore {
                receiver,
                index,
                value,
            } => Self::DenseStore {
                receiver,
                index,
                value,
                index_cache: None,
            },
            NumberInstruction::Binary {
                operation,
                left,
                right,
            } => match operation {
                BinaryOp::Add => Self::Add { left, right },
                BinaryOp::Sub => Self::Sub { left, right },
                BinaryOp::Mul => Self::Mul { left, right },
                BinaryOp::Div => Self::Div { left, right },
                BinaryOp::Rem => Self::Rem { left, right },
                BinaryOp::Shl => Self::Shl { left, right },
                BinaryOp::Shr => Self::Shr { left, right },
                BinaryOp::UShr => Self::UShr { left, right },
                BinaryOp::BitwiseAnd => Self::BitwiseAnd { left, right },
                BinaryOp::BitwiseXor => Self::BitwiseXor { left, right },
                BinaryOp::BitwiseOr => Self::BitwiseOr { left, right },
                _ => return None,
            },
            NumberInstruction::Unary { operation, value } => match operation {
                UnaryOp::Plus => Self::Plus { value },
                UnaryOp::Minus => Self::Minus { value },
                UnaryOp::BitwiseNot => Self::BitwiseNot { value },
                _ => return None,
            },
            NumberInstruction::Update { operation, value } => match operation {
                UpdateOp::Increment => Self::Increment { value },
                UpdateOp::Decrement => Self::Decrement { value },
            },
            NumberInstruction::LoadInvariant(_) | NumberInstruction::MathRound { .. } => {
                return None;
            }
        })
    }
}

/// Fuses a pure terminal addition or subtraction into its immediately following
/// dense store. The store result is the same Number as the binary result, so
/// both old SSA registers can safely remap to the fused instruction. The
/// transformation deliberately declines an index that depends on that binary
/// result: that shape needs the value before the store itself can resolve its
/// index.
pub(super) fn fuse_terminal_add_sub_stores(
    operations: &mut Vec<NestedInstruction>,
    writes: &mut [LocalWrite],
    counter_write: &mut Register,
) -> bool {
    const UNMAPPED: Register = usize::MAX;

    let operation_count = operations.len();
    if operation_count > MAX_DENSE_OPS
        || operations
            .iter()
            .enumerate()
            .any(|(destination, operation)| {
                !operation_references_are_valid(*operation, destination)
            })
        || writes.iter().any(|write| write.value >= operation_count)
        || *counter_write >= operation_count
    {
        return false;
    }

    let original = operations.clone();
    let mut remap = [UNMAPPED; MAX_DENSE_OPS];
    let mut rewritten = Vec::with_capacity(operation_count);
    let mut cursor = 0;
    let mut fused = false;
    while cursor < original.len() {
        if let (Some((is_add, left, right)), Some(store)) = (
            add_or_sub(original[cursor]),
            original.get(cursor + 1).copied(),
        ) && let NestedInstruction::DenseStore {
            receiver,
            index,
            value,
            index_cache,
        } = store
            && value == cursor
            && index != cursor
        {
            let (Some(left), Some(right), Some(index)) = (
                remapped_register(left, &remap),
                remapped_register(right, &remap),
                remapped_register(index, &remap),
            ) else {
                return false;
            };
            let register = rewritten.len();
            remap[cursor] = register;
            remap[cursor + 1] = register;
            rewritten.push(if is_add {
                NestedInstruction::StoreAdd {
                    receiver,
                    index,
                    left,
                    right,
                    index_cache,
                }
            } else {
                NestedInstruction::StoreSub {
                    receiver,
                    index,
                    left,
                    right,
                    index_cache,
                }
            });
            cursor += 2;
            fused = true;
            continue;
        }

        let mut operation = original[cursor];
        if remap_operation_registers(&mut operation, &remap).is_none() {
            return false;
        }
        remap[cursor] = rewritten.len();
        rewritten.push(operation);
        cursor += 1;
    }
    if !fused {
        return false;
    }

    let mut remapped_writes = writes.to_vec();
    for write in &mut remapped_writes {
        let Some(value) = remapped_register(write.value, &remap) else {
            return false;
        };
        write.value = value;
    }
    let Some(remapped_counter_write) = remapped_register(*counter_write, &remap) else {
        return false;
    };

    *operations = rewritten;
    writes.copy_from_slice(&remapped_writes);
    *counter_write = remapped_counter_write;
    true
}

/// Fuses a unique dense load into an immediately following arithmetic store.
///
/// The dense executor has no observable intermediate state: a failed native
/// iteration replays the original bytecode before any staged stores publish.
/// Consequently, when a load register is consumed only by the adjacent store,
/// the pair can retain its original source-read, arithmetic, target-index, and
/// staged-write order in one instruction. A second consumer, including a
/// local live-out, prevents the rewrite so that its original SSA value remains
/// available.
pub(super) fn fuse_dense_load_stores(
    operations: &mut Vec<NestedInstruction>,
    writes: &mut [LocalWrite],
    counter_write: &mut Register,
) -> bool {
    const UNMAPPED: Register = usize::MAX;

    let operation_count = operations.len();
    let Some(uses) = program_use_counts(operations, writes, *counter_write) else {
        return false;
    };

    let original = operations.clone();
    let mut remap = [UNMAPPED; MAX_DENSE_OPS];
    let mut rewritten = Vec::with_capacity(operation_count);
    let mut cursor = 0;
    let mut fused = false;
    while cursor < original.len() {
        let fused_operation = match (original[cursor], original.get(cursor + 1).copied()) {
            (
                NestedInstruction::DenseLoad {
                    receiver: source_receiver,
                    index: source_index,
                    index_cache: source_index_cache,
                },
                Some(NestedInstruction::StoreAdd {
                    receiver,
                    index,
                    left,
                    right,
                    index_cache,
                }),
            ) if uses[cursor] == 1 && index != cursor => {
                let other = if left == cursor {
                    Some(right)
                } else if right == cursor {
                    Some(left)
                } else {
                    None
                };
                other.map(|other| NestedInstruction::StoreAddFromLoad {
                    receiver,
                    index,
                    index_cache,
                    source_receiver,
                    source_index,
                    source_index_cache,
                    other,
                })
            }
            (
                NestedInstruction::DenseLoad {
                    receiver: source_receiver,
                    index: source_index,
                    index_cache: source_index_cache,
                },
                Some(NestedInstruction::StoreSub {
                    receiver,
                    index,
                    left,
                    right,
                    index_cache,
                }),
            ) if uses[cursor] == 1 && index != cursor => {
                let (other, load_on_left) = if left == cursor {
                    (Some(right), true)
                } else if right == cursor {
                    (Some(left), false)
                } else {
                    (None, false)
                };
                other.map(|other| NestedInstruction::StoreSubFromLoad {
                    receiver,
                    index,
                    index_cache,
                    source_receiver,
                    source_index,
                    source_index_cache,
                    other,
                    load_on_left,
                })
            }
            _ => None,
        };
        if let Some(mut operation) = fused_operation {
            if remap_operation_registers(&mut operation, &remap).is_none() {
                return false;
            }
            let register = rewritten.len();
            remap[cursor] = register;
            remap[cursor + 1] = register;
            rewritten.push(operation);
            cursor += 2;
            fused = true;
            continue;
        }

        let mut operation = original[cursor];
        if remap_operation_registers(&mut operation, &remap).is_none() {
            return false;
        }
        remap[cursor] = rewritten.len();
        rewritten.push(operation);
        cursor += 1;
    }
    if !fused {
        return false;
    }

    let mut remapped_writes = writes.to_vec();
    for write in &mut remapped_writes {
        let Some(value) = remapped_register(write.value, &remap) else {
            return false;
        };
        write.value = value;
    }
    let Some(remapped_counter_write) = remapped_register(*counter_write, &remap) else {
        return false;
    };

    *operations = rewritten;
    writes.copy_from_slice(&remapped_writes);
    *counter_write = remapped_counter_write;
    true
}

/// Fuses `mul; mul; add/sub` dataflow when both intermediate products have a
/// single consumer.
///
/// The lowered nested program already admits only pure Number arithmetic, so
/// the fused instruction computes both products in source order and then
/// performs the same separately rounded addition or subtraction. It is not an
/// FMA: preserving the two intermediate IEEE-754 roundings is required for
/// JavaScript Number semantics. Any additional use of either product keeps the
/// original SSA instructions intact.
pub(super) fn fuse_mul_add_sub(
    operations: &mut Vec<NestedInstruction>,
    writes: &mut [LocalWrite],
    counter_write: &mut Register,
) -> bool {
    const UNMAPPED: Register = usize::MAX;

    let operation_count = operations.len();
    let Some(uses) = program_use_counts(operations, writes, *counter_write) else {
        return false;
    };
    let original = operations.clone();
    let mut removed = [false; MAX_DENSE_OPS];
    let mut replacements = [None; MAX_DENSE_OPS];
    let mut fused = false;
    for (destination, operation) in original.iter().copied().enumerate() {
        let Some((is_add, left, right)) = add_or_sub(operation) else {
            continue;
        };
        if uses[left] != 1 || uses[right] != 1 || left == right || removed[left] || removed[right] {
            continue;
        }
        let (Some((left_left, left_right)), Some((right_left, right_right))) =
            (mul_operands(original[left]), mul_operands(original[right]))
        else {
            continue;
        };
        replacements[destination] = Some(if is_add {
            NestedInstruction::MulAdd {
                left_left,
                left_right,
                right_left,
                right_right,
            }
        } else {
            NestedInstruction::MulSub {
                left_left,
                left_right,
                right_left,
                right_right,
            }
        });
        removed[left] = true;
        removed[right] = true;
        fused = true;
    }
    if !fused {
        return false;
    }

    let mut remap = [UNMAPPED; MAX_DENSE_OPS];
    let mut rewritten = Vec::with_capacity(operation_count);
    for (cursor, operation) in original.iter().copied().enumerate() {
        if removed[cursor] {
            continue;
        }
        let mut operation = replacements[cursor].unwrap_or(operation);
        if remap_operation_registers(&mut operation, &remap).is_none() {
            return false;
        }
        remap[cursor] = rewritten.len();
        rewritten.push(operation);
    }

    let mut remapped_writes = writes.to_vec();
    for write in &mut remapped_writes {
        let Some(value) = remapped_register(write.value, &remap) else {
            return false;
        };
        write.value = value;
    }
    let Some(remapped_counter_write) = remapped_register(*counter_write, &remap) else {
        return false;
    };

    *operations = rewritten;
    writes.copy_from_slice(&remapped_writes);
    *counter_write = remapped_counter_write;
    true
}

fn add_or_sub(operation: NestedInstruction) -> Option<(bool, Register, Register)> {
    match operation {
        NestedInstruction::Add { left, right } => Some((true, left, right)),
        NestedInstruction::Sub { left, right } => Some((false, left, right)),
        _ => None,
    }
}

fn mul_operands(operation: NestedInstruction) -> Option<(Register, Register)> {
    match operation {
        NestedInstruction::Mul { left, right } => Some((left, right)),
        _ => None,
    }
}

fn operation_references_are_valid(operation: NestedInstruction, destination: Register) -> bool {
    operation_registers(operation)
        .into_iter()
        .flatten()
        .all(|register| register < destination)
}

fn operation_registers(operation: NestedInstruction) -> [Option<Register>; 4] {
    match operation {
        NestedInstruction::Constant(_) | NestedInstruction::LoadLocal(_) => {
            [None, None, None, None]
        }
        NestedInstruction::DenseLoad { index, .. } => [Some(index), None, None, None],
        NestedInstruction::DenseStore { index, value, .. } => {
            [Some(index), Some(value), None, None]
        }
        NestedInstruction::StoreAdd {
            index, left, right, ..
        }
        | NestedInstruction::StoreSub {
            index, left, right, ..
        } => [Some(index), Some(left), Some(right), None],
        NestedInstruction::StoreAddFromLoad {
            index,
            source_index,
            other,
            ..
        }
        | NestedInstruction::StoreSubFromLoad {
            index,
            source_index,
            other,
            ..
        } => [Some(index), Some(source_index), Some(other), None],
        NestedInstruction::MulAdd {
            left_left,
            left_right,
            right_left,
            right_right,
        }
        | NestedInstruction::MulSub {
            left_left,
            left_right,
            right_left,
            right_right,
        } => [
            Some(left_left),
            Some(left_right),
            Some(right_left),
            Some(right_right),
        ],
        NestedInstruction::Add { left, right }
        | NestedInstruction::Sub { left, right }
        | NestedInstruction::Mul { left, right }
        | NestedInstruction::Div { left, right }
        | NestedInstruction::Rem { left, right }
        | NestedInstruction::Shl { left, right }
        | NestedInstruction::Shr { left, right }
        | NestedInstruction::UShr { left, right }
        | NestedInstruction::BitwiseAnd { left, right }
        | NestedInstruction::BitwiseXor { left, right }
        | NestedInstruction::BitwiseOr { left, right } => [Some(left), Some(right), None, None],
        NestedInstruction::Plus { value }
        | NestedInstruction::Minus { value }
        | NestedInstruction::BitwiseNot { value }
        | NestedInstruction::Increment { value }
        | NestedInstruction::Decrement { value } => [Some(value), None, None, None],
    }
}

fn program_use_counts(
    operations: &[NestedInstruction],
    writes: &[LocalWrite],
    counter_write: Register,
) -> Option<[usize; MAX_DENSE_OPS]> {
    let operation_count = operations.len();
    if operation_count > MAX_DENSE_OPS
        || operations
            .iter()
            .enumerate()
            .any(|(destination, operation)| {
                !operation_references_are_valid(*operation, destination)
            })
        || writes.iter().any(|write| write.value >= operation_count)
        || counter_write >= operation_count
    {
        return None;
    }

    let mut uses = [0_usize; MAX_DENSE_OPS];
    for operation in operations.iter().copied() {
        for register in operation_registers(operation).into_iter().flatten() {
            uses[register] += 1;
        }
    }
    for write in writes {
        uses[write.value] += 1;
    }
    uses[counter_write] += 1;
    Some(uses)
}

fn remap_operation_registers(
    operation: &mut NestedInstruction,
    remap: &[Register; MAX_DENSE_OPS],
) -> Option<()> {
    let remap_register = |register: &mut Register| {
        let remapped = remapped_register(*register, remap)?;
        *register = remapped;
        Some(())
    };
    match operation {
        NestedInstruction::Constant(_) | NestedInstruction::LoadLocal(_) => Some(()),
        NestedInstruction::DenseLoad { index, .. } => remap_register(index),
        NestedInstruction::DenseStore { index, value, .. } => {
            remap_register(index)?;
            remap_register(value)
        }
        NestedInstruction::StoreAdd {
            index, left, right, ..
        }
        | NestedInstruction::StoreSub {
            index, left, right, ..
        } => {
            remap_register(index)?;
            remap_register(left)?;
            remap_register(right)
        }
        NestedInstruction::StoreAddFromLoad {
            index,
            source_index,
            other,
            ..
        }
        | NestedInstruction::StoreSubFromLoad {
            index,
            source_index,
            other,
            ..
        } => {
            remap_register(index)?;
            remap_register(source_index)?;
            remap_register(other)
        }
        NestedInstruction::MulAdd {
            left_left,
            left_right,
            right_left,
            right_right,
        }
        | NestedInstruction::MulSub {
            left_left,
            left_right,
            right_left,
            right_right,
        } => {
            remap_register(left_left)?;
            remap_register(left_right)?;
            remap_register(right_left)?;
            remap_register(right_right)
        }
        NestedInstruction::Add { left, right }
        | NestedInstruction::Sub { left, right }
        | NestedInstruction::Mul { left, right }
        | NestedInstruction::Div { left, right }
        | NestedInstruction::Rem { left, right }
        | NestedInstruction::Shl { left, right }
        | NestedInstruction::Shr { left, right }
        | NestedInstruction::UShr { left, right }
        | NestedInstruction::BitwiseAnd { left, right }
        | NestedInstruction::BitwiseXor { left, right }
        | NestedInstruction::BitwiseOr { left, right } => {
            remap_register(left)?;
            remap_register(right)
        }
        NestedInstruction::Plus { value }
        | NestedInstruction::Minus { value }
        | NestedInstruction::BitwiseNot { value }
        | NestedInstruction::Increment { value }
        | NestedInstruction::Decrement { value } => remap_register(value),
    }
}

fn remapped_register(register: Register, remap: &[Register; MAX_DENSE_OPS]) -> Option<Register> {
    let remapped = *remap.get(register)?;
    (remapped != usize::MAX).then_some(remapped)
}

pub(super) fn assign_index_caches(operations: &mut [NestedInstruction]) -> bool {
    let mut registers = [usize::MAX; MAX_INDEX_CACHES];
    let mut uses = [0_u8; MAX_INDEX_CACHES];
    let mut register_count = 0;
    for operation in operations.iter() {
        for index in dense_index_registers(*operation).into_iter().flatten() {
            let slot = match registers[..register_count]
                .iter()
                .position(|register| *register == index)
            {
                Some(slot) => slot,
                None if register_count < MAX_INDEX_CACHES => {
                    registers[register_count] = index;
                    register_count += 1;
                    register_count - 1
                }
                None => continue,
            };
            uses[slot] = uses[slot].saturating_add(1);
        }
    }

    let mut cached = false;
    for operation in operations {
        let mut caches = [None; 2];
        for (position, index) in dense_index_registers(*operation).into_iter().enumerate() {
            let Some(index) = index else {
                continue;
            };
            let slot = registers[..register_count]
                .iter()
                .position(|register| *register == index)
                .filter(|slot| uses[*slot] > 1)
                .and_then(|slot| u8::try_from(slot).ok());
            cached |= slot.is_some();
            caches[position] = slot;
        }
        set_dense_index_caches(operation, caches);
    }
    cached
}

fn dense_index_registers(operation: NestedInstruction) -> [Option<Register>; 2] {
    match operation {
        NestedInstruction::DenseLoad { index, .. }
        | NestedInstruction::DenseStore { index, .. }
        | NestedInstruction::StoreAdd { index, .. }
        | NestedInstruction::StoreSub { index, .. } => [Some(index), None],
        NestedInstruction::StoreAddFromLoad {
            index,
            source_index,
            ..
        }
        | NestedInstruction::StoreSubFromLoad {
            index,
            source_index,
            ..
        } => [Some(source_index), Some(index)],
        _ => [None, None],
    }
}

fn set_dense_index_caches(operation: &mut NestedInstruction, caches: [Option<u8>; 2]) {
    match operation {
        NestedInstruction::DenseLoad {
            index_cache: slot, ..
        }
        | NestedInstruction::DenseStore {
            index_cache: slot, ..
        }
        | NestedInstruction::StoreAdd {
            index_cache: slot, ..
        }
        | NestedInstruction::StoreSub {
            index_cache: slot, ..
        } => *slot = caches[0],
        NestedInstruction::StoreAddFromLoad {
            index_cache,
            source_index_cache,
            ..
        }
        | NestedInstruction::StoreSubFromLoad {
            index_cache,
            source_index_cache,
            ..
        } => {
            *source_index_cache = caches[0];
            *index_cache = caches[1];
        }
        _ => {}
    }
}

pub(super) trait IndexResolver: Sized {
    fn fresh() -> Self;

    fn resolve(&mut self, slot: Option<u8>, value: f64) -> Option<usize>;
}

pub(super) struct NoIndexCache;

impl IndexResolver for NoIndexCache {
    #[inline(always)]
    fn fresh() -> Self {
        Self
    }

    #[inline(always)]
    fn resolve(&mut self, _slot: Option<u8>, value: f64) -> Option<usize> {
        array_index_from_number(value)
    }
}

pub(super) struct IndexCache {
    values: [usize; MAX_INDEX_CACHES],
    initialized: u8,
}

impl IndexCache {
    pub(super) fn new() -> Self {
        Self {
            values: [0; MAX_INDEX_CACHES],
            initialized: 0,
        }
    }
}

impl IndexResolver for IndexCache {
    #[inline(always)]
    fn fresh() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn resolve(&mut self, slot: Option<u8>, value: f64) -> Option<usize> {
        let Some(slot) = slot else {
            return array_index_from_number(value);
        };
        let bit = 1_u8 << slot;
        if self.initialized & bit != 0 {
            return Some(self.values[usize::from(slot)]);
        }
        let index = array_index_from_number(value)?;
        self.values[usize::from(slot)] = index;
        self.initialized |= bit;
        Some(index)
    }
}

#[inline(always)]
pub(super) fn run_operation<A: DenseAccess, I: IndexResolver>(
    operation: NestedInstruction,
    access: &mut A,
    bank: &LocalBank,
    registers: &[f64],
    indices: &mut I,
) -> Option<f64> {
    Some(match operation {
        NestedInstruction::Constant(value) => value,
        NestedInstruction::LoadLocal(local) => bank.number(local)?,
        NestedInstruction::DenseLoad {
            receiver,
            index,
            index_cache,
        } => access.load_number(receiver, indices.resolve(index_cache, registers[index])?)?,
        NestedInstruction::DenseStore {
            receiver,
            index,
            value,
            index_cache,
        } => {
            let index = indices.resolve(index_cache, registers[index])?;
            let value = registers[value];
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::StoreAdd {
            receiver,
            index,
            left,
            right,
            index_cache,
        } => {
            let value = registers[left] + registers[right];
            let index = indices.resolve(index_cache, registers[index])?;
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::StoreSub {
            receiver,
            index,
            left,
            right,
            index_cache,
        } => {
            let value = registers[left] - registers[right];
            let index = indices.resolve(index_cache, registers[index])?;
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::StoreAddFromLoad {
            receiver,
            index,
            index_cache,
            source_receiver,
            source_index,
            source_index_cache,
            other,
        } => {
            let source_index = indices.resolve(source_index_cache, registers[source_index])?;
            let source = access.load_number(source_receiver, source_index)?;
            let value = source + registers[other];
            let index = indices.resolve(index_cache, registers[index])?;
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::StoreSubFromLoad {
            receiver,
            index,
            index_cache,
            source_receiver,
            source_index,
            source_index_cache,
            other,
            load_on_left,
        } => {
            let source_index = indices.resolve(source_index_cache, registers[source_index])?;
            let source = access.load_number(source_receiver, source_index)?;
            let other = registers[other];
            let value = if load_on_left {
                source - other
            } else {
                other - source
            };
            let index = indices.resolve(index_cache, registers[index])?;
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::MulAdd {
            left_left,
            left_right,
            right_left,
            right_right,
        } => {
            let left = registers[left_left] * registers[left_right];
            let right = registers[right_left] * registers[right_right];
            left + right
        }
        NestedInstruction::MulSub {
            left_left,
            left_right,
            right_left,
            right_right,
        } => {
            let left = registers[left_left] * registers[left_right];
            let right = registers[right_left] * registers[right_right];
            left - right
        }
        NestedInstruction::Add { left, right } => registers[left] + registers[right],
        NestedInstruction::Sub { left, right } => registers[left] - registers[right],
        NestedInstruction::Mul { left, right } => registers[left] * registers[right],
        NestedInstruction::Div { left, right } => registers[left] / registers[right],
        NestedInstruction::Rem { left, right } => {
            crate::operations::number_remainder(registers[left], registers[right])
        }
        NestedInstruction::Shl { left, right } => f64::from(
            crate::to_int32_number(registers[left])
                << (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::Shr { left, right } => f64::from(
            crate::to_int32_number(registers[left])
                >> (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::UShr { left, right } => f64::from(
            crate::to_uint32_number(registers[left])
                >> (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::BitwiseAnd { left, right } => f64::from(
            crate::to_int32_number(registers[left]) & crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::BitwiseXor { left, right } => f64::from(
            crate::to_int32_number(registers[left]) ^ crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::BitwiseOr { left, right } => f64::from(
            crate::to_int32_number(registers[left]) | crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::Plus { value } => registers[value],
        NestedInstruction::Minus { value } => -registers[value],
        NestedInstruction::BitwiseNot { value } => {
            f64::from(!crate::to_int32_number(registers[value]))
        }
        NestedInstruction::Increment { value } => registers[value] + 1.0,
        NestedInstruction::Decrement { value } => registers[value] - 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RejectingAccess;

    impl DenseAccess for RejectingAccess {
        fn reset_iteration(&mut self) {}

        fn load_number(&self, _receiver: usize, _index: usize) -> Option<f64> {
            None
        }

        fn stage_store(&mut self, _receiver: usize, _index: usize, _value: f64) -> bool {
            false
        }

        fn staged_store_count(&self) -> usize {
            0
        }

        fn commit_stores(&mut self) {}
    }

    fn empty_bank() -> LocalBank {
        LocalBank {
            values: [0.0; super::super::super::MAX_DENSE_LOCALS],
            valid: [false; super::super::super::MAX_DENSE_LOCALS],
        }
    }

    #[test]
    fn fuses_terminal_add_sub_stores_and_remaps_their_results() {
        let mut operations = vec![
            NestedInstruction::Constant(10.0),
            NestedInstruction::Constant(4.0),
            NestedInstruction::Constant(0.0),
            NestedInstruction::Add { left: 0, right: 1 },
            NestedInstruction::DenseStore {
                receiver: 0,
                index: 2,
                value: 3,
                index_cache: None,
            },
            NestedInstruction::Sub { left: 3, right: 1 },
            NestedInstruction::DenseStore {
                receiver: 1,
                index: 2,
                value: 5,
                index_cache: None,
            },
            NestedInstruction::Increment { value: 6 },
        ];
        let mut writes = [LocalWrite { local: 0, value: 7 }];
        let mut counter_write = 7;

        assert!(fuse_terminal_add_sub_stores(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 6);
        assert!(matches!(
            operations[3],
            NestedInstruction::StoreAdd {
                receiver: 0,
                index: 2,
                left: 0,
                right: 1,
                index_cache: None,
            }
        ));
        assert!(matches!(
            operations[4],
            NestedInstruction::StoreSub {
                receiver: 1,
                index: 2,
                left: 3,
                right: 1,
                index_cache: None,
            }
        ));
        assert!(matches!(
            operations[5],
            NestedInstruction::Increment { value: 4 }
        ));
        assert_eq!(writes, [LocalWrite { local: 0, value: 5 }]);
        assert_eq!(counter_write, 5);
    }

    #[test]
    fn retains_a_binary_used_as_its_store_index() {
        let mut operations = vec![
            NestedInstruction::Constant(0.0),
            NestedInstruction::Constant(1.0),
            NestedInstruction::Add { left: 0, right: 1 },
            NestedInstruction::DenseStore {
                receiver: 0,
                index: 2,
                value: 2,
                index_cache: None,
            },
        ];
        let mut writes = [LocalWrite { local: 0, value: 3 }];
        let mut counter_write = 3;

        assert!(!fuse_terminal_add_sub_stores(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 4);
        assert!(matches!(operations[2], NestedInstruction::Add { .. }));
        assert!(matches!(
            operations[3],
            NestedInstruction::DenseStore { .. }
        ));
        assert_eq!(writes, [LocalWrite { local: 0, value: 3 }]);
        assert_eq!(counter_write, 3);
    }

    #[test]
    fn fuses_unique_dense_loads_into_arithmetic_stores_and_remaps_outputs() {
        let mut operations = vec![
            NestedInstruction::Constant(0.0),
            NestedInstruction::Constant(1.0),
            NestedInstruction::Constant(2.0),
            NestedInstruction::DenseLoad {
                receiver: 0,
                index: 0,
                index_cache: None,
            },
            NestedInstruction::StoreSub {
                receiver: 1,
                index: 1,
                left: 2,
                right: 3,
                index_cache: None,
            },
            NestedInstruction::DenseLoad {
                receiver: 1,
                index: 1,
                index_cache: None,
            },
            NestedInstruction::StoreAdd {
                receiver: 0,
                index: 0,
                left: 5,
                right: 2,
                index_cache: None,
            },
            NestedInstruction::Increment { value: 6 },
        ];
        let mut writes = [LocalWrite { local: 0, value: 7 }];
        let mut counter_write = 7;

        assert!(fuse_dense_load_stores(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 6);
        assert!(matches!(
            operations[3],
            NestedInstruction::StoreSubFromLoad {
                receiver: 1,
                index: 1,
                source_receiver: 0,
                source_index: 0,
                other: 2,
                load_on_left: false,
                ..
            }
        ));
        assert!(matches!(
            operations[4],
            NestedInstruction::StoreAddFromLoad {
                receiver: 0,
                index: 0,
                source_receiver: 1,
                source_index: 1,
                other: 2,
                ..
            }
        ));
        assert!(matches!(
            operations[5],
            NestedInstruction::Increment { value: 4 }
        ));
        assert_eq!(writes, [LocalWrite { local: 0, value: 5 }]);
        assert_eq!(counter_write, 5);

        assert!(assign_index_caches(&mut operations));
        assert!(matches!(
            operations[3],
            NestedInstruction::StoreSubFromLoad {
                source_index_cache: Some(0),
                index_cache: Some(1),
                ..
            }
        ));
        assert!(matches!(
            operations[4],
            NestedInstruction::StoreAddFromLoad {
                source_index_cache: Some(1),
                index_cache: Some(0),
                ..
            }
        ));
    }

    #[test]
    fn retains_dense_load_with_a_second_consumer() {
        let mut operations = vec![
            NestedInstruction::Constant(0.0),
            NestedInstruction::DenseLoad {
                receiver: 0,
                index: 0,
                index_cache: None,
            },
            NestedInstruction::StoreAdd {
                receiver: 1,
                index: 0,
                left: 1,
                right: 0,
                index_cache: None,
            },
            NestedInstruction::Add { left: 1, right: 0 },
        ];
        let mut writes = [LocalWrite { local: 0, value: 3 }];
        let mut counter_write = 3;

        assert!(!fuse_dense_load_stores(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 4);
        assert!(matches!(operations[1], NestedInstruction::DenseLoad { .. }));
        assert!(matches!(operations[2], NestedInstruction::StoreAdd { .. }));
        assert_eq!(writes, [LocalWrite { local: 0, value: 3 }]);
        assert_eq!(counter_write, 3);
    }

    #[test]
    fn fuses_unique_multiplication_pairs_and_remaps_later_consumers() {
        let mut operations = vec![
            NestedInstruction::Constant(2.0),
            NestedInstruction::Constant(3.0),
            NestedInstruction::Constant(5.0),
            NestedInstruction::Constant(7.0),
            NestedInstruction::Mul { left: 0, right: 1 },
            NestedInstruction::Mul { left: 2, right: 3 },
            NestedInstruction::Sub { left: 4, right: 5 },
            NestedInstruction::Mul { left: 0, right: 2 },
            NestedInstruction::Mul { left: 1, right: 3 },
            NestedInstruction::Add { left: 7, right: 8 },
            NestedInstruction::Add { left: 6, right: 9 },
            NestedInstruction::Increment { value: 10 },
        ];
        let mut writes = [LocalWrite {
            local: 0,
            value: 11,
        }];
        let mut counter_write = 11;

        assert!(fuse_mul_add_sub(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 8);
        assert!(matches!(
            operations[4],
            NestedInstruction::MulSub {
                left_left: 0,
                left_right: 1,
                right_left: 2,
                right_right: 3,
            }
        ));
        assert!(matches!(
            operations[5],
            NestedInstruction::MulAdd {
                left_left: 0,
                left_right: 2,
                right_left: 1,
                right_right: 3,
            }
        ));
        assert!(matches!(
            operations[6],
            NestedInstruction::Add { left: 4, right: 5 }
        ));
        assert!(matches!(
            operations[7],
            NestedInstruction::Increment { value: 6 }
        ));
        assert_eq!(writes, [LocalWrite { local: 0, value: 7 }]);
        assert_eq!(counter_write, 7);
    }

    #[test]
    fn retains_multiplication_with_a_second_consumer() {
        let mut operations = vec![
            NestedInstruction::Constant(2.0),
            NestedInstruction::Constant(3.0),
            NestedInstruction::Constant(5.0),
            NestedInstruction::Mul { left: 0, right: 1 },
            NestedInstruction::Mul { left: 0, right: 2 },
            NestedInstruction::Add { left: 3, right: 4 },
            NestedInstruction::Add { left: 3, right: 5 },
        ];
        let mut writes = [LocalWrite { local: 0, value: 6 }];
        let mut counter_write = 6;

        assert!(!fuse_mul_add_sub(
            &mut operations,
            &mut writes,
            &mut counter_write,
        ));
        assert_eq!(operations.len(), 7);
        assert!(matches!(operations[3], NestedInstruction::Mul { .. }));
        assert!(matches!(operations[5], NestedInstruction::Add { .. }));
        assert_eq!(writes, [LocalWrite { local: 0, value: 6 }]);
        assert_eq!(counter_write, 6);
    }

    #[test]
    fn fused_multiplication_pairs_keep_intermediate_rounding() {
        let epsilon = 2_f64.powi(-27);
        let registers = [1.0 + epsilon, 1.0 - epsilon, -1.0, 1.0];
        let mut access = RejectingAccess;
        let mut indices = NoIndexCache;

        let result = run_operation(
            NestedInstruction::MulAdd {
                left_left: 0,
                left_right: 1,
                right_left: 2,
                right_right: 3,
            },
            &mut access,
            &empty_bank(),
            &registers,
            &mut indices,
        )
        .expect("pure fused arithmetic succeeds");

        // A fused multiply-add would retain the -2^-54 residual. JavaScript
        // requires the two multiplication roundings before the addition.
        assert_eq!(result.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn assigns_index_caches_only_to_repeated_registers() {
        let mut operations = [
            NestedInstruction::DenseLoad {
                receiver: 0,
                index: 3,
                index_cache: None,
            },
            NestedInstruction::DenseStore {
                receiver: 1,
                index: 4,
                value: 0,
                index_cache: None,
            },
            NestedInstruction::DenseStore {
                receiver: 0,
                index: 3,
                value: 1,
                index_cache: None,
            },
        ];

        assert!(assign_index_caches(&mut operations));
        assert!(matches!(
            operations[0],
            NestedInstruction::DenseLoad {
                index_cache: Some(0),
                ..
            }
        ));
        assert!(matches!(
            operations[1],
            NestedInstruction::DenseStore {
                index_cache: None,
                ..
            }
        ));
        assert!(matches!(
            operations[2],
            NestedInstruction::DenseStore {
                index_cache: Some(0),
                ..
            }
        ));
    }

    #[test]
    fn limits_index_caches_to_fixed_capacity() {
        let mut operations = (0..=MAX_INDEX_CACHES)
            .flat_map(|index| {
                [
                    NestedInstruction::DenseLoad {
                        receiver: 0,
                        index,
                        index_cache: None,
                    },
                    NestedInstruction::DenseLoad {
                        receiver: 1,
                        index,
                        index_cache: None,
                    },
                ]
            })
            .collect::<Vec<_>>();

        assert!(assign_index_caches(&mut operations));
        for (index, pair) in operations.chunks_exact(2).enumerate() {
            let expected = (index < MAX_INDEX_CACHES).then_some(index as u8);
            assert!(pair.iter().all(|operation| matches!(
                operation,
                NestedInstruction::DenseLoad { index_cache, .. } if *index_cache == expected
            )));
        }
    }

    #[test]
    fn index_cache_reuses_only_validated_array_indices() {
        let mut cache = IndexCache::new();
        assert_eq!(cache.resolve(Some(0), 3.0), Some(3));
        assert_eq!(cache.resolve(Some(0), 3.0), Some(3));
        assert_eq!(cache.resolve(Some(1), -1.0), None);

        let mut next_iteration = IndexCache::fresh();
        assert_eq!(next_iteration.resolve(Some(0), 7.0), Some(7));
    }
}
