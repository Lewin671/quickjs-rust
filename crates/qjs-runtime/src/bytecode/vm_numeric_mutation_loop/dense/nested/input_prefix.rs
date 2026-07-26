//! Canonical input registers for nested dense programs.

use super::super::*;

/// Pure inputs to one nested dense iteration, partitioned by the scope over
/// which their values remain valid. The compiler's SSA map guarantees that a
/// `LoadLocal` whose local is written by the inner program reads that
/// iteration's entry value; all other local inputs stay unchanged until the
/// enclosing scalar prelude or epilogue runs.
#[derive(Clone, Copy, Debug)]
pub(super) struct NestedInputPrefix {
    pub(super) constant_count: usize,
    pub(super) invariant_local_count: usize,
    pub(super) carried_local_count: usize,
}

impl NestedInputPrefix {
    pub(super) fn dynamic_start(self) -> usize {
        self.constant_count + self.invariant_local_count + self.carried_local_count
    }

    pub(super) fn invariant_local_start(self) -> usize {
        self.constant_count
    }

    pub(super) fn carried_local_start(self) -> usize {
        self.constant_count + self.invariant_local_count
    }
}

pub(super) fn compact_inner_inputs(
    operations: &mut Vec<NumberInstruction>,
    writes: &mut [LocalWrite],
    counter_write: &mut Register,
) -> Option<NestedInputPrefix> {
    let operation_count = operations.len();
    if operation_count > MAX_DENSE_OPS
        || operations
            .iter()
            .enumerate()
            .any(|(destination, operation)| !operation_registers_are_valid(operation, destination))
        || writes.iter().any(|write| write.value >= operation_count)
        || *counter_write >= operation_count
    {
        return None;
    }

    let mut constant_bits = [0_u64; MAX_DENSE_OPS];
    let mut constant_count = 0;
    let mut invariant_locals = [0_usize; MAX_DENSE_OPS];
    let mut invariant_local_count = 0;
    let mut carried_locals = [0_usize; MAX_DENSE_OPS];
    let mut carried_local_count = 0;
    for operation in operations.iter() {
        match *operation {
            NumberInstruction::Constant(value) => {
                let bits = value.to_bits();
                if !constant_bits[..constant_count].contains(&bits) {
                    constant_bits[constant_count] = bits;
                    constant_count += 1;
                }
            }
            NumberInstruction::LoadLocal(local) => {
                let (locals, count) = if writes.iter().any(|write| write.local == local) {
                    (&mut carried_locals, &mut carried_local_count)
                } else {
                    (&mut invariant_locals, &mut invariant_local_count)
                };
                if !locals[..*count].contains(&local) {
                    locals[*count] = local;
                    *count += 1;
                }
            }
            NumberInstruction::LoadInvariant(_)
            | NumberInstruction::DenseLoad { .. }
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. }
            | NumberInstruction::MathRound { .. } => {}
        }
    }

    let prefix = NestedInputPrefix {
        constant_count,
        invariant_local_count,
        carried_local_count,
    };
    let mut remap = [usize::MAX; MAX_DENSE_OPS];
    let mut next_dynamic = prefix.dynamic_start();
    for (old_register, operation) in operations.iter().enumerate() {
        remap[old_register] = match *operation {
            NumberInstruction::Constant(value) => constant_bits[..constant_count]
                .iter()
                .position(|bits| *bits == value.to_bits())?,
            NumberInstruction::LoadLocal(local) => {
                let (locals, start) = if writes.iter().any(|write| write.local == local) {
                    (
                        &carried_locals[..carried_local_count],
                        prefix.carried_local_start(),
                    )
                } else {
                    (
                        &invariant_locals[..invariant_local_count],
                        prefix.invariant_local_start(),
                    )
                };
                start + locals.iter().position(|candidate| *candidate == local)?
            }
            NumberInstruction::LoadInvariant(_)
            | NumberInstruction::DenseLoad { .. }
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. }
            | NumberInstruction::MathRound { .. } => {
                let register = next_dynamic;
                next_dynamic += 1;
                register
            }
        };
    }

    let old_operations = std::mem::take(operations);
    operations.reserve(next_dynamic);
    operations.extend(
        constant_bits[..constant_count]
            .iter()
            .copied()
            .map(f64::from_bits)
            .map(NumberInstruction::Constant),
    );
    operations.extend(
        invariant_locals[..invariant_local_count]
            .iter()
            .copied()
            .map(NumberInstruction::LoadLocal),
    );
    operations.extend(
        carried_locals[..carried_local_count]
            .iter()
            .copied()
            .map(NumberInstruction::LoadLocal),
    );
    for (old_register, mut operation) in old_operations.into_iter().enumerate() {
        if matches!(
            operation,
            NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_)
        ) {
            continue;
        }
        debug_assert_eq!(operations.len(), remap[old_register]);
        remap_operation_registers(&mut operation, &remap[..operation_count]);
        operations.push(operation);
    }
    for write in writes {
        write.value = remap[write.value];
    }
    *counter_write = remap[*counter_write];
    debug_assert_eq!(operations.len(), next_dynamic);
    Some(prefix)
}

