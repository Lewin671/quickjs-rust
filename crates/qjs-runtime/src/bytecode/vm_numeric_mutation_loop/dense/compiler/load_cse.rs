//! Register compaction for redundant dense reads in a transactional program.

use super::*;

/// Removes repeated reads of the exact same leased dense element before a
/// store. The translated number program cannot run arbitrary JavaScript, so a
/// successful earlier `DenseLoad` remains valid until a staged store changes
/// what a later load may observe. Clear every cached load at a store rather
/// than attempting to prove receiver or index disjointness here: the compiler
/// stays conservative while the hot executor skips the duplicate access.
pub(super) fn eliminate_redundant_dense_loads(
    operations: &mut Vec<NumberInstruction>,
    writes: &mut [LocalWrite],
) -> Option<usize> {
    const UNMAPPED: Register = usize::MAX;

    let operation_count = operations.len();
    if operation_count > MAX_DENSE_OPS
        || operations
            .iter()
            .enumerate()
            .any(|(destination, operation)| !operation_references_are_valid(operation, destination))
        || writes.iter().any(|write| write.value >= operation_count)
    {
        return None;
    }

    let old_operations = std::mem::take(operations);
    let mut remap = [UNMAPPED; MAX_DENSE_OPS];
    let mut cached_loads = [[UNMAPPED; MAX_DENSE_OPS]; MAX_DENSE_RECEIVERS];
    let mut rewritten = Vec::with_capacity(operation_count);
    for (old_register, mut operation) in old_operations.into_iter().enumerate() {
        remap_operation_registers(&mut operation, &remap)
            .expect("validated dense programs only reference earlier registers");
        match operation {
            NumberInstruction::DenseLoad { receiver, index } => {
                let cached = cached_loads[receiver][index];
                if cached != UNMAPPED {
                    remap[old_register] = cached;
                    continue;
                }
                let register = rewritten.len();
                cached_loads[receiver][index] = register;
                remap[old_register] = register;
                rewritten.push(NumberInstruction::DenseLoad { receiver, index });
            }
            NumberInstruction::DenseStore { .. } => {
                // A staged store is immediately visible to later loads in the
                // same source iteration through `DenseAccess` forwarding.
                for cached in &mut cached_loads {
                    cached.fill(UNMAPPED);
                }
                remap[old_register] = rewritten.len();
                rewritten.push(operation);
            }
            _ => {
                remap[old_register] = rewritten.len();
                rewritten.push(operation);
            }
        }
    }

    let mut remapped_writes = writes.to_vec();
    for write in &mut remapped_writes {
        write.value = remap[write.value];
        debug_assert_ne!(write.value, UNMAPPED);
    }
    let eliminated = operation_count - rewritten.len();
    *operations = rewritten;
    writes.copy_from_slice(&remapped_writes);
    Some(eliminated)
}

fn operation_references_are_valid(operation: &NumberInstruction, destination: Register) -> bool {
    let valid = |register: Register| register < destination;
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => true,
        NumberInstruction::DenseLoad { receiver, index } => {
            *receiver < MAX_DENSE_RECEIVERS && valid(*index)
        }
        NumberInstruction::DenseStore {
            receiver,
            index,
            value,
        } => *receiver < MAX_DENSE_RECEIVERS && valid(*index) && valid(*value),
        NumberInstruction::Binary { left, right, .. } => valid(*left) && valid(*right),
        NumberInstruction::Unary { value, .. }
        | NumberInstruction::Update { value, .. }
        | NumberInstruction::MathRound { value } => valid(*value),
    }
}

fn remap_operation_registers(
    operation: &mut NumberInstruction,
    remap: &[Register; MAX_DENSE_OPS],
) -> Option<()> {
    let remap_register = |register: &mut Register| {
        let remapped = *remap.get(*register)?;
        (remapped != usize::MAX).then_some(remapped).map(|value| {
            *register = value;
        })
    };
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => Some(()),
        NumberInstruction::DenseLoad { index, .. } => remap_register(index),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => {
            remap_register(index)?;
            remap_register(value)
        }
        NumberInstruction::Unary { value, .. }
        | NumberInstruction::Update { value, .. }
        | NumberInstruction::MathRound { value } => remap_register(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redundant_dense_loads_share_registers_only_until_a_store() {
        let mut operations = vec![
            NumberInstruction::LoadLocal(0),
            NumberInstruction::DenseLoad {
                receiver: 0,
                index: 0,
            },
            NumberInstruction::DenseLoad {
                receiver: 0,
                index: 0,
            },
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 1,
                right: 2,
            },
            NumberInstruction::DenseStore {
                receiver: 0,
                index: 0,
                value: 3,
            },
            NumberInstruction::DenseLoad {
                receiver: 0,
                index: 0,
            },
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 5,
                right: 0,
            },
        ];
        let mut writes = [LocalWrite { local: 1, value: 6 }];

        assert_eq!(
            eliminate_redundant_dense_loads(&mut operations, &mut writes),
            Some(1)
        );
        assert_eq!(operations.len(), 6);
        assert!(matches!(
            operations[1],
            NumberInstruction::DenseLoad {
                receiver: 0,
                index: 0,
            }
        ));
        assert!(matches!(
            operations[2],
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 1,
                right: 1,
            }
        ));
        assert!(matches!(
            operations[3],
            NumberInstruction::DenseStore {
                receiver: 0,
                index: 0,
                value: 2,
            }
        ));
        assert!(matches!(
            operations[4],
            NumberInstruction::DenseLoad {
                receiver: 0,
                index: 0,
            }
        ));
        assert!(matches!(
            operations[5],
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 4,
                right: 0,
            }
        ));
        assert_eq!(writes, [LocalWrite { local: 1, value: 5 }]);
    }
}
