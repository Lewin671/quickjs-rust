//! Bounded numeric helper graphs, prepared once per typed-loop entry.
//!
//! A call to an ordinary JavaScript function used to abort a whole loop region
//! at compile time, because the tier could only lower an intrinsic reached
//! through the `Math` global. That single gap is the terminal blocker on the
//! generic-path half of the external corpus: `imaging-darkroom`,
//! `stanford-crypto-aes`, `crypto-aes`, `access-nsieve` and `string-fasta` all
//! give up on an `Op::Call`, and `imaging-darkroom` alone dispatches 449 M
//! generic instructions with every one of its 1.9 M backedges declining.
//!
//! The helper bodies that matter are pure arithmetic over their parameters:
//!
//! ```text
//! function Clamp(x)       { return (x < 0.0) ? 0.0 : ((x > 1.0) ? 1.0 : x); }
//! function FastLog2(x)    { return Math.log(x) / Math.LN2; }
//! function FastBias(b, x) { return Math.pow(x, FastLog2(b) / LOG2_HALF); }
//! ```
//!
//! Each reads only parameters, number constants, and read-only captured cells
//! holding either a number or another such function, so the whole call graph
//! can be flattened once — at loop entry, where the receiver, the cells and the
//! intrinsics are all in hand — into a register program over [`Typed`] values.
//!
//! **Preparation happens once per entry, never per call.** Rebuilding and
//! re-guarding the graph at every helper invocation was measured on this
//! workload and regressed it to 1.13x
//! (`tasks/performance-units/typed-loop-entry-prepared-numeric-helper-graph.json`).
//! What remains per call is one function-identity comparison.
//!
//! What makes reading the cells once sound is that an admitted region runs no
//! user code: every operation it contains either cannot call out at all or is a
//! prepared helper, and a prepared helper body contains no store of any kind.
//! [`Preparation::region_may_mutate_bindings`] additionally refuses the whole
//! preparation when the region writes a global or a named property, which are
//! the two operations that could otherwise reach a captured binding or replace
//! an intrinsic between entry and the call.

use qjs_ast::{BinaryOp, UnaryOp};

use super::super::ir::{Bytecode, Op};
use super::super::vm::Vm;
use super::Typed;
use crate::Value;
use crate::function::{Function, NativeFunction};

/// How many helper bodies one loop entry may flatten. A graph deeper or wider
/// than this is not a leaf computation any more, and preparation is per entry,
/// so the bound is what keeps that cost bounded too.
const MAX_DEPTH: usize = 4;
const MAX_GRAPHS: usize = 12;
const MAX_OPS: usize = 96;

/// How deep a flattened body may recurse into itself before handing the call
/// back to the interpreter. `callTree`-shaped recursion is the reason this
/// exists at all.
///
/// It is a stack-resource bound, not a correctness one: a flattened body is
/// pure, so stopping at any depth and letting the interpreter run the call is
/// not observable. No test fails when it is removed, because a recursion deep
/// enough to exhaust the Rust stack here would also exhaust the interpreter's.
const MAX_NATIVE_RECURSION: usize = 96;

/// Registers a single helper body may use, arguments included. The runtime
/// register file is a fixed array of this size on the Rust stack, which is what
/// keeps a helper call free of allocation.
pub(super) const MAX_HELPER_REGISTERS: usize = 24;

