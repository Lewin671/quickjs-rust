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

fn add_or_sub(operation: NestedInstruction) -> Option<(bool, Register, Register)> {
    match operation {
        NestedInstruction::Add { left, right } => Some((true, left, right)),
        NestedInstruction::Sub { left, right } => Some((false, left, right)),
        _ => None,
    }
}

fn operation_references_are_valid(operation: NestedInstruction, destination: Register) -> bool {
    let valid = |register: Register| register < destination;
    match operation {
        NestedInstruction::Constant(_) | NestedInstruction::LoadLocal(_) => true,
        NestedInstruction::DenseLoad { index, .. } => valid(index),
        NestedInstruction::DenseStore { index, value, .. } => valid(index) && valid(value),
        NestedInstruction::StoreAdd {
            index, left, right, ..
        }
        | NestedInstruction::StoreSub {
            index, left, right, ..
        } => valid(index) && valid(left) && valid(right),
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
        | NestedInstruction::BitwiseOr { left, right } => valid(left) && valid(right),
        NestedInstruction::Plus { value }
        | NestedInstruction::Minus { value }
        | NestedInstruction::BitwiseNot { value }
        | NestedInstruction::Increment { value }
        | NestedInstruction::Decrement { value } => valid(value),
    }
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
        let Some(index) = dense_index_register(*operation) else {
            continue;
        };
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

    let mut cached = false;
    for operation in operations {
        let Some(index) = dense_index_register(*operation) else {
            continue;
        };
        let slot = registers[..register_count]
            .iter()
            .position(|register| *register == index)
            .filter(|slot| uses[*slot] > 1)
            .and_then(|slot| u8::try_from(slot).ok());
        cached |= slot.is_some();
        set_dense_index_cache(operation, slot);
    }
    cached
}

fn dense_index_register(operation: NestedInstruction) -> Option<Register> {
    match operation {
        NestedInstruction::DenseLoad { index, .. }
        | NestedInstruction::DenseStore { index, .. }
        | NestedInstruction::StoreAdd { index, .. }
        | NestedInstruction::StoreSub { index, .. } => Some(index),
        _ => None,
    }
}

fn set_dense_index_cache(operation: &mut NestedInstruction, index_cache: Option<u8>) {
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
        } => *slot = index_cache,
        _ => {}
    }
}

pub(super) trait IndexResolver {
    fn resolve(&mut self, slot: Option<u8>, value: f64) -> Option<usize>;
}

pub(super) struct NoIndexCache;

impl IndexResolver for NoIndexCache {
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

        let mut next_iteration = IndexCache::new();
        assert_eq!(next_iteration.resolve(Some(0), 7.0), Some(7));
    }
}
