//! Compile-time definite-Number dataflow for the owned loop nest.

use super::super::dense::{MAX_DENSE_LOCALS, NumberInstruction};
use super::ir::{NumericProgram, Radix2NestProof};

#[derive(Clone, Copy)]
pub(super) struct DefinednessInputs<'a> {
    pub(super) radix2: Radix2NestProof,
    pub(super) kernel_reads: &'a [usize],
    pub(super) kernel_writes: &'a [usize],
    pub(super) kernel_kills: &'a [usize],
    pub(super) outer_prelude: &'a NumericProgram,
    pub(super) middle_prelude: &'a NumericProgram,
    pub(super) middle_epilogue: &'a NumericProgram,
    pub(super) outer_epilogue: &'a NumericProgram,
}

#[derive(Clone, Copy)]
struct Region {
    reads: u64,
    writes: u64,
    kills: u64,
}

impl Region {
    fn from_program(program: &NumericProgram) -> Option<Self> {
        let reads = locals_mask(program.operations.iter().filter_map(
            |operation| match operation {
                NumberInstruction::LoadLocal(local) => Some(*local),
                _ => None,
            },
        ))?;
        let writes = locals_mask(program.writes.iter().map(|write| write.local))?;
        let invalidations = locals_mask(program.invalidations.iter().copied())?;
        Some(Self {
            reads,
            writes,
            kills: invalidations & !writes,
        })
    }

    fn backward(self, required_after: u64) -> Option<u64> {
        if required_after & self.kills != 0 {
            return None;
        }
        Some(self.reads | (required_after & !self.writes))
    }
}

pub(super) fn required_entry_mask(flow: DefinednessInputs<'_>) -> Option<u64> {
    const INNER_CONDITION: usize = 0;
    const KERNEL: usize = 1;
    const MIDDLE_EPILOGUE: usize = 2;
    const MIDDLE_PRELUDE: usize = 3;
    const OUTER_EPILOGUE: usize = 4;
    const OUTER_PRELUDE: usize = 5;

    let kernel_writes = locals_mask(flow.kernel_writes.iter().copied())?;
    let kernel = Region {
        reads: locals_mask(flow.kernel_reads.iter().copied())?,
        writes: kernel_writes,
        kills: locals_mask(flow.kernel_kills.iter().copied())? & !kernel_writes,
    };
    let middle_epilogue = Region::from_program(flow.middle_epilogue)?;
    let middle_prelude = Region::from_program(flow.middle_prelude)?;
    let outer_epilogue = Region::from_program(flow.outer_epilogue)?;
    let outer_prelude = Region::from_program(flow.outer_prelude)?;
    let inner_condition_reads = locals_mask([flow.radix2.index, flow.radix2.bound])?;
    let middle_condition_reads = locals_mask([flow.radix2.lane, flow.radix2.span])?;
    let outer_condition_reads = locals_mask([flow.radix2.span, flow.radix2.bound])?;

    let mut inputs = [0_u64; 6];
    loop {
        let previous = inputs;
        inputs[INNER_CONDITION] =
            previous[KERNEL] | previous[MIDDLE_EPILOGUE] | inner_condition_reads;
        inputs[KERNEL] = kernel.backward(previous[INNER_CONDITION])?;
        inputs[MIDDLE_EPILOGUE] = middle_epilogue.backward(
            previous[MIDDLE_PRELUDE] | previous[OUTER_EPILOGUE] | middle_condition_reads,
        )?;
        // The static radix-2 algebra proves `index = lane < span <= bound / 2`
        // after every future middle prelude, so that edge must execute one
        // kernel before it can reach the inner condition again. Only the trace
        // entry itself may take the zero-remaining-kernel edge.
        inputs[MIDDLE_PRELUDE] = middle_prelude.backward(previous[KERNEL])?;
        inputs[OUTER_EPILOGUE] =
            outer_epilogue.backward(previous[OUTER_PRELUDE] | outer_condition_reads)?;
        inputs[OUTER_PRELUDE] = outer_prelude.backward(previous[MIDDLE_PRELUDE])?;
        if inputs == previous {
            return Some(inputs[INNER_CONDITION]);
        }
    }
}

fn locals_mask(locals: impl IntoIterator<Item = usize>) -> Option<u64> {
    locals.into_iter().try_fold(0_u64, |mask, local| {
        (local < MAX_DENSE_LOCALS).then_some(mask | (1_u64 << local))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROOF: Radix2NestProof = Radix2NestProof {
        span: 0,
        bound: 1,
        lane: 2,
        index: 3,
    };

    fn empty_program() -> NumericProgram {
        NumericProgram {
            operations: Vec::new(),
            writes: Vec::new(),
            invalidations: Vec::new(),
        }
    }

    #[test]
    fn zero_remaining_kernel_path_keeps_epilogue_inputs_required_at_entry() {
        let empty = empty_program();
        let middle_epilogue = NumericProgram {
            operations: vec![NumberInstruction::LoadLocal(4)],
            writes: Vec::new(),
            invalidations: Vec::new(),
        };
        let required = required_entry_mask(DefinednessInputs {
            radix2: PROOF,
            kernel_reads: &[],
            kernel_writes: &[4],
            kernel_kills: &[],
            outer_prelude: &empty,
            middle_prelude: &empty,
            middle_epilogue: &middle_epilogue,
            outer_epilogue: &empty,
        })
        .expect("the zero-iteration path has a valid entry requirement");

        assert_ne!(required & (1 << 4), 0);
    }

    #[test]
    fn kernel_self_edge_rejects_a_local_killed_then_read_by_the_next_iteration() {
        let empty = empty_program();
        let kernel = Region {
            reads: 1 << 4,
            writes: 0,
            kills: 1 << 4,
        };
        let condition_reads = (1 << PROOF.index) | (1 << PROOF.bound);
        let first_iteration = kernel
            .backward(condition_reads)
            .expect("one kernel iteration can consume the entry value");
        assert_ne!(first_iteration & (1 << 4), 0);
        assert!(
            kernel.backward(first_iteration | condition_reads).is_none(),
            "the second K entry observes the kill from the first iteration"
        );
        assert!(
            required_entry_mask(DefinednessInputs {
                radix2: PROOF,
                kernel_reads: &[4],
                kernel_writes: &[],
                kernel_kills: &[4],
                outer_prelude: &empty,
                middle_prelude: &empty,
                middle_epilogue: &empty,
                outer_epilogue: &empty,
            })
            .is_none()
        );
    }

    #[test]
    fn a_write_only_kernel_temporary_is_not_required_before_the_kernel() {
        let empty = empty_program();
        let required = required_entry_mask(DefinednessInputs {
            radix2: PROOF,
            kernel_reads: &[],
            kernel_writes: &[4],
            kernel_kills: &[],
            outer_prelude: &empty,
            middle_prelude: &empty,
            middle_epilogue: &empty,
            outer_epilogue: &empty,
        })
        .expect("a write-only temporary is defined before any successor read");

        assert_eq!(required & (1 << 4), 0);
    }
}