fn operation_registers_are_valid(operation: &NumberInstruction, destination: usize) -> bool {
    let valid = |register: Register| register < destination;
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => true,
        NumberInstruction::DenseLoad { index, .. } => valid(*index),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => valid(*index) && valid(*value),
        NumberInstruction::Unary { value, .. }
        | NumberInstruction::Update { value, .. }
        | NumberInstruction::MathRound { value } => valid(*value),
    }
}

fn remap_operation_registers(operation: &mut NumberInstruction, remap: &[usize]) {
    let remap_register = |register: &mut Register| *register = remap[*register];
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => {}
        NumberInstruction::DenseLoad { index, .. } => remap_register(index),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => {
            remap_register(index);
            remap_register(value);
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
    fn partitions_inputs_and_remaps_every_inner_output() {
        let payload_nan = f64::from_bits(0x7ff8_0000_0000_0001);
        let mut operations = vec![
            NumberInstruction::Constant(0.0),
            NumberInstruction::Constant(-0.0),
            NumberInstruction::Constant(payload_nan),
            NumberInstruction::Constant(0.0),
            NumberInstruction::LoadLocal(5),
            NumberInstruction::LoadLocal(7),
            NumberInstruction::LoadLocal(5),
            NumberInstruction::LoadLocal(7),
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 4,
                right: 5,
            },
            NumberInstruction::Binary {
                operation: BinaryOp::Mul,
                left: 8,
                right: 2,
            },
            NumberInstruction::DenseStore {
                receiver: 0,
                index: 5,
                value: 9,
            },
        ];
        let mut writes = [LocalWrite { local: 7, value: 9 }];
        let mut counter_write = 9;

        let prefix = compact_inner_inputs(&mut operations, &mut writes, &mut counter_write)
            .expect("well-formed inner program should compact");

        assert_eq!(prefix.constant_count, 3);
        assert_eq!(prefix.invariant_local_count, 1);
        assert_eq!(prefix.carried_local_count, 1);
        assert_eq!(prefix.dynamic_start(), 5);
        assert_eq!(operations.len(), 8);
        let NumberInstruction::Constant(first) = operations[0] else {
            panic!("first prefix register should be a constant");
        };
        let NumberInstruction::Constant(second) = operations[1] else {
            panic!("second prefix register should be a constant");
        };
        let NumberInstruction::Constant(third) = operations[2] else {
            panic!("third prefix register should be a constant");
        };
        assert_eq!(first.to_bits(), 0.0_f64.to_bits());
        assert_eq!(second.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(third.to_bits(), payload_nan.to_bits());
        assert!(matches!(operations[3], NumberInstruction::LoadLocal(5)));
        assert!(matches!(operations[4], NumberInstruction::LoadLocal(7)));
        assert!(matches!(
            operations[5],
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 3,
                right: 4,
            }
        ));
        assert!(matches!(
            operations[6],
            NumberInstruction::Binary {
                operation: BinaryOp::Mul,
                left: 5,
                right: 2,
            }
        ));
        assert!(matches!(
            operations[7],
            NumberInstruction::DenseStore {
                receiver: 0,
                index: 4,
                value: 6,
            }
        ));
        assert_eq!(writes[0].value, 6);
        assert_eq!(counter_write, 6);
    }
}