/// One operation of a flattened helper body. Every operand is a register index
/// into that body's own file, which starts at zero for every call — no window
/// base is ever added.
#[derive(Clone, Copy, Debug)]
enum HelperOp {
    Const {
        dst: u16,
        value: Typed,
    },
    Move {
        dst: u16,
        src: u16,
    },
    Binary {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    Unary {
        dst: u16,
        op: UnaryOp,
        src: u16,
    },
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    /// A pure floating-point intrinsic, resolved to its `NativeFunction` at
    /// preparation and therefore not re-looked-up per call.
    Native {
        dst: u16,
        native: NativeFunction,
        first: u16,
        second: u16,
        arity: u8,
    },
    /// Another prepared helper in the same graph.
    Call {
        dst: u16,
        graph: u16,
        first: u16,
        second: u16,
        arity: u8,
    },
    Return {
        src: u16,
    },
}

/// One flattened helper body.
#[derive(Clone, Debug)]
struct HelperProgram {
    ops: Vec<HelperOp>,
    arity: u8,
    /// Frame slots the body assigns, each holding a register above the
    /// arguments. They start `undefined`, which is the correct seed for a
    /// hoisted `var`.
    locals: u8,
    /// The exact function this body was flattened from. Every call compares
    /// against it, so a register that comes to hold a different function
    /// deoptimizes rather than running the wrong body.
    callee: Function,
}

/// Every helper body one loop entry prepared, indexed by the call sites that
/// reach them.
#[derive(Clone, Debug, Default)]
pub(super) struct HelperGraph {
    programs: Vec<HelperProgram>,
}

impl HelperGraph {
    /// Runs the body prepared for `callee`, or `None` when this entry prepared
    /// none -- which is every program without a call site, and every callee a
    /// site turned out not to reach.
    pub(super) fn call(
        &self,
        callee: &Value,
        first: Typed,
        second: Typed,
        arity: u8,
    ) -> Option<Typed> {
        if self.programs.is_empty() {
            return None;
        }
        let Value::Function(function) = callee else {
            return None;
        };
        let index = self
            .programs
            .iter()
            .position(|program| program.arity == arity && program.callee == *function)?;
        self.run(u16::try_from(index).ok()?, first, second, 0)
    }

    /// Runs one body. `depth` bounds native recursion: a flattened body is pure,
    /// so abandoning it at any point and letting the interpreter run the call
    /// again is not observable, which is what makes a hard cap safe here.
    fn run(&self, index: u16, first: Typed, second: Typed, depth: usize) -> Option<Typed> {
        if depth >= MAX_NATIVE_RECURSION {
            return None;
        }
        let program = self.programs.get(index as usize)?;
        // `Typed` is `Copy`, so the whole file is a stack array: a helper call
        // allocates nothing and its registers stay in the frame the compiler
        // chose for this function.
        let mut registers = [Typed::Undefined; MAX_HELPER_REGISTERS];
        if program.arity > 0 {
            registers[0] = first;
        }
        if program.arity > 1 {
            registers[1] = second;
        }
        let mut pc = 0_usize;
        loop {
            let op = *program.ops.get(pc)?;
            pc += 1;
            match op {
                HelperOp::Const { dst, value } => registers[dst as usize] = value,
                HelperOp::Move { dst, src } => registers[dst as usize] = registers[src as usize],
                HelperOp::Binary {
                    dst,
                    op,
                    left,
                    right,
                } => {
                    registers[dst as usize] = super::execute::typed_binary(
                        registers[left as usize],
                        op,
                        registers[right as usize],
                    )?;
                }
                HelperOp::Unary { dst, op, src } => {
                    registers[dst as usize] =
                        super::execute::typed_unary(op, registers[src as usize])?;
                }
                HelperOp::JumpIfFalsy { cond, target } => {
                    if !registers[cond as usize].is_truthy() {
                        pc = target as usize;
                    }
                }
                HelperOp::Jump { target } => pc = target as usize,
                HelperOp::Native {
                    dst,
                    native,
                    first,
                    second,
                    arity,
                } => {
                    let value = match arity {
                        1 => super::super::vm_numeric_leaf::math_unary(
                            native,
                            registers[first as usize].number()?,
                        )?,
                        2 => super::super::vm_numeric_leaf::math_binary(
                            native,
                            registers[first as usize].number()?,
                            registers[second as usize].number()?,
                        )?,
                        _ => return None,
                    };
                    registers[dst as usize] = Typed::Number(value);
                }
                HelperOp::Call {
                    dst,
                    graph,
                    first,
                    second,
                    arity,
                } => {
                    let callee = self.programs.get(graph as usize)?;
                    if callee.arity != arity {
                        return None;
                    }
                    registers[dst as usize] = self.run(
                        graph,
                        registers[first as usize],
                        registers[second as usize],
                        depth + 1,
                    )?;
                }
                HelperOp::Return { src } => return Some(registers[src as usize]),
            }
        }
    }
}

/// What one abstract stack entry of a helper body holds while it is flattened.
///
/// Only `Register` survives into the emitted program; the rest are
/// preparation-time facts about a value that is constant for this entry.
#[derive(Clone)]
enum Slot {
    Register(u16),
    /// A function reached through a captured cell or a global, callable here.
    Callee(Value),
    /// A native intrinsic reached as a property of an object.
    Native(NativeFunction),
    /// A namespace object whose data properties may be read.
    Namespace(Value),
}

/// Flattens helper bodies for one loop entry.
pub(super) struct Preparation {
    graph: HelperGraph,
}

impl Preparation {
    /// Whether the region could reach a captured binding or an intrinsic
    /// between entry and a helper call, which is what preparation-time reads
    /// depend on not happening.
    ///
    /// Dense element writes and frame-slot writes cannot: neither can name a
    /// binding cell or a namespace property. The three operations listed
    /// could, at least in principle -- a named write whose receiver is the
    /// global object or `Math` would do it.
    ///
    /// No test demonstrates a break without this check, because a region that
    /// writes a global declines earlier for its own reasons and a named write
    /// to a namespace object is not a shape the tier admits today. It is
    /// therefore a stated precondition of reading the bindings once, not a
    /// defence with a witness -- and it costs nothing, since it is evaluated
    /// once per entry.
    pub(super) fn region_may_mutate_bindings(program: &super::TypedLoopProgram) -> bool {
        program.ops.iter().any(|op| {
            matches!(
                op,
                super::TypedOp::StoreSloppyGlobal { .. }
                    | super::TypedOp::SetNamed { .. }
                    | super::TypedOp::ComputedWrite { .. }
            )
        })
    }

