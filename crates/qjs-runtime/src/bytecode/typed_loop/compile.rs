//! Lowering a loop region's bytecode into a register program.
//!
//! The walk is linear from the header to the backedge, interpreting the operand
//! stack abstractly. A stack entry always lives in the register with its depth's
//! index — in the scalar file or the boxed one, whichever its class names — so
//! two paths that meet at a join agree on where every value is without phi
//! nodes, and a backward jump inside the region (a nested loop) needs nothing
//! beyond checking that the state it delivers matches the state recorded there.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use std::{cell::OnceCell, rc::Rc};

use super::super::ir::{Bytecode, Op};
use super::{
    Class, DeoptSite, MAX_REGION_OPS, MAX_REGISTERS, MAX_STACK_DEPTH, Typed, TypedLoopProgram,
    TypedOp,
};
use crate::Value;

/// Compiles every loop region of `bytecode` that the whitelist admits.
pub(crate) fn compile_all(bytecode: &Bytecode) -> Vec<TypedLoopProgram> {
    bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(ip, op)| match op {
            Op::Jump(target) if *target < ip => Some((*target, ip)),
            // A fused increment carries the backedge of the loop it closes, and
            // the interpreter identifies that loop by the instruction the skip
            // reaches — the same pair every tier is offered.
            Op::IncrementLocal {
                slot: _,
                skip,
                jump: Some(target),
            } if *target < ip => Some((*target, ip + skip)),
            _ => None,
        })
        .filter_map(|(header, backedge)| compile(bytecode, header, backedge))
        .collect()
}

fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<TypedLoopProgram> {
    let code = &bytecode.code;
    if backedge.checked_sub(header)? > MAX_REGION_OPS {
        return None;
    }
    // Whether a frame slot holds a number or an object is not visible in the
    // bytecode, so compile, learn which slots must be boxed, and recompile. The
    // set only grows, so this converges.
    let mut boxed_slots: Vec<u32> = Vec::new();
    let builder = loop {
        let mut builder = Builder::new(bytecode, header, backedge, boxed_slots.clone());
        if builder.compile().is_some() {
            break builder;
        }
        let discovered: Vec<u32> = builder
            .discovered_boxed
            .iter()
            .copied()
            .filter(|slot| !boxed_slots.contains(slot))
            .collect();
        if discovered.is_empty() {
            return None;
        }
        boxed_slots.extend(discovered);
    };
    let Builder {
        ops,
        sites,
        site_entries,
        next_register,
        local_slots,
        written_locals,
        receiver_slots,
        global_reads,
        sloppy_global_writes,
        next_boxed,
        boxed_locals,
        written_boxed_locals,
        boxed_global_reads,
        names,
        constants,
        boxed_constants,
        cache_count,
        ..
    } = builder;
    // A region that never leaves through its header test cannot be entered
    // safely, and one with no operations is not worth a program.
    if ops.is_empty() || !ops.iter().any(|op| matches!(op, TypedOp::Exit { .. })) {
        return None;
    }
    // What this tier removes is dispatch and boxing around *scalar* arithmetic.
    // Where the work is the property protocol instead, the interpreter's own
    // inline caches are already as good, and running such a region natively
    // measured neutral to slower — so a region whose operations are mostly boxed
    // stays with the interpreter.
    let boxed_ops = ops
        .iter()
        .filter(|op| {
            matches!(
                op,
                TypedOp::GetNamed { .. }
                    | TypedOp::SetNamed { .. }
                    | TypedOp::ElementRead { .. }
                    | TypedOp::MoveBoxed { .. }
                    | TypedOp::Box { .. }
                    | TypedOp::Unbox { .. }
            )
        })
        .count();
    if boxed_ops * 3 > ops.len() {
        return None;
    }
    debug_assert!(matches!(
        code.get(backedge.min(code.len() - 1)),
        Some(Op::Jump(_) | Op::IncrementLocal { .. } | Op::Pop | Op::LoadConst(_))
    ));
    Some(TypedLoopProgram {
        header,
        backedge,
        ops,
        sites,
        site_entries,
        register_count: next_register,
        local_slots,
        written_locals,
        receiver_slots,
        global_reads,
        sloppy_global_writes,
        boxed_count: next_boxed,
        boxed_locals,
        written_boxed_locals,
        boxed_global_reads,
        names,
        constant_registers: constants,
        boxed_constant_registers: boxed_constants,
        cache_count,
        scratch_pool: OnceCell::new(),
    })
}

/// Where a register's value came from, so a property read can recognize a
/// receiver that is really a frame slot.
#[derive(Clone, Copy, PartialEq)]
enum Origin {
    Computed,
    Local(u32),
}

