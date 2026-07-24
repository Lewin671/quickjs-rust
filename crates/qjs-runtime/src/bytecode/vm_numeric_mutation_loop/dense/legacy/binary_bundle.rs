//! Detection of independent, isomorphic pure-Binary register chains.
//!
//! Bundles are immutable execution metadata. They never change the operation
//! stream or its `destination_register == operation_index` numbering. A
//! consumer may only change the order of operations inside a validated bundle:
//! every lane reads either an input produced before the whole bundle or an
//! earlier result from that same lane.

use qjs_ast::BinaryOp;

use super::NumberInstruction;

pub(super) const MAX_BINARY_BUNDLE_LANES: usize = 4;
pub(super) const MAX_BINARY_CHAIN_LENGTH: usize = 16;
const MAX_EXTERNAL_ALIASES: usize = MAX_BINARY_CHAIN_LENGTH * 2;
const MIN_SAVED_DISPATCHES: usize = 4;
// Three two-operation lanes are the smallest layout that saves four dynamic
// dispatches. Shorter Binary runs cannot contain any accepted bundle.
const MIN_BINARY_BUNDLE_OPERATIONS: usize = 6;
const MAX_BINARY_BUNDLE_CANDIDATES: usize = (MAX_BINARY_BUNDLE_LANES - 1) * MAX_BINARY_CHAIN_LENGTH;
const MAX_BINARY_BUNDLE_OPERATIONS: usize = MAX_BINARY_BUNDLE_LANES * MAX_BINARY_CHAIN_LENGTH;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BinaryBundle {
    pub(super) start: usize,
    pub(super) lane_count: usize,
    pub(super) chain_length: usize,
}

impl BinaryBundle {
    pub(super) fn operation_count(self) -> Option<usize> {
        self.lane_count.checked_mul(self.chain_length)
    }

    pub(super) fn saved_dispatches(self) -> Option<usize> {
        self.lane_count
            .checked_sub(1)?
            .checked_mul(self.chain_length)
    }

    pub(super) fn validated_end(
        self,
        operations: &[NumberInstruction],
        dynamic_start: usize,
    ) -> Option<usize> {
        self.validate(operations, dynamic_start)
            .map(|candidate| candidate.end)
    }

    fn checked_layout(
        self,
        operation_count: usize,
        dynamic_start: usize,
    ) -> Option<(usize, usize)> {
        if self.start < dynamic_start
            || !(2..=MAX_BINARY_BUNDLE_LANES).contains(&self.lane_count)
            || !(1..=MAX_BINARY_CHAIN_LENGTH).contains(&self.chain_length)
        {
            return None;
        }
        let saved_dispatches = self.saved_dispatches()?;
        if saved_dispatches < MIN_SAVED_DISPATCHES {
            return None;
        }
        let end = self.start.checked_add(self.operation_count()?)?;
        (end <= operation_count).then_some((end, saved_dispatches))
    }

    fn validate(
        self,
        operations: &[NumberInstruction],
        dynamic_start: usize,
    ) -> Option<ValidatedCandidate> {
        let (end, saved_dispatches) = self.checked_layout(operations.len(), dynamic_start)?;
        if operations[self.start..end]
            .iter()
            .any(|operation| !matches!(operation, NumberInstruction::Binary { .. }))
            || !is_isomorphic_bundle(operations, self)
        {
            return None;
        }
        Some(ValidatedCandidate {
            bundle: self,
            end,
            saved_dispatches,
        })
    }
}

