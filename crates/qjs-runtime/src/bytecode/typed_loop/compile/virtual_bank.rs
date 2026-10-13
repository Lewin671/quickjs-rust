//! Read-only primitive inputs from the selected stream's frame-owned bank.
//!
//! A physical virtual instruction owns one entry stack, even when translated
//! to several register operations. Intermediate operands are never invented
//! in its deopt site. All bank values are seeded before executing the region.

use super::{Builder, Op, Origin, Typed, TypedOp, admitted_binary};

impl Builder<'_> {
    fn virtual_input(&mut self, slot: usize) -> Option<u16> {
        let slot = u32::try_from(slot).ok()?;
        // Reject every bank mutation/lifetime effect in this region, including
        // unreachable paths. Aliases and layout validity are the lowerer's
        // existing CFG proof; contents must also stay invariant here.
        if self.code[self.header..=self.backedge].iter().any(|op| {
            matches!(
                op,
                Op::StoreVirtualValue { .. }
                    | Op::InitVirtualObject { .. }
                    | Op::InitVirtualConstants { .. }
                    | Op::InitVirtualFunction { .. }
                    | Op::CallVirtualFunction { .. }
            )
        }) {
            return None;
        }
        if let Some((register, _)) = self.virtual_slots.iter().find(|(_, input)| *input == slot) {
            return Some(*register);
        }
        let register = self.fresh()?;
        self.virtual_slots.push((register, slot));
        Some(register)
    }

    pub(super) fn compile_virtual_op(&mut self, op: &Op) -> Option<()> {
        match *op {
            Op::LoadVirtualValue { slot, discard } => {
                for _ in 0..discard {
                    self.stack.pop()?;
                }
                let src = self.virtual_input(slot)?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Move { dst, src });
                self.push(dst, Origin::Computed);
            }
            Op::LoadVirtualLength { length, discard } => {
                for _ in 0..discard {
                    self.stack.pop()?;
                }
                let src = self.constant_register(Typed::Number(length as f64))?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Move { dst, src });
                self.push(dst, Origin::Computed);
            }
            Op::LoadVirtualBinary {
                left,
                right,
                op,
                skip,
            } => {
                if !admitted_binary(op) {
                    return None;
                }
                let left = self.virtual_input(left)?;
                let right = self.virtual_input(right)?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Binary {
                    dst,
                    op,
                    left,
                    right,
                });
                self.push(dst, Origin::Computed);
                self.pending_skip = skip;
            }
            Op::LoadVirtualNumber { value, skip } => {
                let src = self.constant_register(Typed::Number(value))?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Move { dst, src });
                self.push(dst, Origin::Computed);
                self.pending_skip = skip;
            }
            Op::GuardVirtualObject => {}
            _ => return None,
        }
        Some(())
    }
}