struct Builder<'a> {
    bytecode: &'a Bytecode,
    header: usize,
    backedge: usize,
    ops: Vec<TypedOp>,
    sites: Vec<DeoptSite>,
    site_entries: Vec<(Class, u16)>,
    /// Resume information the next emitted operation inherits.
    site: DeoptSite,
    next_register: usize,
    /// Register holding each referenced frame slot, keyed by slot.
    local_slots: Vec<(u16, u32)>,
    written_locals: Vec<(u16, u32)>,
    receiver_slots: Vec<u32>,
    global_reads: Vec<(u16, String)>,
    sloppy_global_writes: Vec<(u32, String)>,
    next_boxed: usize,
    boxed_locals: Vec<(u16, u32)>,
    written_boxed_locals: Vec<u16>,
    boxed_global_reads: Vec<(u16, String)>,
    names: Vec<Rc<str>>,
    cache_count: usize,
    /// Slots this pass treats as boxed, and slots the pass discovered must be.
    boxed_slots: Vec<u32>,
    discovered_boxed: Vec<u32>,
    /// Abstract operand stack of (register, class, origin).
    stack: Vec<(u16, Class, Origin)>,
    /// Abstract stack recorded for each bytecode index reached by a jump. Two
    /// paths that meet must agree on depth, class, and origin — registers follow
    /// from depth — so a join that disagrees declines instead of miscompiling.
    states: Vec<Option<Vec<(u16, Class, Origin)>>>,
    /// Registers seeded once with a constant, as (register, value).
    constants: Vec<(u16, Typed)>,
    /// Boxed registers seeded once with a constant.
    boxed_constants: Vec<(u16, Value)>,
    /// Whether each instruction in the region is the target of a jump.
    is_target: Vec<bool>,
    /// Instructions the last compiled operation subsumed, which the walk skips.
    pending_skip: usize,
    /// Whether the walk is past an unconditional jump, so the next instruction
    /// is reachable only through a recorded join state.
    unreachable: bool,
    /// Bytecode index -> program index, for patching jumps.
    program_index: Vec<Option<u32>>,
    pending_jumps: Vec<(usize, usize)>,
}

impl<'a> Builder<'a> {
    fn new(bytecode: &'a Bytecode, header: usize, backedge: usize, boxed_slots: Vec<u32>) -> Self {
        let span = backedge - header + 1;
        Self {
            bytecode,
            header,
            backedge,
            ops: Vec::new(),
            sites: Vec::new(),
            site_entries: Vec::new(),
            site: DeoptSite {
                ip: header as u32,
                start: 0,
                len: 0,
            },
            next_register: MAX_STACK_DEPTH,
            local_slots: Vec::new(),
            written_locals: Vec::new(),
            receiver_slots: Vec::new(),
            global_reads: Vec::new(),
            sloppy_global_writes: Vec::new(),
            next_boxed: MAX_STACK_DEPTH,
            boxed_locals: Vec::new(),
            written_boxed_locals: Vec::new(),
            boxed_global_reads: Vec::new(),
            names: Vec::new(),
            cache_count: 0,
            constants: Vec::new(),
            boxed_constants: Vec::new(),
            is_target: is_target(bytecode, header, backedge),
            boxed_slots,
            discovered_boxed: Vec::new(),
            stack: Vec::new(),
            pending_skip: 0,
            unreachable: false,
            states: vec![None; span],
            program_index: vec![None; span],
            pending_jumps: Vec::new(),
        }
    }

    fn fresh(&mut self) -> Option<u16> {
        if self.next_register >= MAX_REGISTERS {
            return None;
        }
        let register = u16::try_from(self.next_register).ok()?;
        self.next_register += 1;
        Some(register)
    }

    fn local_register(&mut self, slot: usize) -> Option<u16> {
        let slot = u32::try_from(slot).ok()?;
        if let Some((register, _)) = self
            .local_slots
            .iter()
            .find(|(_, candidate)| *candidate == slot)
        {
            return Some(*register);
        }
        let register = self.fresh()?;
        self.local_slots.push((register, slot));
        Some(register)
    }

    fn note_write(&mut self, register: u16, slot: usize) -> Option<()> {
        let slot = u32::try_from(slot).ok()?;
        if !self.written_locals.contains(&(register, slot)) {
            self.written_locals.push((register, slot));
        }
        Some(())
    }

    /// Redirects the operation that just computed `register` so it writes `dst`
    /// instead, which removes the copy a store would otherwise need. Only valid
    /// when nothing else still refers to `register` and the caller emits no
    /// further operation for this instruction: the operand-stack entry naming
    /// `register` disappears with the copy.
    fn fold_into_destination(&mut self, register: u16, dst: u16) -> bool {
        if usize::from(register) >= MAX_STACK_DEPTH
            || self
                .stack
                .iter()
                .any(|(candidate, class, _)| *candidate == register && *class == Class::Scalar)
        {
            return false;
        }
        let Some(last) = self.ops.last_mut() else {
            return false;
        };
        let produced = match last {
            TypedOp::Binary { dst, .. }
            | TypedOp::Unary { dst, .. }
            | TypedOp::Update { dst, .. }
            | TypedOp::ToNumeric { dst, .. }
            | TypedOp::DenseRead { dst, .. }
            | TypedOp::CallNumericNative { dst, .. }
            | TypedOp::Move { dst, .. } => dst,
            _ => return false,
        };
        if *produced != register {
            return false;
        }
        *produced = dst;
        true
    }

    /// How many instructions in the region assign `slot`.
    fn writes_in_region(&self, slot: usize) -> usize {
        self.bytecode.code[self.header..=self.backedge.min(self.bytecode.code.len() - 1)]
            .iter()
            .filter(|op| match op {
                Op::StoreLocal(candidate)
                | Op::AssignLocal(candidate)
                | Op::ClearLocal(candidate)
                | Op::StoreLocalOrGlobalSloppy {
                    slot: candidate, ..
                }
                | Op::IncrementLocal {
                    slot: candidate, ..
                }
                | Op::CopyLocal { to: candidate, .. } => *candidate == slot,
                Op::BinaryAssignLocals { target, stores, .. } => {
                    *target == slot || stores.contains(&slot)
                }
                _ => false,
            })
            .count()
    }