    /// Prepares every call site of `program`, or `None` if any of them cannot
    /// be flattened -- in which case the whole loop program declines rather
    /// than paying a deoptimization per iteration.
    pub(super) fn prepare(
        vm: &mut Vm<'_>,
        program: &super::TypedLoopProgram,
    ) -> Option<HelperGraph> {
        if Self::region_may_mutate_bindings(program) {
            return None;
        }
        let mut preparation = Self {
            graph: HelperGraph::default(),
        };
        for site in &program.helper_sites {
            let callee = vm.local_slot_value(site.callee_slot as usize)?;
            // A body the closed-form evaluators already answer is left to
            // them: they resolve it in one pass with no interpretation, and
            // shadowing one cost `math-cordic` 3.7%. The site then simply has
            // no graph entry, which is the state every call site was in before
            // this module existed.
            if closed_form_already_answers(&callee, site.arity) {
                continue;
            }
            preparation.prepare_callee(vm, &callee, site.arity, 0)?;
        }
        Some(preparation.graph)
    }

    fn prepare_callee(
        &mut self,
        vm: &mut Vm<'_>,
        callee: &Value,
        arity: u8,
        depth: usize,
    ) -> Option<u16> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let Value::Function(function) = callee else {
            return None;
        };
        if let Some(index) = self
            .graph
            .programs
            .iter()
            .position(|program| program.callee == *function && program.arity == arity)
        {
            return u16::try_from(index).ok();
        }
        if self.graph.programs.len() >= MAX_GRAPHS {
            return None;
        }
        // Anything with its own calling protocol -- bound, native, class
        // constructor, generator, async -- is outside what a flattened
        // arithmetic body can stand for. `is_direct_leaf_function` is the
        // interpreter's own predicate for a callee whose parameters may be
        // seeded straight into slots, which is exactly this shape.
        if !crate::function::is_direct_leaf_function(callee) {
            return None;
        }
        let bytecode = function.bytecode.as_ref()?;
        if bytecode.parameter_slots().len() != usize::from(arity) {
            return None;
        }
        // Reserve this body's index *before* walking it, so a self-call inside
        // finds itself rather than recursing until the depth bound. Mutual
        // recursion resolves the same way.
        //
        // Everything this attempt appended is dropped if the walk fails. That
        // is hygiene rather than a guard: `ops` is only filled on success, and
        // an empty body stops at its first instruction and deoptimizes.
        let index = u16::try_from(self.graph.programs.len()).ok()?;
        let reserved = self.graph.programs.len();
        self.graph.programs.push(HelperProgram {
            ops: Vec::new(),
            arity,
            callee: function.clone(),
            locals: 0,
        });
        let Some((ops, locals)) = self.flatten(vm, function, bytecode, arity, depth) else {
            self.graph.programs.truncate(reserved);
            return None;
        };
        let program = self.graph.programs.get_mut(reserved)?;
        program.ops = ops;
        program.locals = locals;
        Some(index)
    }

