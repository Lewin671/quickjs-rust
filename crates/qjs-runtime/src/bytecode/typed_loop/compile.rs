//! Lowering a loop region's bytecode into a register program.
//!
//! The walk is linear from the header to the backedge, interpreting the operand
//! stack abstractly. A stack entry always lives in the register with its depth's
//! index — in the scalar file or the boxed one, whichever its class names — so
//! two paths that meet at a join agree on where every value is without phi
//! nodes, and a backward jump inside the region (a nested loop) needs nothing
//! beyond checking that the state it delivers matches the state recorded there.

use qjs_ast::{BinaryOp, UnaryOp};

use std::rc::Rc;

use super::super::ir::{Bytecode, Op};
use super::{
    DeoptSite, MAX_REGION_OPS, MAX_REGISTERS, MAX_STACK_DEPTH, Typed, TypedLoopProgram, TypedOp,
};
use crate::Value;

/// Compiles every loop region of `bytecode` that the whitelist admits.
pub(crate) fn compile_all(bytecode: &Bytecode) -> Vec<TypedLoopProgram> {
    bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(target) if *target < backedge => Some((*target, backedge)),
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
        next_register,
        local_slots,
        written_locals,
        receiver_slots,
        global_reads,
        next_boxed,
        boxed_locals,
        written_boxed_locals,
        boxed_global_reads,
        names,
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
                    | TypedOp::ConstBoxed { .. }
                    | TypedOp::Box { .. }
                    | TypedOp::Unbox { .. }
            )
        })
        .count();
    if boxed_ops * 3 > ops.len() {
        return None;
    }
    debug_assert!(matches!(code.get(backedge), Some(Op::Jump(_))));
    Some(TypedLoopProgram {
        header,
        backedge,
        ops,
        sites,
        register_count: next_register,
        local_slots,
        written_locals,
        receiver_slots,
        global_reads,
        boxed_count: next_boxed,
        boxed_locals,
        written_boxed_locals,
        boxed_global_reads,
        names,
        boxed_constants,
        cache_count,
    })
}

/// Where a register's value came from, so a property read can recognize a
/// receiver that is really a frame slot.
#[derive(Clone, Copy, PartialEq)]
enum Origin {
    Computed,
    Local(u32),
}

/// Which register file a stack entry lives in. Scalars are unboxed numbers,
/// booleans, and `undefined`; boxed registers hold any `Value`, which is what a
/// property read produces and what an object receiver has to be.
#[derive(Clone, Copy, PartialEq)]
enum Class {
    Scalar,
    Boxed,
}