    /// Whether any instruction in the region reads `slot`.
    fn reads_in_region(&self, slot: usize) -> bool {
        self.bytecode.code[self.header..=self.backedge.min(self.bytecode.code.len() - 1)]
            .iter()
            .any(|op| match op {
                Op::LoadLocal(candidate)
                | Op::LoadLocalOrUndefined(candidate)
                | Op::IncrementLocal {
                    slot: candidate, ..
                }
                | Op::CopyLocal {
                    from: candidate, ..
                } => *candidate == slot,
                Op::CompareLocalsJumpFalse { left, right, .. } => *left == slot || *right == slot,
                Op::GetPropNamed { cache, .. } => cache.local_slot() == Some(slot),
                _ => false,
            })
    }

    /// Register the next pushed value must be computed into: its depth's index
    /// in the scalar file.
    fn slot_scalar(&self) -> Option<u16> {
        (self.stack.len() < MAX_STACK_DEPTH).then_some(self.stack.len() as u16)
    }

    /// Register the next pushed value must be computed into: its depth's index
    /// in the boxed file.
    fn slot_boxed(&self) -> Option<u16> {
        self.slot_scalar()
    }

    /// Pushes an operand. A computed value is pushed in its depth's register;
    /// a value that already lives somewhere stable — a frame slot's register, a
    /// constant's — is pushed by reference, and copied into its depth's register
    /// only when something is about to overwrite it or a join needs it there.
    fn push(&mut self, register: u16, origin: Origin) {
        self.stack.push((register, Class::Scalar, origin));
    }

    fn push_boxed(&mut self, register: u16, origin: Origin) {
        self.stack.push((register, Class::Boxed, origin));
    }

    /// Copies every operand that names `register` into its own depth's register,
    /// so a write to `register` cannot change an operand's value.
    ///
    /// Working from the top down is what makes this safe: an operand names
    /// either its own depth, a lower depth (only `Dup` produces that), or a
    /// register outside the stack range, so writing a depth's register can never
    /// disturb an operand that has not been copied yet.
    fn spill(&mut self, register: u16, class: Class) -> Option<()> {
        for depth in (0..self.stack.len()).rev() {
            let (current, current_class, origin) = self.stack[depth];
            if current != register || current_class != class {
                continue;
            }
            let dst = u16::try_from(depth).ok()?;
            if dst == register {
                continue;
            }
            self.emit(match class {
                Class::Scalar => TypedOp::Move { dst, src: register },
                Class::Boxed => TypedOp::MoveBoxed { dst, src: register },
            });
            self.stack[depth] = (dst, class, origin);
        }
        Some(())
    }

    /// Copies every operand into its own depth's register, which is where both
    /// paths of a join agree to find it.
    fn normalize(&mut self) -> Option<()> {
        for depth in (0..self.stack.len()).rev() {
            let (register, class, origin) = self.stack[depth];
            let dst = u16::try_from(depth).ok()?;
            if register == dst {
                continue;
            }
            self.emit(match class {
                Class::Scalar => TypedOp::Move { dst, src: register },
                Class::Boxed => TypedOp::MoveBoxed { dst, src: register },
            });
            self.stack[depth] = (dst, class, origin);
        }
        Some(())
    }

    /// Register holding `value` for the whole run, allocated once and seeded at
    /// entry, so a constant costs nothing per iteration.
    fn constant_register(&mut self, value: Typed) -> Option<u16> {
        if let Some((register, _)) = self
            .constants
            .iter()
            .find(|(_, candidate)| *candidate == value)
        {
            return Some(*register);
        }
        let register = self.fresh()?;
        self.constants.push((register, value));
        Some(register)
    }

    /// Same for a constant only a boxed register can hold.
    fn boxed_constant_register(&mut self, value: &Value) -> Option<u16> {
        if let Some((register, _)) = self
            .boxed_constants
            .iter()
            .find(|(_, candidate)| candidate == value)
        {
            return Some(*register);
        }
        let register = self.fresh_boxed()?;
        self.boxed_constants.push((register, value.clone()));
        Some(register)
    }

    /// Pops one operand as a scalar, narrowing a boxed register on the way.
    fn pop(&mut self) -> Option<(u16, Origin)> {
        let (register, class, origin) = self.stack.pop()?;
        match class {
            Class::Scalar => Some((register, origin)),
            Class::Boxed => {
                let dst = self.fresh()?;
                self.emit(TypedOp::Unbox { dst, src: register });
                Some((dst, origin))
            }
        }
    }

    /// Pops one operand as a boxed value, widening a scalar register.
    ///
    /// A scalar register that came straight from a frame slot cannot be widened
    /// soundly — the slot may hold an object this pass modelled as a number — so
    /// the slot is recorded and the region recompiled with it boxed.
    fn pop_boxed(&mut self) -> Option<(u16, Origin)> {
        let (register, class, origin) = self.stack.pop()?;
        match class {
            Class::Boxed => Some((register, origin)),
            Class::Scalar => {
                if let Origin::Local(slot) = origin {
                    self.discovered_boxed.push(slot);
                    return None;
                }
                let dst = self.fresh_boxed()?;
                self.emit(TypedOp::Box { dst, src: register });
                Some((dst, origin))
            }
        }
    }

    fn fresh_boxed(&mut self) -> Option<u16> {
        if self.next_boxed >= MAX_REGISTERS {
            return None;
        }
        let register = u16::try_from(self.next_boxed).ok()?;
        self.next_boxed += 1;
        Some(register)
    }

    /// Boxed register mirroring frame slot `slot`, allocating one if new.
    fn boxed_local_register(&mut self, slot: usize) -> Option<u16> {
        let slot = u32::try_from(slot).ok()?;
        if let Some((register, _)) = self
            .boxed_locals
            .iter()
            .find(|(_, candidate)| *candidate == slot)
        {
            return Some(*register);
        }
        let register = self.fresh_boxed()?;
        self.boxed_locals.push((register, slot));
        Some(register)
    }

