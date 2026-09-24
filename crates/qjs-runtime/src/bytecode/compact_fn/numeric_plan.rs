//! Compact bodies over numbers only, run on `f64` registers with their calls.
//!
//! A recursive numeric body -- `fib`, `ack`, `tak` -- runs on the compact
//! tier at about 900 instructions a call against QuickJS-NG's 400, most of it
//! frame machinery for `Value` registers. When every register of the body
//! only ever holds a number, a boolean or `undefined`, and every call it makes
//! reaches another such body through a captured cell, the whole call tree can
//! run on `f64` registers with a frame stack of its own: a boolean as 0 or 1
//! and `undefined` as NaN are exactly what `ToNumber` makes of them, which is
//! all the arithmetic, relational and truthiness operations observe.
//!
//! Such a tree runs no user code but its own and writes nothing outside its
//! registers, so abandoning it at any point -- a callee that is not such a
//! body, a recursion deeper than [`MAX_DEPTH`] -- and running the call again
//! on the ordinary path is not observable.

use std::rc::Rc;

use qjs_ast::BinaryOp;

use super::{CompactFunctionProgram, CompactOp};
use crate::Value;
use crate::bytecode::ir::Bytecode;
use crate::bytecode::typed_loop::helper_graph::numeric::{
    Cmp, NumOp, binary as lower_binary, first_free_register, optimize,
};
use crate::function::Upvalue;

mod from_bytecode;

/// Widest body the shared optimizer handles (`helper_graph::numeric`).
const MAX_OPTIMIZED_REGISTERS: usize =
    crate::bytecode::typed_loop::helper_graph::MAX_HELPER_REGISTERS;

/// How deep a plan's own frame stack may grow before the call is handed back.
const MAX_DEPTH: usize = 10_000;

const NUMBER: u8 = 1;
const BOOLEAN: u8 = 1 << 1;
const UNDEFINED: u8 = 1 << 2;
/// A register holding the callee a received cell supplied.
const CALLEE: u8 = 1 << 3;

/// A compact body lowered to `f64` registers.
#[derive(Debug)]
pub(in crate::bytecode) struct NumericPlan {
    ops: Box<[NumOp]>,
    registers: usize,
    /// The register each argument lands in, by position.
    parameters: Box<[u16]>,
    /// Constants the optimizer moved into registers, written per frame.
    constants: Box<[(u16, f64)]>,
    /// What every return returns: `NUMBER`, or `BOOLEAN` encoded as 0 or 1.
    returns: u8,
}