    fn flatten(
        &mut self,
        vm: &mut Vm<'_>,
        function: &Function,
        bytecode: &Bytecode,
        arity: u8,
        depth: usize,
    ) -> Option<(Vec<HelperOp>, u8)> {
        let code = &bytecode.code;
        if code.len() > MAX_OPS || !matches!(code.first(), Some(Op::FunctionPrologueEnd)) {
            return None;
        }
        // Slots the body assigns get a register each, above the arguments, so
        // the abstract stack starts after both. Collected up front because the
        // layout has to be fixed before the first operation is emitted.
        let mut locals: Vec<usize> = Vec::new();
        for op in code {
            let (Op::StoreLocal(slot) | Op::AssignLocal(slot)) = op else {
                continue;
            };
            if bytecode.parameter_slots().contains(slot) {
                // Assigning a parameter would make the argument register live
                // in two roles; nothing this tier needs does it.
                return None;
            }
            if !locals.contains(slot) {
                locals.push(*slot);
            }
        }
        let local_count = u8::try_from(locals.len()).ok()?;
        let mut walk = Walk {
            ops: Vec::with_capacity(code.len()),
            stack: Vec::new(),
            states: vec![None; code.len() + 1],
            program_index: vec![None; code.len() + 1],
            pending: Vec::new(),
            unreachable: false,
            arity,
            locals,
        };
        for ip in 0..code.len() {
            if walk.unreachable {
                match walk.states[ip].clone() {
                    Some(state) => {
                        walk.stack = state;
                        walk.unreachable = false;
                    }
                    None => continue,
                }
            } else if let Some(recorded) = walk.states[ip].clone() {
                // Every live entry at a join is a register named by its depth,
                // so agreeing on depth is agreeing on the whole state.
                if recorded.len() != walk.stack.len() {
                    return None;
                }
                if !walk
                    .stack
                    .iter()
                    .all(|slot| matches!(slot, Slot::Register(_)))
                {
                    return None;
                }
            }
            walk.program_index[ip] = u16::try_from(walk.ops.len()).ok();
            self.step(vm, &mut walk, function, bytecode, code.get(ip)?, depth)?;
        }
        walk.program_index[code.len()] = u16::try_from(walk.ops.len()).ok();
        for (slot, target_ip) in std::mem::take(&mut walk.pending) {
            let target = u32::from(walk.program_index.get(target_ip).copied().flatten()?);
            match &mut walk.ops[slot] {
                HelperOp::Jump { target: field } | HelperOp::JumpIfFalsy { target: field, .. } => {
                    *field = target;
                }
                _ => return None,
            }
        }
        // A body that can fall off its end has an implicit `return undefined`
        // this representation does not model.
        walk.unreachable.then_some((walk.ops, local_count))
    }

