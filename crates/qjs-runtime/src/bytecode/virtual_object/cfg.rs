use std::collections::BTreeSet;

use super::super::ir::Op;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ControlFlowGraph {
    pub(super) blocks: Vec<BasicBlock>,
    instruction_blocks: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BasicBlock {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) successors: Vec<usize>,
    pub(super) predecessors: Vec<usize>,
}

impl ControlFlowGraph {
    pub(super) fn empty() -> Self {
        Self {
            blocks: Vec::new(),
            instruction_blocks: Vec::new(),
        }
    }

    pub(super) fn build(code: &[Op]) -> Result<Self, ()> {
        if code.is_empty() {
            return Ok(Self::empty());
        }
        let mut leaders = BTreeSet::from([0]);
        for (ip, op) in code.iter().enumerate() {
            let target = match op {
                Op::Jump(target)
                | Op::JumpIfFalse(target)
                | Op::JumpIfTrue(target)
                | Op::JumpIfNotNullish(target)
                | Op::AbruptJump(target) => Some(*target),
                _ => None,
            };
            if let Some(target) = target {
                if target >= code.len() {
                    return Err(());
                }
                leaders.insert(target);
            }
            if (target.is_some() || is_terminal(op)) && ip + 1 < code.len() {
                leaders.insert(ip + 1);
            }
        }

        let leaders = leaders.into_iter().collect::<Vec<_>>();
        let mut blocks = leaders
            .iter()
            .enumerate()
            .map(|(index, start)| BasicBlock {
                start: *start,
                end: leaders.get(index + 1).copied().unwrap_or(code.len()),
                successors: Vec::new(),
                predecessors: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut instruction_blocks = vec![0; code.len()];
        for (block, range) in blocks.iter().enumerate() {
            instruction_blocks[range.start..range.end].fill(block);
        }

        for block in 0..blocks.len() {
            let last_ip = blocks[block].end - 1;
            let fallthrough = (block + 1 < blocks.len()).then_some(block + 1);
            let mut successors = match &code[last_ip] {
                Op::Jump(target) | Op::AbruptJump(target) => {
                    vec![instruction_blocks[*target]]
                }
                Op::JumpIfFalse(target) | Op::JumpIfTrue(target) | Op::JumpIfNotNullish(target) => {
                    let mut successors = vec![instruction_blocks[*target]];
                    if let Some(fallthrough) = fallthrough {
                        successors.push(fallthrough);
                    }
                    successors
                }
                op if is_terminal(op) => Vec::new(),
                _ => fallthrough.into_iter().collect(),
            };
            successors.sort_unstable();
            successors.dedup();
            blocks[block].successors = successors;
        }
        for block in 0..blocks.len() {
            let successors = blocks[block].successors.clone();
            for successor in successors {
                blocks[successor].predecessors.push(block);
            }
        }
        Ok(Self {
            blocks,
            instruction_blocks,
        })
    }
}

fn is_terminal(op: &Op) -> bool {
    matches!(op, Op::Return | Op::Throw | Op::ThrowReferenceError(_))
}