impl NumericPlan {
    /// Lowers `program`, the compact form of `bytecode`, when every value it
    /// holds is one the encoding represents and it returns only numbers.
    pub(super) fn lower(bytecode: &Bytecode, program: &CompactFunctionProgram) -> Option<Self> {
        let parameters: Vec<u16> = bytecode
            .parameter_slots()
            .iter()
            .map(|&slot| u16::try_from(slot).ok())
            .collect::<Option<_>>()?;
        let before = infer(program, bytecode, &parameters)?;
        let mut ops = Vec::with_capacity(program.ops.len());
        let mut callee_slot = vec![u16::MAX; program.register_count];
        for (op, kinds) in program.ops.iter().zip(&before) {
            let Some(kinds) = kinds else {
                // Unreachable: any encoding will do.
                ops.push(NumOp::Nop);
                continue;
            };
            let kind = |register: u16| kinds.get(usize::from(register)).copied().unwrap_or(0);
            ops.push(match *op {
                CompactOp::LoadConst { dst, index } => NumOp::Const {
                    dst,
                    value: encode(bytecode.constants.get(index as usize)?)?,
                },
                CompactOp::Move { dst, src } => {
                    if kind(src) & CALLEE != 0 {
                        return None;
                    }
                    NumOp::Move { dst, src }
                }
                CompactOp::LoadUpvalueLocal { dst, slot } => {
                    *callee_slot.get_mut(usize::from(dst))? = slot;
                    // Nothing to do at run time: the call resolves the cell.
                    NumOp::Nop
                }
                CompactOp::Drop { .. } => NumOp::Nop,
                CompactOp::Binary {
                    dst,
                    op,
                    left,
                    right,
                } => {
                    if kind(left) & CALLEE != 0 || kind(right) & CALLEE != 0 {
                        return None;
                    }
                    if is_equality(op) && (kind(left) != NUMBER || kind(right) != NUMBER) {
                        return None;
                    }
                    if !admitted(op) {
                        return None;
                    }
                    lower_binary(dst, op, left, right)
                }
                CompactOp::JumpIfFalsy { cond, target } => {
                    if kind(cond) & CALLEE != 0 {
                        return None;
                    }
                    NumOp::JumpIfFalsy { cond, target }
                }
                CompactOp::Jump { target } => NumOp::Jump { target },
                CompactOp::Call { dst, base, argc } => {
                    if kind(base) != CALLEE {
                        return None;
                    }
                    let slot = *callee_slot.get(usize::from(base))?;
                    if slot == u16::MAX {
                        return None;
                    }
                    for argument in 0..u16::from(argc) {
                        if kind(base.checked_add(1 + argument)?) & CALLEE != 0 {
                            return None;
                        }
                    }
                    NumOp::Call {
                        dst,
                        callee: slot,
                        args: base.checked_add(1)?,
                        argc,
                    }
                }
                CompactOp::Return { src } => {
                    if kind(src) != NUMBER {
                        return None;
                    }
                    NumOp::Return { src }
                }
            });
        }
        // The optimizer tracks liveness in one 32-bit word, so only a body
        // naming fewer registers than a helper may is optimized; any other
        // runs its operations as lowered.
        let (ops, constants) = if program.register_count <= MAX_OPTIMIZED_REGISTERS
            && first_free_register(&ops) <= MAX_OPTIMIZED_REGISTERS
        {
            optimize(ops)
        } else {
            (ops, Vec::new())
        };
        // The frame holds the body's registers and the constant registers
        // the optimizer placed above them.
        let registers = constants
            .iter()
            .map(|&(register, _)| usize::from(register) + 1)
            .max()
            .unwrap_or(0)
            .max(program.register_count);
        Some(Self {
            ops: ops.into_boxed_slice(),
            registers,
            parameters: parameters.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            returns: NUMBER,
        })
    }

    /// Whether the plan calls other bodies, which it resolves through its
    /// function's received cells.
    pub(in crate::bytecode) fn calls_out(&self) -> bool {
        self.ops.iter().any(|op| matches!(op, NumOp::Call { .. }))
    }
}

/// The plan for `bytecode`, lowering it on first use.
pub(in crate::bytecode) fn plan_for(bytecode: &Bytecode) -> Option<&Rc<NumericPlan>> {
    bytecode
        .compact_numeric_plan
        .get_or_init(|| {
            super::program_for(bytecode)
                .and_then(|program| NumericPlan::lower(bytecode, program))
                .or_else(|| from_bytecode::lower(bytecode))
                .map(Rc::new)
        })
        .as_ref()
}

/// Calls one body may have resolved per run before it hands the call back.
const MAX_LINKS: usize = 8;

/// One body a run has reached, and the calls of it already resolved.
struct Body {
    plan: Rc<NumericPlan>,
    /// The function whose cells the body's calls read; `None` for the root,
    /// whose cells and bytecode the caller lends.
    function: Option<crate::Function>,
    bytecode: Option<Rc<Bytecode>>,
    /// (received-upvalue slot, body index) for each call resolved so far.
    /// The cells cannot change while a plan runs -- it writes nothing but
    /// its registers -- so a call resolves once per run.
    links: [(u16, u16); MAX_LINKS],
    link_count: u8,
}

#[derive(Clone, Copy)]
struct Frame {
    body: u16,
    pc: u32,
    base: usize,
    /// The caller's register receiving this frame's result.
    dst: usize,
}