#[derive(Clone, Copy)]
struct ValidatedCandidate {
    bundle: BinaryBundle,
    end: usize,
    saved_dispatches: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OperandShape {
    External(u8),
    Internal(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OperationShape {
    operation: BinaryOp,
    left: OperandShape,
    right: OperandShape,
}

fn operand_shape(
    register: usize,
    destination: usize,
    bundle_start: usize,
    lane_start: usize,
    external_registers: &mut [usize; MAX_EXTERNAL_ALIASES],
    external_count: &mut usize,
) -> Option<OperandShape> {
    if register >= destination {
        return None;
    }
    if register < bundle_start {
        let external = match external_registers[..*external_count]
            .iter()
            .position(|candidate| *candidate == register)
        {
            Some(external) => external,
            None => {
                let external = *external_count;
                *external_registers.get_mut(external)? = register;
                *external_count = external.checked_add(1)?;
                external
            }
        };
        return u8::try_from(external).ok().map(OperandShape::External);
    }
    if register < lane_start {
        // This is a dependency on an earlier lane. Reordering such a range
        // would be observable, so it cannot be bundled.
        return None;
    }
    u8::try_from(register - lane_start)
        .ok()
        .map(OperandShape::Internal)
}

fn is_isomorphic_bundle(operations: &[NumberInstruction], bundle: BinaryBundle) -> bool {
    record_shape_validation();
    let mut reference = [None; MAX_BINARY_CHAIN_LENGTH];
    for lane in 0..bundle.lane_count {
        let Some(lane_offset) = lane.checked_mul(bundle.chain_length) else {
            return false;
        };
        let Some(lane_start) = bundle.start.checked_add(lane_offset) else {
            return false;
        };
        let Some(lane_end) = lane_start.checked_add(bundle.chain_length) else {
            return false;
        };
        let Some(instructions) = operations.get(lane_start..lane_end) else {
            return false;
        };
        let mut external_registers = [usize::MAX; MAX_EXTERNAL_ALIASES];
        let mut external_count = 0;
        for (step, instruction) in instructions.iter().enumerate() {
            let destination = lane_start + step;
            let NumberInstruction::Binary {
                operation,
                left,
                right,
            } = *instruction
            else {
                return false;
            };
            let Some(left) = operand_shape(
                left,
                destination,
                bundle.start,
                lane_start,
                &mut external_registers,
                &mut external_count,
            ) else {
                return false;
            };
            let Some(right) = operand_shape(
                right,
                destination,
                bundle.start,
                lane_start,
                &mut external_registers,
                &mut external_count,
            ) else {
                return false;
            };
            let shape = OperationShape {
                operation,
                left,
                right,
            };
            if lane == 0 {
                reference[step] = Some(shape);
            } else if reference[step] != Some(shape) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
thread_local! {
    static SHAPE_VALIDATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HEAP_STORAGE_PATH_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[inline]
fn record_shape_validation() {
    #[cfg(test)]
    SHAPE_VALIDATIONS.set(SHAPE_VALIDATIONS.get() + 1);
}

#[inline]
fn record_heap_storage_path_entry() {
    #[cfg(test)]
    HEAP_STORAGE_PATH_ENTRIES.set(HEAP_STORAGE_PATH_ENTRIES.get() + 1);
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct BundleScore {
    saved_dispatches: usize,
    // With equal dispatch savings, fewer bundles avoid executor-loop overhead.
    inverse_bundle_count: std::cmp::Reverse<usize>,
}

impl BundleScore {
    fn with_candidate(self, candidate: ValidatedCandidate) -> Option<Self> {
        Some(Self {
            saved_dispatches: self
                .saved_dispatches
                .checked_add(candidate.saved_dispatches)?,
            inverse_bundle_count: std::cmp::Reverse(self.inverse_bundle_count.0.checked_add(1)?),
        })
    }
}

fn has_minimum_binary_run(operations: &[NumberInstruction], dynamic_start: usize) -> bool {
    let mut run_length = 0;
    for operation in &operations[dynamic_start..] {
        if matches!(operation, NumberInstruction::Binary { .. }) {
            run_length += 1;
            if run_length >= MIN_BINARY_BUNDLE_OPERATIONS {
                return true;
            }
        } else {
            run_length = 0;
        }
    }
    false
}

/// Selects a deterministic, non-overlapping set of bundles with maximum total
/// dispatch savings. Detection is bounded by the dense-plan operation limit,
/// four lanes, and sixteen operations per lane.
pub(super) fn detect(operations: &[NumberInstruction], dynamic_start: usize) -> Vec<BinaryBundle> {
    if dynamic_start > operations.len() || !has_minimum_binary_run(operations, dynamic_start) {
        return Vec::new();
    }

    record_heap_storage_path_entry();
    let mut scores = vec![BundleScore::default(); operations.len() + 1];
    let mut choices = vec![None; operations.len()];
    let mut binary_run_length = 0;
    for start in (dynamic_start..operations.len()).rev() {
        if matches!(operations[start], NumberInstruction::Binary { .. }) {
            binary_run_length = (binary_run_length + 1).min(MAX_BINARY_BUNDLE_OPERATIONS);
        } else {
            binary_run_length = 0;
        }

        let mut best_score = scores[start + 1];
        let mut best_candidate = None;
        if binary_run_length >= MIN_BINARY_BUNDLE_OPERATIONS {
            let empty_candidate = ValidatedCandidate {
                bundle: BinaryBundle {
                    start: 0,
                    lane_count: 0,
                    chain_length: 0,
                },
                end: 0,
                saved_dispatches: 0,
            };
            let mut candidates = [empty_candidate; MAX_BINARY_BUNDLE_CANDIDATES];
            let mut candidate_count = 0;
            for lane_count in 2..=MAX_BINARY_BUNDLE_LANES {
                for chain_length in 1..=MAX_BINARY_CHAIN_LENGTH {
                    let bundle = BinaryBundle {
                        start,
                        lane_count,
                        chain_length,
                    };
                    let Some(operation_count) = bundle.operation_count() else {
                        continue;
                    };
                    if operation_count > binary_run_length {
                        continue;
                    }
                    let Some((end, saved_dispatches)) =
                        bundle.checked_layout(operations.len(), dynamic_start)
                    else {
                        continue;
                    };
                    if !is_isomorphic_bundle(operations, bundle) {
                        continue;
                    }
                    candidates[candidate_count] = ValidatedCandidate {
                        bundle,
                        end,
                        saved_dispatches,
                    };
                    candidate_count += 1;
                }
            }
            let candidates = &mut candidates[..candidate_count];
            candidates.sort_unstable_by_key(|candidate| {
                (
                    std::cmp::Reverse(candidate.saved_dispatches),
                    std::cmp::Reverse(candidate.bundle.operation_count().unwrap_or_default()),
                    std::cmp::Reverse(candidate.bundle.lane_count),
                    std::cmp::Reverse(candidate.bundle.chain_length),
                )
            });
            for candidate in candidates {
                let Some(score) = scores[candidate.end].with_candidate(*candidate) else {
                    continue;
                };
                if score > best_score {
                    best_score = score;
                    best_candidate = Some(*candidate);
                }
            }
        }
        scores[start] = best_score;
        choices[start] = best_candidate;
    }

    let mut bundles = Vec::new();
    let mut cursor = dynamic_start;
    while cursor < operations.len() {
        let Some(candidate) = choices[cursor] else {
            cursor += 1;
            continue;
        };
        bundles.push(candidate.bundle);
        cursor = candidate.end;
    }
    bundles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(local: usize) -> NumberInstruction {
        NumberInstruction::LoadLocal(local)
    }

    fn binary(operation: BinaryOp, left: usize, right: usize) -> NumberInstruction {
        NumberInstruction::Binary {
            operation,
            left,
            right,
        }
    }

    fn four_isomorphic_lanes() -> Vec<NumberInstruction> {
        let mut operations = (0..12).map(input).collect::<Vec<_>>();
        for lane in 0..4 {
            let lane_start = 12 + lane * 4;
            operations.push(binary(BinaryOp::Mul, lane * 2, 8));
            operations.push(binary(BinaryOp::Mul, lane * 2 + 1, 9));
            operations.push(binary(BinaryOp::Add, lane_start, lane_start + 1));
            operations.push(binary(BinaryOp::Add, lane_start + 2, 10));
        }
        operations
    }

    fn repeated_isomorphic_lanes(
        lane_count: usize,
        chain_length: usize,
    ) -> (Vec<NumberInstruction>, usize) {
        let dynamic_start = lane_count * 2 + 2;
        let shared_factor = lane_count * 2;
        let shared_bias = shared_factor + 1;
        let mut operations = (0..dynamic_start).map(input).collect::<Vec<_>>();
        for lane in 0..lane_count {
            let lane_start = dynamic_start + lane * chain_length;
            operations.push(binary(BinaryOp::Mul, lane * 2, shared_factor));
            for step in 1..chain_length {
                operations.push(binary(BinaryOp::Add, lane_start + step - 1, shared_bias));
            }
        }
        (operations, dynamic_start)
    }

    #[test]
    fn detects_four_independent_isomorphic_binary_chains() {
        let operations = four_isomorphic_lanes();
        assert_eq!(
            detect(&operations, 12),
            vec![BinaryBundle {
                start: 12,
                lane_count: 4,
                chain_length: 4,
            }]
        );
    }

    #[test]
    fn accepts_minimum_savings_two_lane_and_three_lane_boundaries() {
        for (lane_count, chain_length) in [(2, 4), (3, 2)] {
            let (operations, dynamic_start) = repeated_isomorphic_lanes(lane_count, chain_length);
            assert_eq!(
                detect(&operations, dynamic_start),
                vec![BinaryBundle {
                    start: dynamic_start,
                    lane_count,
                    chain_length,
                }]
            );
        }
    }

    #[test]
    fn rejects_layout_below_minimum_dispatch_savings() {
        let (operations, dynamic_start) = repeated_isomorphic_lanes(2, 3);
        let bundle = BinaryBundle {
            start: dynamic_start,
            lane_count: 2,
            chain_length: 3,
        };
        assert_eq!(bundle.saved_dispatches(), Some(3));
        assert_eq!(bundle.validated_end(&operations, dynamic_start), None);
        assert!(detect(&operations, dynamic_start).is_empty());
    }

    #[test]
    fn barrier_only_stream_skips_all_shape_validation_and_candidate_storage() {
        SHAPE_VALIDATIONS.set(0);
        HEAP_STORAGE_PATH_ENTRIES.set(0);
        let operations = (0..256)
            .map(|_| NumberInstruction::Unary {
                operation: qjs_ast::UnaryOp::Minus,
                value: 0,
            })
            .collect::<Vec<_>>();
        assert!(detect(&operations, 0).is_empty());
        assert_eq!(SHAPE_VALIDATIONS.get(), 0);
        assert_eq!(HEAP_STORAGE_PATH_ENTRIES.get(), 0);
    }

    #[test]
    fn rejects_shape_mismatch_cross_lane_dependencies_and_effect_boundaries() {
        let bundle = BinaryBundle {
            start: 12,
            lane_count: 4,
            chain_length: 4,
        };
        let mut operations = four_isomorphic_lanes();
        operations[22] = binary(BinaryOp::Sub, 20, 21);
        assert_eq!(bundle.validated_end(&operations, 12), None);

        let mut operations = four_isomorphic_lanes();
        operations[16] = binary(BinaryOp::Mul, 12, 8);
        assert_eq!(bundle.validated_end(&operations, 12), None);

        let mut operations = four_isomorphic_lanes();
        operations[18] = NumberInstruction::Unary {
            operation: qjs_ast::UnaryOp::Minus,
            value: 17,
        };
        assert_eq!(bundle.validated_end(&operations, 12), None);
    }

    #[test]
    fn chooses_the_non_overlapping_layout_with_maximum_total_savings() {
        let operations = four_isomorphic_lanes();
        let bundles = detect(&operations, 12);
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].saved_dispatches(), Some(12));
        assert_eq!(bundles[0].operation_count(), Some(16));
    }

    #[test]
    fn hostile_bundle_bounds_fail_closed() {
        let operations = four_isomorphic_lanes();
        for bundle in [
            BinaryBundle {
                start: 11,
                lane_count: 4,
                chain_length: 4,
            },
            BinaryBundle {
                start: 12,
                lane_count: 5,
                chain_length: 4,
            },
            BinaryBundle {
                start: 12,
                lane_count: 4,
                chain_length: 17,
            },
            BinaryBundle {
                start: usize::MAX,
                lane_count: 4,
                chain_length: 4,
            },
        ] {
            assert_eq!(bundle.validated_end(&operations, 12), None);
        }
    }
}