    /// Boxed register for a receiver the bytecode names as a frame slot rather
    /// than pushing. A slot this pass modelled as scalar has to be boxed first,
    /// so it is recorded and the region recompiled.
    fn boxed_receiver(&mut self, slot: usize) -> Option<u16> {
        if !self.slot_is_boxed(slot) {
            self.discovered_boxed.push(u32::try_from(slot).ok()?);
            return None;
        }
        self.boxed_local_register(slot)
    }

    fn slot_is_boxed(&self, slot: usize) -> bool {
        u32::try_from(slot).is_ok_and(|slot| self.boxed_slots.contains(&slot))
    }

    fn name_index(&mut self, name: &Rc<str>) -> Option<u16> {
        if let Some(index) = self
            .names
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, name))
        {
            return u16::try_from(index).ok();
        }
        self.names.push(Rc::clone(name));
        u16::try_from(self.names.len() - 1).ok()
    }

    fn next_cache(&mut self) -> Option<u16> {
        let index = u16::try_from(self.cache_count).ok()?;
        self.cache_count += 1;
        Some(index)
    }

    fn emit(&mut self, op: TypedOp) {
        self.ops.push(op);
        self.sites.push(self.site);
    }

    /// Records that the operations emitted from now on belong to the bytecode
    /// instruction at `ip`, which starts from the current abstract stack.
    fn open_site(&mut self, ip: usize) -> Option<()> {
        let start = u32::try_from(self.site_entries.len()).ok()?;
        let len = u8::try_from(self.stack.len()).ok()?;
        for &(register, class, _) in &self.stack {
            self.site_entries.push((class, register));
        }
        self.site = DeoptSite {
            ip: u32::try_from(ip).ok()?,
            start,
            len,
        };
        Some(())
    }

    fn compile(&mut self) -> Option<()> {
        let code = &self.bytecode.code;
        let mut ip = self.header;
        while ip <= self.backedge {
            let offset = ip - self.header;
            // Code after an unconditional jump runs only when something jumps to
            // it, so it starts from that jump's recorded stack rather than from
            // the fall-through one. With no recorded state it is unreachable and
            // the region declines rather than guess.
            if self.unreachable {
                let state = self.states[offset].clone()?;
                self.stack = state;
                self.unreachable = false;
            }
            if self.is_target[offset] {
                self.normalize()?;
            }
            // A join must agree with every path that reaches it on depth and on
            // register class — the register itself follows from the depth. Only
            // provenance may differ, and there the join keeps the weaker one.
            match self.states[offset].clone() {
                Some(state) => {
                    let merged = merge_states(&state, &self.stack)?;
                    self.stack = merged.clone();
                    self.states[offset] = Some(merged);
                }
                None => self.states[offset] = Some(self.stack.clone()),
            }
            self.program_index[offset] = u32::try_from(self.ops.len()).ok();
            self.open_site(ip)?;
            if let Some(next) = self.compile_element_assignment(ip)? {
                ip = next;
                continue;
            }
            let op = code.get(ip)?;
            self.compile_op(op, ip)?;
            ip += 1 + std::mem::take(&mut self.pending_skip);
        }
        // Patch the forward jumps now that every program index is known.
        for (program_slot, target_ip) in std::mem::take(&mut self.pending_jumps) {
            let offset = target_ip.checked_sub(self.header)?;
            let target = (*self.program_index.get(offset)?)?;
            match &mut self.ops[program_slot] {
                TypedOp::JumpIfFalsy { target: slot, .. } | TypedOp::Jump { target: slot } => {
                    *slot = target;
                }
                _ => return None,
            }
        }
        Some(())
    }

    /// Matches the compiler-temporary sequence for `receiver[key] = value`,
    /// compiles the scalar key and value expressions in between, and matches
    /// the `SetProp` tail. Returns the instruction after the tail.
    ///
    /// The idiom has to be recognized as a unit because its receiver and key
    /// travel through compiler temporaries, and a property key is not something
    /// an unboxed scalar register can hold. The temporaries are bypassed
    /// entirely, which is only sound while nothing outside the region reads
    /// them — checked here.
    fn compile_element_assignment(&mut self, ip: usize) -> Option<Option<usize>> {
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
        let receiver_slot = u32::try_from(receiver).ok()?;
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
    fn compile_pure_scalar_expression(&mut self, start: usize, end: usize) -> Option<u16> {
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

    /// Boxed register seeded from the global binding `name`, allocating one if
    /// new.
    fn global_register(&mut self, name: &str) -> Option<u16> {
        if let Some((register, _)) = self
            .boxed_global_reads
            .iter()
            .find(|(_, candidate)| candidate == name)
        {
            return Some(*register);
        }
        let register = self.fresh_boxed()?;
        self.boxed_global_reads.push((register, name.to_owned()));
        Some(register)
    }

    /// Whether the region contains an instruction that could write `name`,
    /// which would make a hoisted read of that same binding observably stale.
    fn region_writes_global_named(&self, name: &str) -> bool {
        self.bytecode.code[self.header..=self.backedge]
            .iter()
            .any(|op| match op {
                Op::StoreGlobalStrict(candidate) | Op::DefineGlobalVar(candidate) => {
                    candidate == name
                }
                Op::StoreGlobalSloppy {
                    name: candidate, ..
                }
                | Op::StoreLocalOrGlobalSloppy {
                    name: candidate, ..
                }
                | Op::AppendStringLiteralGlobal {
                    name: candidate, ..
                } => candidate == name,
                _ => false,
            })
    }

    /// A named-property write could target `globalThis` and mutate a hoisted
    /// global binding. A region with a sloppy fallback sink therefore keeps
    /// those writes out of this tier; dense array writes remain separately
    /// guarded and cannot alter a global object's binding descriptors.
    fn region_writes_a_sloppy_global(&self) -> bool {
        self.bytecode.code[self.header..=self.backedge]
            .iter()
            .any(|op| matches!(op, Op::StoreLocalOrGlobalSloppy { .. }))
    }

    /// Returns the frame slot for an unresolved sloppy binding only when this
    /// bytecode owns the compiler-emitted fallback slot for the same name.
    fn sloppy_global_fallback_slot(&self, name: &str) -> Option<usize> {
        let slot = self.bytecode.local_slot(name)?;
        let local = self.bytecode.locals.get(slot)?;
        (local.name == name && local.sloppy_global_fallback && !local.compiler_temporary)
            .then_some(slot)
    }

    /// Index of the prepared sink for a sloppy fallback write, adding it once
    /// per slot/name pair. The runtime validates the dynamic binding and
    /// property identities before entering the program.
    fn sloppy_global_write_index(&mut self, slot: usize, name: &str) -> Option<u16> {
        if self.sloppy_global_fallback_slot(name) != Some(slot) {
            return None;
        }
        let slot = u32::try_from(slot).ok()?;
        if let Some(index) =
            self.sloppy_global_writes
                .iter()
                .position(|(candidate_slot, candidate_name)| {
                    *candidate_slot == slot && candidate_name == name
                })
        {
            return u16::try_from(index).ok();
        }
        self.sloppy_global_writes.push((slot, name.to_owned()));
        u16::try_from(self.sloppy_global_writes.len() - 1).ok()
    }

    /// Index of `slot` in the program's receiver list, adding it if new.
    fn receiver_index(&mut self, slot: u32) -> Option<u16> {
        if let Some(index) = self
            .receiver_slots
            .iter()
            .position(|candidate| *candidate == slot)
        {
            return u16::try_from(index).ok();
        }
        self.receiver_slots.push(slot);
        u16::try_from(self.receiver_slots.len() - 1).ok()
    }

    /// Whether any instruction outside the loop region reads or writes `slot`.
    fn slot_is_read_outside_region(&self, slot: usize) -> bool {
        self.bytecode.code.iter().enumerate().any(|(ip, op)| {
            if ip >= self.header && ip <= self.backedge {
                return false;
            }
            matches!(
                op,
                Op::LoadLocal(candidate)
                    | Op::LoadLocalOrUndefined(candidate)
                    | Op::StoreLocal(candidate)
                    | Op::AssignLocal(candidate)
                    | Op::ClearLocal(candidate)
                    if *candidate == slot
            )
        })
    }

    fn compile_op(&mut self, op: &Op, ip: usize) -> Option<()> {
        match op {
            Op::LoadConst(index) => {
                let constant = self.bytecode.constants.get(*index)?;
                match Typed::from_value(constant) {
                    Some(value) => {
                        let register = self.constant_register(value)?;
                        self.push(register, Origin::Computed);
                    }
                    None => {
                        let constant = constant.clone();
                        let register = self.boxed_constant_register(&constant)?;
                        self.push_boxed(register, Origin::Computed);
                    }
                }
            }
            Op::LoadLocal(slot) => {
                let origin = Origin::Local(u32::try_from(*slot).ok()?);
                if self.slot_is_boxed(*slot) {
                    let register = self.boxed_local_register(*slot)?;
                    self.push_boxed(register, origin);
                } else {
                    let register = self.local_register(*slot)?;
                    self.push(register, origin);
                }
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                if self.slot_is_boxed(*slot) {
                    let (src, _) = self.pop_boxed()?;
                    let dst = self.boxed_local_register(*slot)?;
                    self.spill(dst, Class::Boxed)?;
                    self.emit(TypedOp::MoveBoxed { dst, src });
                    if !self.written_boxed_locals.contains(&dst) {
                        self.written_boxed_locals.push(dst);
                    }
                } else {
                    // A boxed value stored into a slot this pass treats as
                    // scalar means the pass guessed wrong; record it so the next
                    // pass boxes the slot.
                    if matches!(self.stack.last(), Some((_, Class::Boxed, _))) {
                        self.discovered_boxed.push(u32::try_from(*slot).ok()?);
                        return None;
                    }
                    let (src, _) = self.pop()?;
                    let dst = self.local_register(*slot)?;
                    self.spill(dst, Class::Scalar)?;
                    if !self.fold_into_destination(src, dst) {
                        self.emit(TypedOp::Move { dst, src });
                    }
                    self.note_write(dst, *slot)?;
                }
            }
            Op::StoreLocalOrGlobalSloppy { slot, name } => {
                if self.slot_is_boxed(*slot)
                    || self.sloppy_global_fallback_slot(name) != Some(*slot)
                {
                    return None;
                }
                let target = self.sloppy_global_write_index(*slot, name)?;
                let (src, _) = self.pop()?;
                let dst = self.local_register(*slot)?;
                self.spill(dst, Class::Scalar)?;
                // The store can still decline at run time. Keep `src` intact
                // until after it publishes so its bytecode site's operand
                // stack can be rebuilt and the generic store replayed exactly.
                self.emit(TypedOp::StoreSloppyGlobal { target, value: src });
                if src != dst {
                    self.emit(TypedOp::Move { dst, src });
                }
            }
            Op::Dup => {
                let top = *self.stack.last()?;
                self.stack.push(top);
            }
            Op::Pop => {
                self.pop()?;
            }
            Op::ToNumeric => {
                let (src, _) = self.pop()?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::ToNumeric { dst, src });
                self.push(dst, Origin::Computed);
            }
            Op::Binary(binary) if admitted_binary(*binary) => {
                let (right, _) = self.pop()?;
                let (left, _) = self.pop()?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Binary {
                    dst,
                    op: *binary,
                    left,
                    right,
                });
                self.push(dst, Origin::Computed);
            }
            Op::Unary(unary) if admitted_unary(*unary) => {
                let (src, _) = self.pop()?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Unary {
                    dst,
                    op: *unary,
                    src,
                });
                self.push(dst, Origin::Computed);
            }
            Op::Update(update) => {
                let (src, _) = self.pop()?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::Update {
                    dst,
                    op: *update,
                    src,
                });
                self.push(dst, Origin::Computed);
            }
            Op::GetProp => {
                let (index, _) = self.pop()?;
                let (receiver_register, receiver_class, receiver) = self.stack.pop()?;
                if receiver_class == Class::Scalar {
                    // An array is not something the scalar file can hold, so a
                    // receiver has to come from a boxed register: that is also
                    // what lets a deoptimization rebuild the operand stack.
                    let Origin::Local(slot) = receiver else {
                        return None;
                    };
                    self.discovered_boxed.push(slot);
                    return None;
                }
                let Origin::Local(receiver_slot) = receiver else {
                    // The receiver is a property read or a global, so the array
                    // it names is only known at run time.
                    let dst = self.slot_boxed()?;
                    self.emit(TypedOp::ElementRead {
                        dst,
                        receiver: receiver_register,
                        index,
                    });
                    self.push_boxed(dst, Origin::Computed);
                    return Some(());
                };
                // A frame slot the region never writes is resolved to its array
                // once per entry, which costs less than revalidating the slot on
                // every element access.
                let receiver = self.receiver_index(receiver_slot)?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::DenseRead {
                    dst,
                    receiver,
                    index,
                });
                self.push(dst, Origin::Computed);
            }
            Op::LoadGlobal(name) => {
                // A matching sloppy fallback read is the register that its
                // per-iteration sink updates. Every other global read may be
                // hoisted only when the region does not write that binding.
                if let Some(slot) = self.sloppy_global_fallback_slot(name) {
                    if self.slot_is_boxed(slot) {
                        return None;
                    }
                    let register = self.local_register(slot)?;
                    self.push(register, Origin::Local(u32::try_from(slot).ok()?));
                    return Some(());
                }
                if self.region_writes_global_named(name) {
                    return None;
                }
                let register = self.global_register(name)?;
                let dst = self.slot_boxed()?;
                self.emit(TypedOp::MoveBoxed { dst, src: register });
                self.push_boxed(dst, Origin::Computed);
            }
            Op::GetPropNamed { key, cache } => {
                let (object, _) = match cache.local_slot() {
                    Some(slot) => (self.boxed_receiver(slot)?, Origin::Local(0)),
                    None => self.pop_boxed()?,
                };
                let name = self.name_index(key)?;
                let cache = self.next_cache()?;
                let dst = self.slot_boxed()?;
                self.emit(TypedOp::GetNamed {
                    dst,
                    object,
                    name,
                    cache,
                });
                self.push_boxed(dst, Origin::Computed);
            }
            Op::SetPropNamed { key, .. } => {
                if self.region_writes_a_sloppy_global() {
                    return None;
                }
                let (value, _) = self.pop_boxed()?;
                let (object, _) = self.pop_boxed()?;
                let _ = &object;
                let name = self.name_index(key)?;
                self.emit(TypedOp::SetNamed {
                    object,
                    name,
                    value,
                });
                // The assignment's value stays on the operand stack.
                let dst = self.slot_boxed()?;
                self.emit(TypedOp::MoveBoxed { dst, src: value });
                self.push_boxed(dst, Origin::Computed);
            }
            Op::CallResolved(argc) if *argc <= 2 => {
                let mut args = [0_u16; 2];
                for index in (0..*argc).rev() {
                    let (register, _) = self.pop()?;
                    args[index] = register;
                }
                let (callee, _) = self.pop_boxed()?;
                // The receiver is dropped: only receiver-independent intrinsics
                // are admitted, and the run-time check proves that.
                let _ = self.stack.pop()?;
                let dst = self.slot_scalar()?;
                self.emit(TypedOp::CallNumericNative {
                    dst,
                    callee,
                    first: args[0],
                    second: args[1],
                    arity: u8::try_from(*argc).ok()?,
                });
                self.push(dst, Origin::Computed);
            }
            Op::JumpIfFalse(target) => {
                // The condition stays on the operand stack: an in-region branch
                // has a `Pop` at its target, and the loop's exit pops it too.
                // The condition is inspected without popping, so a boxed one is
                // narrowed into a scalar copy first.
                let (cond, class, origin) = *self.stack.last()?;
                let cond = if class == Class::Scalar {
                    cond
                } else {
                    let dst = self.fresh()?;
                    self.emit(TypedOp::Unbox { dst, src: cond });
                    let top = self.stack.last_mut()?;
                    *top = (cond, Class::Boxed, origin);
                    dst
                };
                if *target > self.backedge {
                    self.emit(TypedOp::Exit {
                        cond,
                        exit_ip: u32::try_from(*target).ok()?,
                    });
                } else {
                    if *target < self.header {
                        return None;
                    }
                    self.normalize()?;
                    self.record_target(*target, ip)?;
                    self.pending_jumps.push((self.ops.len(), *target));
                    self.emit(TypedOp::JumpIfFalsy { cond, target: 0 });
                }
            }
            Op::BinaryAssignLocals {
                op,
                target,
                stores,
                skip,
            } if admitted_binary(*op) => {
                let (right, _) = self.pop()?;
                let (left, _) = self.pop()?;
                if [*target, stores[0], stores[1]]
                    .into_iter()
                    .any(|slot| self.slot_is_boxed(slot))
                {
                    return None;
                }
                // The result goes straight into the target's register: the
                // operation reads both operands before assigning, so an operand
                // that is the target itself is fine.
                let value = self.local_register(*target)?;
                self.spill(value, Class::Scalar)?;
                self.emit(TypedOp::Binary {
                    dst: value,
                    op: *op,
                    left,
                    right,
                });
                self.note_write(value, *target)?;
                // The fused form assigns the same value to two completion
                // temporaries. One that this instruction alone writes, and that
                // nothing in the region reads, needs no register of its own — the
                // write-back can take the target's.
                for slot in stores {
                    let aliasable = self.bytecode.local_is_compiler_temporary(*slot)
                        && !self.reads_in_region(*slot)
                        && self.writes_in_region(*slot) == 1
                        && self.writes_in_region(*target) == 1;
                    if aliasable {
                        self.note_write(value, *slot)?;
                        continue;
                    }
                    let dst = self.local_register(*slot)?;
                    self.spill(dst, Class::Scalar)?;
                    self.emit(TypedOp::Move { dst, src: value });
                    self.note_write(dst, *slot)?;
                }
                self.pending_skip = *skip;
            }
            Op::IncrementLocal { slot, skip, jump } => {
                if self.slot_is_boxed(*slot) {
                    return None;
                }
                let register = self.local_register(*slot)?;
                self.spill(register, Class::Scalar)?;
                // `Update` coerces its operand the way the unfused
                // `ToNumeric; Update` pair does, so one operation is enough.
                self.emit(TypedOp::Update {
                    dst: register,
                    op: UpdateOp::Increment,
                    src: register,
                });
                self.note_write(register, *slot)?;
                match jump {
                    // The increment that closes this region: the executor loops
                    // on its own, and the walk ends with the skipped span.
                    Some(target) if *target == self.header && ip + *skip == self.backedge => {
                        self.close_backedge()?;
                    }
                    Some(_) => return None,
                    None => {}
                }
                self.pending_skip = *skip;
            }
            Op::CopyLocal { from, to, skip } => {
                match (self.slot_is_boxed(*from), self.slot_is_boxed(*to)) {
                    (false, false) => {
                        let src = self.local_register(*from)?;
                        // A completion temporary this instruction alone writes,
                        // and nothing in the region reads, can share the source's
                        // register: the write-back gives both slots that value.
                        if self.bytecode.local_is_compiler_temporary(*to)
                            && !self.reads_in_region(*to)
                            && self.writes_in_region(*to) == 1
                        {
                            self.note_write(src, *to)?;
                        } else {
                            let dst = self.local_register(*to)?;
                            self.spill(dst, Class::Scalar)?;
                            self.emit(TypedOp::Move { dst, src });
                            self.note_write(dst, *to)?;
                        }
                    }
                    (true, true) => {
                        let src = self.boxed_local_register(*from)?;
                        let dst = self.boxed_local_register(*to)?;
                        self.spill(dst, Class::Boxed)?;
                        self.emit(TypedOp::MoveBoxed { dst, src });
                        if !self.written_boxed_locals.contains(&dst) {
                            self.written_boxed_locals.push(dst);
                        }
                    }
                    _ => return None,
                }
                self.pending_skip = *skip;
            }
            Op::CompareLocalsJumpFalse {
                left,
                right,
                op,
                target,
                skip,
                discard,
            } if admitted_binary(*op) => {
                if self.slot_is_boxed(*left) || self.slot_is_boxed(*right) {
                    return None;
                }
                let left = self.local_register(*left)?;
                let right = self.local_register(*right)?;
                let cond = if *discard {
                    let cond = self.fresh()?;
                    self.emit(TypedOp::Binary {
                        dst: cond,
                        op: *op,
                        left,
                        right,
                    });
                    cond
                } else {
                    // The unfused shape leaves the condition on the stack for the
                    // `Pop` at either successor, so the abstract stack grows and
                    // the exit has to rebuild that entry.
                    let cond = self.slot_scalar()?;
                    self.emit(TypedOp::Binary {
                        dst: cond,
                        op: *op,
                        left,
                        right,
                    });
                    self.push(cond, Origin::Computed);
                    self.open_site(ip)?;
                    cond
                };
                // A falsy comparison jumps; the fused form skips the successor's
                // `Pop` when it also elided the push.
                let exit = if *discard { *target + 1 } else { *target };
                if exit > self.backedge {
                    self.emit(TypedOp::Exit {
                        cond,
                        exit_ip: u32::try_from(exit).ok()?,
                    });
                } else {
                    if exit < self.header {
                        return None;
                    }
                    self.normalize()?;
                    self.record_target(exit, ip)?;
                    self.pending_jumps.push((self.ops.len(), exit));
                    self.emit(TypedOp::JumpIfFalsy { cond, target: 0 });
                }
                self.pending_skip = *skip + usize::from(*discard);
            }
            Op::Jump(target) if *target == self.header && ip == self.backedge => {
                // The backedge itself: the executor loops on its own.
                self.close_backedge()?;
            }
            Op::Jump(target) => {
                if *target < self.header || *target > self.backedge {
                    return None;
                }
                self.normalize()?;
                self.record_target(*target, ip)?;
                self.pending_jumps.push((self.ops.len(), *target));
                self.emit(TypedOp::Jump { target: 0 });
                self.unreachable = true;
                // Everything up to the target is unreachable from here; the
                // linear walk continues, and the recorded depth keeps the join
                // consistent.
            }
            _ => return None,
        }
        Some(())
    }

    /// Checks that looping is equivalent to jumping to the header.
    ///
    /// The header's state has to be a prefix of what the backedge delivers.
    /// Entries beyond it are values the loop body pushes and never pops — the
    /// completion-value bookkeeping leaks one per iteration in some shapes — and
    /// dropping them is unobservable: nothing reads an operand the body left
    /// behind, and the frame discards them when it returns.
    fn close_backedge(&mut self) -> Option<()> {
        self.normalize()?;
        let header = self.states.first()?.clone()?;
        (header.len() <= self.stack.len() && header == self.stack[..header.len()]).then_some(())
    }

    /// Records the abstract stack a jump delivers to `target`.
    ///
    /// A forward jump merges into whatever is recorded, because the walk has not
    /// reached the target yet and will see the merged state. A backward jump
    /// cannot weaken a state the walk already used, so it requires the recorded
    /// one to be no stronger than what it delivers.
    fn record_target(&mut self, target: usize, ip: usize) -> Option<()> {
        let offset = target.checked_sub(self.header)?;
        let state = self.stack.clone();
        if target <= ip {
            let existing = self.states.get(offset)?.as_ref()?;
            return (merge_states(existing, &state)?.as_slice() == existing.as_slice())
                .then_some(());
        }
        match self.states.get_mut(offset)? {
            Some(existing) => {
                let merged = merge_states(existing, &state)?;
                *existing = merged;
                Some(())
            }
            slot => {
                *slot = Some(state);
                Some(())
            }
        }
    }
}