struct Builder<'a> {
    bytecode: &'a Bytecode,
    header: usize,
    backedge: usize,
    ops: Vec<TypedOp>,
    sites: Vec<DeoptSite>,
    /// Resume information the next emitted operation inherits.
    site: DeoptSite,
    next_register: usize,
    /// Register holding each referenced frame slot, keyed by slot.
    local_slots: Vec<(u16, u32)>,
    written_locals: Vec<u16>,
    receiver_slots: Vec<u32>,
    global_reads: Vec<(u16, String)>,
    next_boxed: usize,
    boxed_locals: Vec<(u16, u32)>,
    written_boxed_locals: Vec<u16>,
    boxed_global_reads: Vec<(u16, String)>,
    names: Vec<Rc<str>>,
    boxed_constants: Vec<Value>,
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
            site: DeoptSite {
                ip: header as u32,
                depth: 0,
                boxed: 0,
            },
            next_register: MAX_STACK_DEPTH,
            local_slots: Vec::new(),
            written_locals: Vec::new(),
            receiver_slots: Vec::new(),
            global_reads: Vec::new(),
            next_boxed: MAX_STACK_DEPTH,
            boxed_locals: Vec::new(),
            written_boxed_locals: Vec::new(),
            boxed_global_reads: Vec::new(),
            names: Vec::new(),
            boxed_constants: Vec::new(),
            cache_count: 0,
            boxed_slots,
            discovered_boxed: Vec::new(),
            stack: Vec::new(),
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

    fn note_write(&mut self, register: u16) {
        if !self.written_locals.contains(&register) {
            self.written_locals.push(register);
        }
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

    fn push(&mut self, register: u16, origin: Origin) {
        debug_assert_eq!(usize::from(register), self.stack.len());
        self.stack.push((register, Class::Scalar, origin));
    }

    fn push_boxed(&mut self, register: u16, origin: Origin) {
        debug_assert_eq!(usize::from(register), self.stack.len());
        self.stack.push((register, Class::Boxed, origin));
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
        let mut boxed = 0_u64;
        for (depth, (_, class, _)) in self.stack.iter().enumerate() {
            if *class == Class::Boxed {
                boxed |= 1_u64 << depth;
            }
        }
        self.site = DeoptSite {
            ip: u32::try_from(ip).ok()?,
            depth: u8::try_from(self.stack.len()).ok()?,
            boxed,
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
            ip += 1;
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

    /// Matches the four-instruction prologue the compiler emits for
    /// `receiver[key] = value`, compiles the value expression in between, and
    /// matches the `SetProp` tail. Returns the instruction after the tail.
    ///
    /// The idiom has to be recognized as a unit because its receiver and key
    /// travel through compiler temporaries, and a property key is not something
    /// an unboxed scalar register can hold. The temporaries are bypassed
    /// entirely, which is only sound while nothing outside the region reads
    /// them — checked here.
    fn compile_element_assignment(&mut self, ip: usize) -> Option<Option<usize>> {
        let code = &self.bytecode.code;
        let (
            Some(Op::LoadLocal(receiver)),
            Some(Op::StoreLocal(receiver_temp)),
            Some(Op::LoadLocal(index)),
            Some(Op::StoreLocal(index_temp)),
        ) = (
            code.get(ip),
            code.get(ip + 1),
            code.get(ip + 2),
            code.get(ip + 3),
        )
        else {
            return Some(None);
        };
        let (receiver, receiver_temp, index, index_temp) =
            (*receiver, *receiver_temp, *index, *index_temp);
        if receiver == receiver_temp || index_temp == receiver_temp {
            return Some(None);
        }
        // Find the tail: `StoreLocal(v); LoadLocal(rt); LoadLocal(it);
        // LoadLocal(v); SetProp`.
        let mut cursor = ip + 4;
        let tail = loop {
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
                && *second == index_temp
                && *third == *value_temp
            {
                break (cursor, *value_temp);
            }
            cursor += 1;
        };
        let (value_store, value_temp) = tail;
        for temp in [receiver_temp, index_temp, value_temp] {
            if !self.bytecode.local_is_compiler_temporary(temp)
                || self.slot_is_read_outside_region(temp)
            {
                return Some(None);
            }
        }
        // Compile the index and the value expression with the temporaries
        // elided: the index register is produced here, and the value expression
        // is whatever the region computes between the prologue and the tail.
        let index_register = self.local_register(index)?;
        let index_copy = self.fresh()?;
        self.emit(TypedOp::Move {
            dst: index_copy,
            src: index_register,
        });
        let mut inner = ip + 4;
        while inner < value_store {
            if let Some(next) = self.compile_element_assignment(inner)? {
                inner = next;
                continue;
            }
            let op = self.bytecode.code.get(inner)?;
            // A branch inside the value expression would need its own join
            // bookkeeping relative to the elided temporaries.
            if matches!(op, Op::Jump(_) | Op::JumpIfFalse(_)) {
                return None;
            }
            self.compile_op(op, inner)?;
            inner += 1;
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

    /// Whether the region contains any instruction that could write a global,
    /// which would make a hoisted read observably stale.
    fn region_writes_a_global(&self) -> bool {
        self.bytecode.code[self.header..=self.backedge]
            .iter()
            .any(|op| {
                matches!(
                    op,
                    Op::StoreGlobalStrict(_)
                        | Op::StoreGlobalSloppy { .. }
                        | Op::StoreLocalOrGlobalSloppy { .. }
                        | Op::DefineGlobalVar(_)
                        | Op::AppendStringLiteralGlobal { .. }
                )
            })
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
                        let dst = self.slot_scalar()?;
                        self.emit(TypedOp::Const { dst, value });
                        self.push(dst, Origin::Computed);
                    }
                    None => {
                        let constant = constant.clone();
                        let index = u16::try_from(self.boxed_constants.len()).ok()?;
                        self.boxed_constants.push(constant);
                        let dst = self.slot_boxed()?;
                        self.emit(TypedOp::ConstBoxed {
                            dst,
                            constant: index,
                        });
                        self.push_boxed(dst, Origin::Computed);
                    }
                }
            }
            Op::LoadLocal(slot) => {
                let origin = Origin::Local(u32::try_from(*slot).ok()?);
                if self.slot_is_boxed(*slot) {
                    let src = self.boxed_local_register(*slot)?;
                    let dst = self.slot_boxed()?;
                    self.emit(TypedOp::MoveBoxed { dst, src });
                    self.push_boxed(dst, origin);
                } else {
                    let src = self.local_register(*slot)?;
                    let dst = self.slot_scalar()?;
                    self.emit(TypedOp::Move { dst, src });
                    self.push(dst, origin);
                }
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                if self.slot_is_boxed(*slot) {
                    let (src, _) = self.pop_boxed()?;
                    let dst = self.boxed_local_register(*slot)?;
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
                    self.emit(TypedOp::Move { dst, src });
                    self.note_write(dst);
                }
            }
            Op::Dup => {
                let (src, class, origin) = *self.stack.last()?;
                match class {
                    Class::Scalar => {
                        let dst = self.slot_scalar()?;
                        self.emit(TypedOp::Move { dst, src });
                        self.push(dst, origin);
                    }
                    Class::Boxed => {
                        let dst = self.slot_boxed()?;
                        self.emit(TypedOp::MoveBoxed { dst, src });
                        self.push_boxed(dst, origin);
                    }
                }
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
                if receiver_class == Class::Boxed {
                    // The receiver came from a property read or a global, so the
                    // array is only known at run time.
                    let dst = self.slot_boxed()?;
                    self.emit(TypedOp::ElementRead {
                        dst,
                        receiver: receiver_register,
                        index,
                    });
                    self.push_boxed(dst, Origin::Computed);
                    return Some(());
                }
                let Origin::Local(receiver_slot) = receiver else {
                    return None;
                };
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
                // A global read is hoisted to loop entry, which is only
                // equivalent while the region writes no global at all. The value
                // lands in a boxed register: an object receiver needs one, and a
                // numeric global is narrowed at its first use.
                if self.region_writes_a_global() {
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
                    self.record_target(*target, ip)?;
                    self.pending_jumps.push((self.ops.len(), *target));
                    self.emit(TypedOp::JumpIfFalsy { cond, target: 0 });
                }
            }
            Op::Jump(target) if *target == self.header && ip == self.backedge => {
                // The backedge itself: the executor loops on its own.
            }
            Op::Jump(target) => {
                if *target < self.header || *target > self.backedge {
                    return None;
                }
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