    fn step(
        &mut self,
        vm: &mut Vm<'_>,
        walk: &mut Walk,
        function: &Function,
        bytecode: &Bytecode,
        op: &Op,
        depth: usize,
    ) -> Option<()> {
        match op {
            Op::FunctionPrologueEnd => {}
            Op::LoadConst(index) => {
                let value = Typed::from_value(bytecode.constants.get(*index)?)?;
                let dst = walk.push_register()?;
                walk.ops.push(HelperOp::Const { dst, value });
            }
            Op::LoadLocal(slot) => {
                if let Some(index) = bytecode
                    .parameter_slots()
                    .iter()
                    .position(|parameter| parameter == slot)
                {
                    let src = u16::try_from(index).ok()?;
                    let dst = walk.push_register()?;
                    walk.ops.push(HelperOp::Move { dst, src });
                } else if let Some(src) = walk.local_register(*slot) {
                    let dst = walk.push_register()?;
                    walk.ops.push(HelperOp::Move { dst, src });
                } else {
                    let value = captured_value(function, bytecode, *slot)?;
                    walk.push_value(&value)?;
                }
            }
            Op::LoadGlobal(name) => {
                // An accessor on the global object would be observable per
                // read, so the binding has to be a plain one.
                if vm
                    .global_this_own_property(name)
                    .is_some_and(|property| property.is_accessor())
                {
                    return None;
                }
                let value = vm.load_global(name).ok()?;
                walk.push_value(&value)?;
            }
            Op::GetPropNamed { key, .. } => {
                let Some(Slot::Namespace(object)) = walk.stack.pop() else {
                    return None;
                };
                let Value::Object(object) = object else {
                    return None;
                };
                let value = super::execute::ordinary_data_property(&object, key)?;
                walk.push_value(&value)?;
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                let src = walk.pop_register()?;
                let dst = walk.local_register(*slot)?;
                walk.ops.push(HelperOp::Move { dst, src });
            }
            Op::Dup => {
                let top = walk.stack.last()?.clone();
                walk.stack.push(top);
            }
            Op::Pop => {
                walk.stack.pop()?;
            }
            Op::Binary(op) if super::compile::admitted_binary(*op) => {
                let right = walk.pop_register()?;
                let left = walk.pop_register()?;
                let dst = walk.push_register()?;
                walk.ops.push(HelperOp::Binary {
                    dst,
                    op: *op,
                    left,
                    right,
                });
            }
            Op::Unary(op) if super::compile::admitted_unary(*op) => {
                let src = walk.pop_register()?;
                let dst = walk.push_register()?;
                walk.ops.push(HelperOp::Unary { dst, op: *op, src });
            }
            Op::JumpIfFalse(target) => {
                // The branch peeks, so both edges carry the same stack.
                let Some(Slot::Register(cond)) = walk.stack.last().cloned() else {
                    return None;
                };
                walk.record_target(*target)?;
                walk.pending.push((walk.ops.len(), *target));
                walk.ops.push(HelperOp::JumpIfFalsy { cond, target: 0 });
            }
            Op::Jump(target) => {
                walk.record_target(*target)?;
                walk.pending.push((walk.ops.len(), *target));
                walk.ops.push(HelperOp::Jump { target: 0 });
                walk.unreachable = true;
            }
            Op::Call(argc) if *argc <= 2 => {
                self.call(vm, walk, *argc, false, depth)?;
            }
            Op::CallResolved(argc) if *argc <= 2 => {
                self.call(vm, walk, *argc, true, depth)?;
            }
            Op::CallResolvedGuardedMathUnary => {
                self.call(vm, walk, 1, true, depth)?;
            }
            Op::Return => {
                let src = walk.pop_register()?;
                walk.ops.push(HelperOp::Return { src });
                walk.unreachable = true;
            }
            _ => return None,
        }
        Some(())
    }

    /// Lowers one call inside a helper body. `resolved` marks the receiver-
    /// carrying form, whose abstract stack is `[receiver, callee, args...]`.
    fn call(
        &mut self,
        vm: &mut Vm<'_>,
        walk: &mut Walk,
        argc: usize,
        resolved: bool,
        depth: usize,
    ) -> Option<()> {
        let mut args = [0_u16; 2];
        for index in (0..argc).rev() {
            args[index] = walk.pop_register()?;
        }
        let callee = walk.stack.pop()?;
        if resolved {
            // The receiver is a namespace whose property produced the callee;
            // it carries no value of its own into the computation.
            match walk.stack.pop()? {
                Slot::Namespace(_) => {}
                _ => return None,
            }
        }
        let arity = u8::try_from(argc).ok()?;
        let dst = walk.push_register()?;
        match callee {
            Slot::Native(native) => walk.ops.push(HelperOp::Native {
                dst,
                native,
                first: args[0],
                second: args[1],
                arity,
            }),
            Slot::Callee(value) => {
                let graph = self.prepare_callee(vm, &value, arity, depth + 1)?;
                walk.ops.push(HelperOp::Call {
                    dst,
                    graph,
                    first: args[0],
                    second: args[1],
                    arity,
                });
            }
            _ => return None,
        }
        Some(())
    }
}