/// Joins two abstract stacks: equal depth and equal register class per entry,
/// keeping the weaker provenance. `None` when they cannot be joined.
fn merge_states(
    left: &[(u16, Class, Origin)],
    right: &[(u16, Class, Origin)],
) -> Option<Vec<(u16, Class, Origin)>> {
    if left.len() != right.len() {
        return None;
    }
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            (left.0 == right.0 && left.1 == right.1).then_some((
                left.0,
                left.1,
                if left.2 == right.2 {
                    left.2
                } else {
                    Origin::Computed
                },
            ))
        })
        .collect()
}

/// Which instructions of a region are the target of a jump inside it: those are
/// the joins, and an operand has to be in its depth's register at each one.
fn is_target(bytecode: &Bytecode, header: usize, backedge: usize) -> Vec<bool> {
    let mut targets = vec![false; backedge - header + 1];
    let mut mark = |target: usize| {
        if let Some(offset) = target.checked_sub(header)
            && offset < targets.len()
        {
            targets[offset] = true;
        }
    };
    for op in &bytecode.code[header..=backedge.min(bytecode.code.len() - 1)] {
        match op {
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => mark(*target),
            Op::CompareLocalsJumpFalse { target, .. } => {
                mark(*target);
                mark(*target + 1);
            }
            Op::IncrementLocal {
                jump: Some(target), ..
            } => mark(*target),
            _ => {}
        }
    }
    targets
}

