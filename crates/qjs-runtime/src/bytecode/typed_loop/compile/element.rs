//! Lowering of `receiver[key] = value` element assignments inside a region.
//!
//! The bytecode for an element assignment spills its receiver, key and value
//! into compiler temporaries before the `SetProp`; this lowering elides those
//! temporaries and emits one dense write, which is what lets `a[i] = expr`
//! run without an operand stack. It is the largest single lowering in the
//! builder, so it lives in its own file next to the builder it extends.

use super::{
    Builder, Origin, TypedOp, expression_has_control_flow, scalar_expression_may_write_or_branch,
};
use crate::bytecode::ir::Op;

impl Builder<'_> {
    pub(super) fn compile_element_assignment(&mut self, ip: usize) -> Option<Option<usize>> {
        let code = &self.bytecode.code;
        let (Some(Op::LoadLocal(receiver)), Some(Op::StoreLocal(receiver_temp))) =
            (code.get(ip), code.get(ip + 1))
        else {
            return Some(None);
        };
        let (receiver, receiver_temp) = (*receiver, *receiver_temp);
        if receiver == receiver_temp {
            return Some(None);
        }
        // The dense write below resolves its receiver once per region entry,
        // which is only sound for a slot the region never reassigns. A
        // reassigned receiver takes the ordinary per-instruction path, whose
        // computed write reads the boxed register on every store.
        if self.writes_in_region(receiver) > 0 {
            return Some(None);
        }
        // Find `StoreLocal(index_temp)` followed later by the tail
        // `StoreLocal(value_temp); LoadLocal(receiver_temp); LoadLocal(index_temp);
        // LoadLocal(value_temp); SetProp`. The key expression may contain any
        // side-effect-free scalar bytecode that this tier already admits; a
        // simple `LoadLocal(index); StoreLocal(index_temp)` is its smallest
        // instance.
        let mut cursor = ip + 2;
        let (index_store, index_temp, value_store, value_temp) = loop {
            if cursor + 4 > self.backedge {
                return Some(None);
            }
            if let (
                Some(Op::StoreLocal(value_temp)),
                Some(Op::LoadLocal(first)),
                Some(Op::LoadLocal(second)),
                Some(Op::LoadLocal(third)),
                Some(Op::SetProp { .. }),
            ) = (
                code.get(cursor),
                code.get(cursor + 1),
                code.get(cursor + 2),
                code.get(cursor + 3),
                code.get(cursor + 4),
            ) && *first == receiver_temp
                && *third == *value_temp
            {
                let index_temp = *second;
                let index_store = (ip + 2..cursor).rev().find(|candidate| {
                    matches!(code.get(*candidate), Some(Op::StoreLocal(slot)) if *slot == index_temp)
                });
                if let Some(index_store) = index_store {
                    break (index_store, index_temp, cursor, *value_temp);
                }
            }
            cursor += 1;
        };
        if receiver_temp == index_temp || receiver_temp == value_temp || index_temp == value_temp {
            return Some(None);
        }
        for temp in [receiver_temp, index_temp, value_temp] {
            if !self.bytecode.local_is_compiler_temporary(temp)
                || self.slot_is_read_outside_region(temp)
                || self.writes_in_region(temp) != 1
            {
                return Some(None);
            }
        }
        // Compile the key and value expressions with their compiler
        // temporaries elided. Copy the computed key before the value
        // expression: JavaScript evaluates it first, so a later local write in
        // the value expression must not change the eventual dense-array index.
        // The value expression must be branch-free: a join inside it would
        // need its own bookkeeping relative to the elided temporaries. That is
        // decided here, by inspection, rather than part-way through lowering.
        //
        // The distinction matters more than it looks. Saying "cannot lower
        // this" after emitting operations has to fail with `None`, which takes
        // the *whole region* down; saying it beforehand can return
        // `Some(None)` and let the ordinary per-instruction path try. A
        // `table[key] = (table[key] || 0) + 1` is exactly that case: this
        // idiom only lowers to a dense array write, so a dictionary receiver
        // was never going to be expressible here, and its `||` was aborting
        // the region before the ordinary path ever saw it.
        for probe in index_store + 1..value_store {
            if expression_has_control_flow(code.get(probe)?) {
                return Some(None);
            }
        }
        let receiver_slot = u32::try_from(receiver).ok()?;
        let index_register = self.compile_pure_scalar_expression(ip + 2, index_store)?;
        let index_copy = self.fresh()?;
        self.emit(TypedOp::Move {
            dst: index_copy,
            src: index_register,
        });
        let mut inner = index_store + 1;
        while inner < value_store {
            if let Some(next) = self.compile_element_assignment(inner)? {
                inner = next;
                continue;
            }
            let op = self.bytecode.code.get(inner)?;
            // A branch inside the value expression would need its own join
            // bookkeeping relative to the elided temporaries.
            if expression_has_control_flow(op) {
                return None;
            }
            self.compile_op(op, inner)?;
            inner += 1 + std::mem::take(&mut self.pending_skip);
        }
        if inner != value_store {
            return None;
        }
        let (value, _) = self.pop()?;
        let receiver = self.receiver_index(receiver_slot)?;
        self.emit(TypedOp::DenseWrite {
            receiver,
            index: index_copy,
            value,
        });
        // `SetProp` leaves the assigned value on the operand stack.
        let dst = self.slot_scalar()?;
        self.emit(TypedOp::Move { dst, src: value });
        self.push(dst, Origin::Computed);
        Some(Some(value_store + 5))
    }

    /// Lowers the scalar key expression of an element assignment. This region
    /// intentionally excludes writes and branches: its deoptimization sites
    /// are anchored at the surrounding assignment, so replaying it must not
    /// duplicate an observable effect before the final `DenseWrite`.
    pub(super) fn compile_pure_scalar_expression(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<u16> {
        let depth = self.stack.len();
        let mut cursor = start;
        while cursor < end {
            let op = self.bytecode.code.get(cursor)?;
            if scalar_expression_may_write_or_branch(op) {
                return None;
            }
            self.compile_op(op, cursor)?;
            cursor += 1 + std::mem::take(&mut self.pending_skip);
        }
        if cursor != end || self.stack.len() != depth + 1 {
            return None;
        }
        self.pop().map(|(register, _)| register)
    }
}