/// Whether `vm_numeric_leaf` resolves this callee on its own.
///
/// The probe evaluates the body once with numeric arguments. That is not
/// observable: the evaluator's whole precondition is a closed-form numeric
/// expression, which is why the tier is already allowed to call it in place of
/// an ordinary call. It runs once per loop entry.
fn closed_form_already_answers(callee: &Value, arity: u8) -> bool {
    let Value::Function(function) = callee else {
        return false;
    };
    let Some(bytecode) = function.bytecode.as_ref() else {
        return false;
    };
    let probe = [Value::Number(1.0), Value::Number(1.0)];
    let Some(arguments) = probe.get(..usize::from(arity)) else {
        return false;
    };
    super::super::vm_numeric_leaf::try_eval_numeric_leaf(
        bytecode,
        &function.params,
        arguments,
        &function.upvalues,
    )
    .is_some()
}

/// Reads the read-only captured cell a helper body's non-parameter local names.
///
/// Only a cell the body cannot write qualifies: its value is then fixed for the
/// whole entry, which is what lets preparation fold it into the program.
fn captured_value(function: &Function, bytecode: &Bytecode, slot: usize) -> Option<Value> {
    let slots = bytecode.direct_readonly_received_upvalue_slots()?;
    let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
    (slots & bit != 0).then_some(())?;
    let index = bytecode.direct_readonly_received_upvalue_index(slot)?;
    let value = function.upvalues.get(index)?.get();
    (!value.is_uninitialized_lexical_marker()).then_some(value)
}

struct Walk {
    ops: Vec<HelperOp>,
    stack: Vec<Slot>,
    states: Vec<Option<Vec<Slot>>>,
    program_index: Vec<Option<u16>>,
    pending: Vec<(usize, usize)>,
    unreachable: bool,
    arity: u8,
    /// Frame slots this body assigns, in register order above the arguments.
    locals: Vec<usize>,
}

impl Walk {
    /// The register holding an assigned frame slot, or `None` when the body
    /// never assigns it.
    fn local_register(&self, slot: usize) -> Option<u16> {
        let index = self
            .locals
            .iter()
            .position(|candidate| *candidate == slot)?;
        u16::try_from(usize::from(self.arity) + index).ok()
    }

    /// Registers `0..arity` hold the arguments and the assigned locals follow,
    /// so an abstract stack entry at depth `d` lives above both.
    fn push_register(&mut self) -> Option<u16> {
        let base = usize::from(self.arity) + self.locals.len();
        let register = u16::try_from(base + self.stack.len()).ok()?;
        (usize::from(register) < MAX_HELPER_REGISTERS).then_some(())?;
        self.stack.push(Slot::Register(register));
        Some(register)
    }

    fn pop_register(&mut self) -> Option<u16> {
        match self.stack.pop()? {
            Slot::Register(register) => Some(register),
            _ => None,
        }
    }

    fn push_value(&mut self, value: &Value) -> Option<()> {
        let slot = match value {
            Value::Number(_) | Value::Boolean(_) | Value::Undefined => {
                let typed = Typed::from_value(value)?;
                let dst = self.push_register()?;
                self.ops.push(HelperOp::Const { dst, value: typed });
                return Some(());
            }
            Value::Function(function) if function.native.is_some() && function.bound.is_none() => {
                Slot::Native(function.native?)
            }
            Value::Function(_) => Slot::Callee(value.clone()),
            Value::Object(_) => Slot::Namespace(value.clone()),
            _ => return None,
        };
        self.stack.push(slot);
        Some(())
    }

    /// Records the abstract stack a forward branch delivers to `target`.
    fn record_target(&mut self, target: usize) -> Option<()> {
        let state = self.states.get_mut(target)?;
        match state {
            Some(existing) => (existing.len() == self.stack.len()).then_some(()),
            None => {
                *state = Some(self.stack.clone());
                Some(())
            }
        }
    }
}