/// Index expressions are replayed from the surrounding assignment's entry if
/// a later typed operation declines. They therefore cannot contain a write or
/// branch that would become observable twice. The admitted scalar operations
/// themselves either operate on Numbers/booleans or decline before invoking
/// any user code.
fn scalar_expression_may_write_or_branch(op: &Op) -> bool {
    expression_has_control_flow(op)
        || matches!(
            op,
            Op::AppendStringLiteralLocal { .. }
                | Op::AppendStringLiteralGlobal { .. }
                | Op::StoreLocal(_)
                | Op::AssignLocal(_)
                | Op::ClearLocal(_)
                | Op::DefineGlobalVar(_)
                | Op::StoreGlobalStrict(_)
                | Op::StoreGlobalSloppy { .. }
                | Op::StoreLocalOrGlobalSloppy { .. }
                | Op::StoreIdentWith { .. }
                | Op::StoreResolvedIdentWith { .. }
                | Op::SetProp { .. }
                | Op::SetPropNamed { .. }
                | Op::SetPropIndex { .. }
                | Op::SetPrivate(_)
                | Op::DeleteProp { .. }
                | Op::DeleteIdent(_)
                | Op::DeleteIdentWith { .. }
                | Op::IncrementLocal { .. }
                | Op::CopyLocal { .. }
                | Op::BinaryAssignLocals { .. }
        )
}

fn expression_has_control_flow(op: &Op) -> bool {
    matches!(
        op,
        Op::Jump(_)
            | Op::JumpIfFalse(_)
            | Op::JumpIfTrue(_)
            | Op::JumpIfNotNullish(_)
            | Op::AbruptJump(_)
            | Op::CompareLocalsJumpFalse { .. }
    )
}

fn admitted_binary(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::StrictEq
            | BinaryOp::StrictNe
    )
}

fn admitted_unary(op: UnaryOp) -> bool {
    matches!(
        op,
        UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitwiseNot | UnaryOp::Not
    )
}