/// Storage a run reuses: the register stack, the frames and the bodies.
#[derive(Default)]
struct Scratch {
    registers: Vec<f64>,
    frames: Vec<Frame>,
    bodies: Vec<Body>,
}

thread_local! {
    /// Taken for the length of a run and put back after, so a run allocates
    /// nothing once warm; a run nested inside another (a plan never calls
    /// out, but a caller may re-enter) finds it taken and uses fresh storage.
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

/// Runs `plan` -- the root body, whose received cells are `cells` -- on
/// number arguments, or `None` to hand the call back unobserved.
pub(in crate::bytecode) fn run(
    plan: &Rc<NumericPlan>,
    root_bytecode: &Bytecode,
    cells: &[Upvalue],
    args: &[f64],
) -> Option<Value> {
    let mut scratch = SCRATCH.with(|slot| slot.take());
    let result = run_with(plan, root_bytecode, cells, args, &mut scratch);
    scratch.registers.clear();
    scratch.frames.clear();
    scratch.bodies.clear();
    SCRATCH.with(|slot| slot.replace(scratch));
    let result = result?;
    Some(if plan.returns == BOOLEAN {
        Value::Boolean(result != 0.0)
    } else {
        Value::Number(result)
    })
}

fn run_with(
    plan: &Rc<NumericPlan>,
    root_bytecode: &Bytecode,
    cells: &[Upvalue],
    args: &[f64],
    scratch: &mut Scratch,
) -> Option<f64> {
    let Scratch {
        registers,
        frames,
        bodies,
    } = scratch;
    bodies.push(Body {
        plan: Rc::clone(plan),
        function: None,
        bytecode: None,
        links: [(0, 0); MAX_LINKS],
        link_count: 0,
    });
    // The plan was lowered with every parameter a number: a missing
    // argument is `undefined`, which the encoding cannot tell from `NaN`.
    if args.len() < plan.parameters.len() {
        return None;
    }
    registers.resize(plan.registers, f64::NAN);
    for &(register, value) in &*plan.constants {
        *registers.get_mut(usize::from(register))? = value;
    }
    for (&register, &argument) in plan.parameters.iter().zip(args) {
        *registers.get_mut(usize::from(register))? = argument;
    }
    let mut body = 0_usize;
    let mut pc = 0_usize;
    let mut base = 0_usize;
    let int32 = crate::conversion::to_int32_number;
    let uint32 = crate::conversion::to_uint32_number;
    loop {
        // One pass of this loop per frame switch: the running body's
        // operations are borrowed for the whole inner loop.
        let current = Rc::clone(&bodies.get(body)?.plan);
        let ops = &current.ops[..];
        let window = registers.get_mut(base..)?;
        macro_rules! get {
            ($register:expr) => {
                *window.get(usize::from($register))?
            };
        }
        macro_rules! set {
            ($register:expr, $value:expr) => {{
                let value = $value;
                *window.get_mut(usize::from($register))? = value;
            }};
        }
        // What left the inner loop: a call to set up, or a returned value.
        let (call, returned) = loop {
            let op = *ops.get(pc)?;
            pc += 1;
            match op {
                NumOp::Const { dst, value } => set!(dst, value),
                NumOp::Move { dst, src } => set!(dst, get!(src)),
                NumOp::Add { dst, left, right } => set!(dst, get!(left) + get!(right)),
                NumOp::Sub { dst, left, right } => set!(dst, get!(left) - get!(right)),
                NumOp::Mul { dst, left, right } => set!(dst, get!(left) * get!(right)),
                NumOp::Div { dst, left, right } => set!(dst, get!(left) / get!(right)),
                NumOp::BitAnd { dst, left, right } => {
                    set!(dst, f64::from(int32(get!(left)) & int32(get!(right))));
                }
                NumOp::BitOr { dst, left, right } => {
                    set!(dst, f64::from(int32(get!(left)) | int32(get!(right))));
                }
                NumOp::BitXor { dst, left, right } => {
                    set!(dst, f64::from(int32(get!(left)) ^ int32(get!(right))));
                }
                NumOp::Shl { dst, left, right } => {
                    set!(
                        dst,
                        f64::from(int32(get!(left)) << (uint32(get!(right)) & 0x1f))
                    );
                }
                NumOp::Shr { dst, left, right } => {
                    set!(
                        dst,
                        f64::from(int32(get!(left)) >> (uint32(get!(right)) & 0x1f))
                    );
                }
                NumOp::UShr { dst, left, right } => {
                    set!(
                        dst,
                        f64::from(uint32(get!(left)) >> (uint32(get!(right)) & 0x1f))
                    );
                }
                NumOp::Lt { dst, left, right } => set!(dst, flag(get!(left) < get!(right))),
                NumOp::Le { dst, left, right } => set!(dst, flag(get!(left) <= get!(right))),
                NumOp::Gt { dst, left, right } => set!(dst, flag(get!(left) > get!(right))),
                NumOp::Ge { dst, left, right } => set!(dst, flag(get!(left) >= get!(right))),
                NumOp::Eq { dst, left, right } => set!(dst, flag(get!(left) == get!(right))),
                NumOp::Ne { dst, left, right } => set!(dst, flag(get!(left) != get!(right))),
                NumOp::Other {
                    dst,
                    op,
                    left,
                    right,
                } => set!(dst, binary(op, get!(left), get!(right))?),
                NumOp::Neg { dst, src } => set!(dst, -get!(src)),
                NumOp::BitNot { dst, src } => set!(dst, f64::from(!int32(get!(src)))),
                NumOp::Not { dst, src } => {
                    let value = get!(src);
                    set!(dst, flag(value == 0.0 || value.is_nan()));
                }
                NumOp::Native { .. } | NumOp::Bail => return None,
                NumOp::JumpIfFalsy { cond, target } => {
                    let value = get!(cond);
                    if value == 0.0 || value.is_nan() {
                        pc = target as usize;
                    }
                }
                NumOp::JumpUnless {
                    cmp,
                    left,
                    right,
                    target,
                } => {
                    let (left, right) = (get!(left), get!(right));
                    let holds = match cmp {
                        Cmp::Lt => left < right,
                        Cmp::Le => left <= right,
                        Cmp::Gt => left > right,
                        Cmp::Ge => left >= right,
                        Cmp::Eq => left == right,
                        Cmp::Ne => left != right,
                    };
                    if !holds {
                        pc = target as usize;
                    }
                }
                NumOp::JumpIfAndZero {
                    left,
                    right,
                    target,
                } => {
                    if int32(get!(left)) & int32(get!(right)) == 0 {
                        pc = target as usize;
                    }
                }
                NumOp::Nop => {}
                NumOp::Jump { target } => pc = target as usize,
                NumOp::Call {
                    dst,
                    callee,
                    args,
                    argc,
                } => break (Some((dst, callee, args, argc)), 0.0),
                NumOp::Return { src } => break (None, get!(src)),
            }
        };
        match call {
            Some((dst, slot, args, argc)) => {
                if frames.len() >= MAX_DEPTH {
                    return None;
                }
                let callee = resolve(bodies, body, slot, root_bytecode, cells)?;
                let callee_plan = &bodies.get(callee)?.plan;
                if usize::from(argc) != callee_plan.parameters.len() {
                    return None;
                }
                let callee_base = base + current.registers;
                let end = callee_base + callee_plan.registers;
                if registers.len() < end {
                    registers.resize(end, f64::NAN);
                }
                // Every register a body does not receive starts `undefined`.
                registers.get_mut(callee_base..end)?.fill(f64::NAN);
                for &(register, value) in &*callee_plan.constants {
                    *registers.get_mut(callee_base + usize::from(register))? = value;
                }
                for (index, &parameter) in callee_plan.parameters.iter().enumerate() {
                    let value = *registers.get(base + usize::from(args) + index)?;
                    *registers.get_mut(callee_base + usize::from(parameter))? = value;
                }
                frames.push(Frame {
                    body: u16::try_from(body).ok()?,
                    pc: u32::try_from(pc).ok()?,
                    base,
                    dst: base + usize::from(dst),
                });
                body = callee;
                pc = 0;
                base = callee_base;
            }
            None => {
                let Some(frame) = frames.pop() else {
                    return Some(returned);
                };
                body = usize::from(frame.body);
                pc = frame.pc as usize;
                base = frame.base;
                *registers.get_mut(frame.dst)? = returned;
            }
        }
    }
}

/// The body index a call of received-upvalue slot `slot` from body `caller`
/// reaches, resolving and adding it on first use.
fn resolve(
    bodies: &mut Vec<Body>,
    caller: usize,
    slot: u16,
    root_bytecode: &Bytecode,
    root_cells: &[Upvalue],
) -> Option<usize> {
    let caller_body = bodies.get(caller)?;
    if let Some(&(_, callee)) = caller_body
        .links
        .get(..usize::from(caller_body.link_count))?
        .iter()
        .find(|(linked, _)| *linked == slot)
    {
        return Some(usize::from(callee));
    }
    let callee = link(bodies, caller, slot, root_bytecode, root_cells)?;
    let caller_body = bodies.get_mut(caller)?;
    let count = usize::from(caller_body.link_count);
    *caller_body.links.get_mut(count)? = (slot, u16::try_from(callee).ok()?);
    caller_body.link_count += 1;
    Some(callee)
}

/// Resolves the call of received-upvalue slot `slot` in body `caller` to a
/// body of this run, adding one for a function it has not reached yet.
fn link(
    bodies: &mut Vec<Body>,
    caller: usize,
    slot: u16,
    root_bytecode: &Bytecode,
    root_cells: &[Upvalue],
) -> Option<usize> {
    let caller_body = bodies.get(caller)?;
    let bytecode: &Bytecode = caller_body.bytecode.as_deref().unwrap_or(root_bytecode);
    let index = bytecode.direct_readonly_received_upvalue_index(usize::from(slot))?;
    let cell = match &caller_body.function {
        Some(function) => function.upvalues.get(index)?,
        None => root_cells.get(index)?,
    };
    // Compared in place: a body this run already reached costs no clone.
    let existing = cell.with_value(|value| {
        let Value::Function(callee) = value else {
            return None;
        };
        Some(bodies.iter().position(|body| {
            body.function
                .as_ref()
                .is_some_and(|function| function == callee)
        }))
    })?;
    if let Some(existing) = existing {
        return Some(existing);
    }
    let callee_value = cell.get();
    let Value::Function(callee) = &callee_value else {
        return None;
    };
    if !super::activation::admits_numeric_callee(&callee_value, callee) {
        return None;
    }
    let callee = callee.clone();
    let callee_bytecode = Rc::clone(callee.bytecode.as_ref()?);
    let plan = Rc::clone(plan_for(&callee_bytecode)?);
    // A caller treats what a call returns as a number.
    if plan.returns != NUMBER {
        return None;
    }
    bodies.push(Body {
        plan,
        function: Some(callee),
        bytecode: Some(callee_bytecode),
        links: [(0, 0); MAX_LINKS],
        link_count: 0,
    });
    Some(bodies.len() - 1)
}

/// The kinds each register may hold before each operation, or `None` for a
/// body the encoding cannot represent.
fn infer(
    program: &CompactFunctionProgram,
    bytecode: &Bytecode,
    parameters: &[u16],
) -> Option<Vec<Option<Vec<u8>>>> {
    let count = program.register_count;
    let mut entry = vec![UNDEFINED; count];
    for &parameter in parameters {
        *entry.get_mut(usize::from(parameter))? = NUMBER;
    }
    let mut before: Vec<Option<Vec<u8>>> = vec![None; program.ops.len()];
    *before.first_mut()? = Some(entry);
    let mut work = vec![0_usize];
    while let Some(pc) = work.pop() {
        let mut kinds = before.get(pc)?.clone()?;
        let mut successors: [Option<usize>; 2] = [Some(pc + 1), None];
        match *program.ops.get(pc)? {
            CompactOp::LoadConst { dst, index } => {
                *kinds.get_mut(usize::from(dst))? =
                    kind_of(bytecode.constants.get(index as usize)?)?;
            }
            CompactOp::Move { dst, src } => {
                *kinds.get_mut(usize::from(dst))? = *kinds.get(usize::from(src))?;
            }
            CompactOp::LoadUpvalueLocal { dst, .. } => *kinds.get_mut(usize::from(dst))? = CALLEE,
            CompactOp::Drop { .. } => {}
            CompactOp::Binary { dst, op, .. } => {
                *kinds.get_mut(usize::from(dst))? =
                    if is_comparison(op) { BOOLEAN } else { NUMBER };
            }
            CompactOp::JumpIfFalsy { target, .. } => successors[1] = Some(target as usize),
            CompactOp::Jump { target } => successors = [Some(target as usize), None],
            CompactOp::Call { dst, .. } => *kinds.get_mut(usize::from(dst))? = NUMBER,
            CompactOp::Return { .. } => successors = [None, None],
        }
        for next in successors.into_iter().flatten() {
            if next >= program.ops.len() {
                // Falling off the end returns `undefined`, which this plan
                // does not return.
                return None;
            }
            let slot = before.get_mut(next)?;
            let merged = match slot {
                Some(existing) => existing.iter().zip(&kinds).map(|(a, b)| a | b).collect(),
                None => kinds.clone(),
            };
            if slot.as_ref() != Some(&merged) {
                *slot = Some(merged);
                work.push(next);
            }
        }
    }
    Some(before)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn binary(op: BinaryOp, left: f64, right: f64) -> Option<f64> {
    let int32 = crate::conversion::to_int32_number;
    let uint32 = crate::conversion::to_uint32_number;
    let flag = |value: bool| if value { 1.0 } else { 0.0 };
    Some(match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
        BinaryOp::Rem => crate::operations::number_remainder(left, right),
        BinaryOp::BitwiseAnd => f64::from(int32(left) & int32(right)),
        BinaryOp::BitwiseOr => f64::from(int32(left) | int32(right)),
        BinaryOp::BitwiseXor => f64::from(int32(left) ^ int32(right)),
        BinaryOp::Shl => f64::from(int32(left) << (uint32(right) & 0x1f)),
        BinaryOp::Shr => f64::from(int32(left) >> (uint32(right) & 0x1f)),
        BinaryOp::UShr => f64::from(uint32(left) >> (uint32(right) & 0x1f)),
        BinaryOp::Lt => flag(left < right),
        BinaryOp::Le => flag(left <= right),
        BinaryOp::Gt => flag(left > right),
        BinaryOp::Ge => flag(left >= right),
        BinaryOp::Eq | BinaryOp::StrictEq => flag(left == right),
        BinaryOp::Ne | BinaryOp::StrictNe => flag(left != right),
        _ => return None,
    })
}

fn admitted(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
    ) || is_comparison(op)
}

fn is_equality(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq | BinaryOp::StrictEq | BinaryOp::Ne | BinaryOp::StrictNe
    )
}

fn is_comparison(op: BinaryOp) -> bool {
    is_equality(op)
        || matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        )
}

fn kind_of(value: &Value) -> Option<u8> {
    Some(match value {
        Value::Number(_) => NUMBER,
        Value::Boolean(_) => BOOLEAN,
        Value::Undefined => UNDEFINED,
        _ => return None,
    })
}

fn encode(value: &Value) -> Option<f64> {
    Some(match value {
        Value::Number(number) => *number,
        Value::Boolean(value) => {
            if *value {
                1.0
            } else {
                0.0
            }
        }
        Value::Undefined => f64::NAN,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
