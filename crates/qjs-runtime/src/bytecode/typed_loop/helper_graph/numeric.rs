//! Helper bodies lowered to a program over `f64` registers.
//!
//! A flattened helper runs over [`Typed`] registers, so every operation
//! re-checks its operands' tags and returns through an `Option`: about forty
//! instructions an operation, where QuickJS-NG's bytecode spends about
//! sixteen. `bits-in-byte`'s `bitsinbyte` -- a counting loop over a byte --
//! cost 5,800 instructions a call that way against QuickJS-NG's 1,960.
//!
//! Most helper bodies only ever hold numbers, booleans and `undefined`, and
//! for those the three can share one `f64` encoding: a boolean as 0 or 1 and
//! `undefined` as NaN are exactly what `ToNumber` makes of them, so every
//! arithmetic, bitwise and relational operator and every truthiness test
//! gives the same answer on the encoding as on the value. What the encoding
//! loses is the type, which only equality and the returned value observe. A
//! dataflow pass proves, per register and per operation, which of the three
//! a register may hold; a body whose equality operands are not both proven
//! numbers, whose returned register is not proven to hold one kind, or that
//! calls another helper keeps the tagged interpreter.

use qjs_ast::{BinaryOp, UnaryOp};

use super::{HelperOp, MAX_HELPER_REGISTERS};
use crate::bytecode::typed_loop::Typed;
use crate::function::NativeFunction;

/// What a register may hold, as a set.
const NUMBER: u8 = 1;
const BOOLEAN: u8 = 1 << 1;
const UNDEFINED: u8 = 1 << 2;

type Kinds = [u8; MAX_HELPER_REGISTERS];

/// The run-time register file: the next power of two above every register a
/// helper may name.
const FILE: usize = MAX_HELPER_REGISTERS.next_power_of_two();

#[derive(Clone, Copy, Debug)]
enum NumOp {
    Const {
        dst: u16,
        value: f64,
    },
    Move {
        dst: u16,
        src: u16,
    },
    Add {
        dst: u16,
        left: u16,
        right: u16,
    },
    Sub {
        dst: u16,
        left: u16,
        right: u16,
    },
    Mul {
        dst: u16,
        left: u16,
        right: u16,
    },
    Div {
        dst: u16,
        left: u16,
        right: u16,
    },
    BitAnd {
        dst: u16,
        left: u16,
        right: u16,
    },
    BitOr {
        dst: u16,
        left: u16,
        right: u16,
    },
    BitXor {
        dst: u16,
        left: u16,
        right: u16,
    },
    Shl {
        dst: u16,
        left: u16,
        right: u16,
    },
    Shr {
        dst: u16,
        left: u16,
        right: u16,
    },
    UShr {
        dst: u16,
        left: u16,
        right: u16,
    },
    Lt {
        dst: u16,
        left: u16,
        right: u16,
    },
    Le {
        dst: u16,
        left: u16,
        right: u16,
    },
    Gt {
        dst: u16,
        left: u16,
        right: u16,
    },
    Ge {
        dst: u16,
        left: u16,
        right: u16,
    },
    Eq {
        dst: u16,
        left: u16,
        right: u16,
    },
    Ne {
        dst: u16,
        left: u16,
        right: u16,
    },
    /// Any other admitted operator (`%`, `**`), on the encoded numbers.
    Other {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    Neg {
        dst: u16,
        src: u16,
    },
    BitNot {
        dst: u16,
        src: u16,
    },
    Not {
        dst: u16,
        src: u16,
    },
    Native {
        dst: u16,
        native: NativeFunction,
        first: u16,
        second: u16,
        arity: u8,
    },
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    /// A comparison and the branch on it, fused: jumps unless `left cmp right`.
    JumpUnless {
        cmp: Cmp,
        left: u16,
        right: u16,
        target: u32,
    },
    /// `if (left & right)`, fused: jumps when the bitwise and is zero.
    JumpIfAndZero {
        left: u16,
        right: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    Return {
        src: u16,
    },
    /// Removed by optimization; dropped before the program runs.
    Nop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

/// How the returned register's encoding is decoded.
#[derive(Clone, Copy, Debug)]
enum ReturnKind {
    Number,
    Boolean,
    Undefined,
}

/// A helper body over encoded numbers, and the one kind it returns.
#[derive(Clone, Debug)]
pub(super) struct NumProgram {
    ops: Box<[NumOp]>,
    returns: ReturnKind,
    /// Constants kept in the registers above every register the helper
    /// names, loaded once per call rather than by an operation each time.
    constants: Box<[(u16, f64)]>,
}

impl NumProgram {
    /// Lowers `ops`, whose first `arity` registers are the arguments, or
    /// `None` when the encoding could be observed.
    pub(super) fn lower(ops: &[HelperOp], arity: u8) -> Option<Self> {
        let before = infer(ops, arity)?;
        let mut returns: Option<u8> = None;
        let mut lowered = Vec::with_capacity(ops.len());
        for (op, kinds) in ops.iter().zip(&before) {
            // An operation the body never reaches has no proven state; it
            // cannot run, so any encoding of it will do.
            let kinds = kinds.unwrap_or([NUMBER; MAX_HELPER_REGISTERS]);
            let kind = |register: u16| kinds.get(usize::from(register)).copied().unwrap_or(0);
            lowered.push(match *op {
                HelperOp::Const { dst, value } => NumOp::Const {
                    dst,
                    value: encode(value),
                },
                HelperOp::Move { dst, src } => NumOp::Move { dst, src },
                HelperOp::Binary {
                    dst,
                    op,
                    left,
                    right,
                } => {
                    if is_equality(op) && (kind(left) != NUMBER || kind(right) != NUMBER) {
                        return None;
                    }
                    binary(dst, op, left, right)
                }
                HelperOp::Unary { dst, op, src } => match op {
                    UnaryOp::Minus => NumOp::Neg { dst, src },
                    UnaryOp::Plus => NumOp::Move { dst, src },
                    UnaryOp::BitwiseNot => NumOp::BitNot { dst, src },
                    UnaryOp::Not => NumOp::Not { dst, src },
                    _ => return None,
                },
                HelperOp::Native {
                    dst,
                    native,
                    first,
                    second,
                    arity,
                } => NumOp::Native {
                    dst,
                    native,
                    first,
                    second,
                    arity,
                },
                HelperOp::JumpIfFalsy { cond, target } => NumOp::JumpIfFalsy { cond, target },
                HelperOp::Jump { target } => NumOp::Jump { target },
                HelperOp::Return { src } => {
                    let returned = kind(src);
                    if returned.count_ones() != 1
                        || returns.is_some_and(|previous| previous != returned)
                    {
                        return None;
                    }
                    returns = Some(returned);
                    NumOp::Return { src }
                }
                HelperOp::Call { .. } => return None,
            });
        }
        let returns = match returns? {
            NUMBER => ReturnKind::Number,
            BOOLEAN => ReturnKind::Boolean,
            _ => ReturnKind::Undefined,
        };
        let (ops, constants) = optimize(lowered);
        Some(Self {
            ops: ops.into_boxed_slice(),
            returns,
            constants: constants.into_boxed_slice(),
        })
    }

    /// Runs the body on number arguments.
    #[inline(never)]
    pub(super) fn run(&self, args: &[f64]) -> Option<Typed> {
        // Every register that is not an argument starts `undefined`. The file
        // is a power of two wider than any register the helper names, so an
        // operand is masked into range rather than bounds-checked.
        let mut r = [f64::NAN; FILE];
        for (register, argument) in r.iter_mut().zip(args) {
            *register = *argument;
        }
        for &(register, value) in &*self.constants {
            r[usize::from(register) & (FILE - 1)] = value;
        }
        let mut pc = 0_usize;
        loop {
            let op = *self.ops.get(pc)?;
            pc += 1;
            macro_rules! set {
                ($dst:expr, $value:expr) => {{
                    let value = $value;
                    r[usize::from($dst) & (FILE - 1)] = value;
                }};
            }
            macro_rules! get {
                ($register:expr) => {
                    r[usize::from($register) & (FILE - 1)]
                };
            }
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
                } => {
                    let value = crate::bytecode::typed_loop::execute::typed_binary(
                        Typed::Number(get!(left)),
                        op,
                        Typed::Number(get!(right)),
                    )?;
                    set!(dst, encode(value));
                }
                NumOp::Neg { dst, src } => set!(dst, -get!(src)),
                NumOp::BitNot { dst, src } => set!(dst, f64::from(!int32(get!(src)))),
                NumOp::Not { dst, src } => set!(dst, flag(falsy(get!(src)))),
                NumOp::Native {
                    dst,
                    native,
                    first,
                    second,
                    arity,
                } => {
                    let value = match arity {
                        1 => crate::bytecode::vm_numeric_leaf::math_unary(native, get!(first))?,
                        2 => crate::bytecode::vm_numeric_leaf::math_binary(
                            native,
                            get!(first),
                            get!(second),
                        )?,
                        _ => return None,
                    };
                    set!(dst, value);
                }
                NumOp::JumpIfFalsy { cond, target } => {
                    if falsy(get!(cond)) {
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
                NumOp::Return { src } => {
                    let value = get!(src);
                    return Some(match self.returns {
                        ReturnKind::Number => Typed::Number(value),
                        ReturnKind::Boolean => Typed::Boolean(value != 0.0),
                        ReturnKind::Undefined => Typed::Undefined,
                    });
                }
            }
        }
    }
}

/// The kinds each register may hold before each operation, or `None` for
/// an operation no path reaches; `None` overall for a body this encoding
/// cannot represent.
fn infer(ops: &[HelperOp], arity: u8) -> Option<Vec<Option<Kinds>>> {
    let mut entry = [UNDEFINED; MAX_HELPER_REGISTERS];
    for kind in entry.iter_mut().take(usize::from(arity)) {
        *kind = NUMBER;
    }
    let mut before: Vec<Option<Kinds>> = vec![None; ops.len()];
    let mut work = vec![0_usize];
    *before.first_mut()? = Some(entry);
    while let Some(pc) = work.pop() {
        let mut kinds = before.get(pc).copied().flatten()?;
        let mut successors: [Option<usize>; 2] = [Some(pc + 1), None];
        match *ops.get(pc)? {
            HelperOp::Const { dst, value } => *kinds.get_mut(usize::from(dst))? = kind_of(value),
            HelperOp::Move { dst, src } => {
                *kinds.get_mut(usize::from(dst))? = *kinds.get(usize::from(src))?;
            }
            HelperOp::Binary { dst, op, .. } => {
                *kinds.get_mut(usize::from(dst))? =
                    if is_comparison(op) { BOOLEAN } else { NUMBER };
            }
            HelperOp::Unary { dst, op, .. } => {
                *kinds.get_mut(usize::from(dst))? =
                    if op == UnaryOp::Not { BOOLEAN } else { NUMBER };
            }
            HelperOp::Native { dst, .. } => *kinds.get_mut(usize::from(dst))? = NUMBER,
            HelperOp::JumpIfFalsy { target, .. } => successors[1] = Some(target as usize),
            HelperOp::Jump { target } => successors = [Some(target as usize), None],
            HelperOp::Return { .. } => successors = [None, None],
            HelperOp::Call { .. } => return None,
        }
        for next in successors.into_iter().flatten() {
            let slot = before.get_mut(next)?;
            let merged = match slot {
                Some(existing) => {
                    let mut merged = *existing;
                    for (into, from) in merged.iter_mut().zip(kinds) {
                        *into |= from;
                    }
                    merged
                }
                None => kinds,
            };
            if slot.as_ref() != Some(&merged) {
                *slot = Some(merged);
                work.push(next);
            }
        }
    }
    Some(before)
}

fn binary(dst: u16, op: BinaryOp, left: u16, right: u16) -> NumOp {
    match op {
        BinaryOp::Add => NumOp::Add { dst, left, right },
        BinaryOp::Sub => NumOp::Sub { dst, left, right },
        BinaryOp::Mul => NumOp::Mul { dst, left, right },
        BinaryOp::Div => NumOp::Div { dst, left, right },
        BinaryOp::BitwiseAnd => NumOp::BitAnd { dst, left, right },
        BinaryOp::BitwiseOr => NumOp::BitOr { dst, left, right },
        BinaryOp::BitwiseXor => NumOp::BitXor { dst, left, right },
        BinaryOp::Shl => NumOp::Shl { dst, left, right },
        BinaryOp::Shr => NumOp::Shr { dst, left, right },
        BinaryOp::UShr => NumOp::UShr { dst, left, right },
        BinaryOp::Lt => NumOp::Lt { dst, left, right },
        BinaryOp::Le => NumOp::Le { dst, left, right },
        BinaryOp::Gt => NumOp::Gt { dst, left, right },
        BinaryOp::Ge => NumOp::Ge { dst, left, right },
        BinaryOp::Eq | BinaryOp::StrictEq => NumOp::Eq { dst, left, right },
        BinaryOp::Ne | BinaryOp::StrictNe => NumOp::Ne { dst, left, right },
        op => NumOp::Other {
            dst,
            op,
            left,
            right,
        },
    }
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

fn kind_of(value: Typed) -> u8 {
    match value {
        Typed::Number(_) => NUMBER,
        Typed::Boolean(_) => BOOLEAN,
        Typed::Undefined => UNDEFINED,
    }
}

/// A value's encoding: what `ToNumber` makes of it.
fn encode(value: Typed) -> f64 {
    match value {
        Typed::Number(number) => number,
        Typed::Boolean(value) => flag(value),
        Typed::Undefined => f64::NAN,
    }
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn falsy(value: f64) -> bool {
    value == 0.0 || value.is_nan()
}

fn int32(number: f64) -> i32 {
    crate::conversion::to_int32_number(number)
}

fn uint32(number: f64) -> u32 {
    crate::conversion::to_uint32_number(number)
}

/// Register-level cleanup of a lowered body, which the stack-shaped
/// flattening leaves full of temporaries: constants move to registers loaded
/// once per call, copies are propagated within blocks, dead writes are
/// removed, a result feeding only a copy is written to the copy's
/// destination, and a comparison or bitwise and feeding only a branch is
/// fused with it. `bitsinbyte`'s loop goes from 22 operations an iteration
/// to 6.
fn optimize(mut ops: Vec<NumOp>) -> (Vec<NumOp>, Vec<(u16, f64)>) {
    let constants = hoist_constants(&mut ops);
    for _ in 0..3 {
        propagate_copies(&mut ops);
        remove_dead_writes(&mut ops);
        forward_results(&mut ops);
        remove_dead_writes(&mut ops);
    }
    fuse_branches(&mut ops);
    (compact(ops), constants)
}

/// The highest register a lowered body names, plus one.
fn first_free_register(ops: &[NumOp]) -> usize {
    ops.iter()
        .flat_map(|op| {
            let (reads, write) = operands(op);
            reads.into_iter().chain([write]).flatten()
        })
        .map(|register| usize::from(register) + 1)
        .max()
        .unwrap_or(0)
        .max(MAX_HELPER_REGISTERS)
}

fn hoist_constants(ops: &mut [NumOp]) -> Vec<(u16, f64)> {
    let mut constants: Vec<(u16, f64)> = Vec::new();
    let mut next = first_free_register(ops);
    for op in ops.iter_mut() {
        let NumOp::Const { dst, value } = *op else {
            continue;
        };
        let register = match constants
            .iter()
            .find(|(_, existing)| existing.to_bits() == value.to_bits())
        {
            Some(&(register, _)) => register,
            None if next < FILE => {
                let Ok(register) = u16::try_from(next) else {
                    continue;
                };
                next += 1;
                constants.push((register, value));
                register
            }
            None => continue,
        };
        *op = NumOp::Move { dst, src: register };
    }
    constants
}

/// The registers an operation reads, and the one it writes.
fn operands(op: &NumOp) -> ([Option<u16>; 2], Option<u16>) {
    match *op {
        NumOp::Const { dst, .. } => ([None, None], Some(dst)),
        NumOp::Move { dst, src }
        | NumOp::Neg { dst, src }
        | NumOp::BitNot { dst, src }
        | NumOp::Not { dst, src } => ([Some(src), None], Some(dst)),
        NumOp::Add { dst, left, right }
        | NumOp::Sub { dst, left, right }
        | NumOp::Mul { dst, left, right }
        | NumOp::Div { dst, left, right }
        | NumOp::BitAnd { dst, left, right }
        | NumOp::BitOr { dst, left, right }
        | NumOp::BitXor { dst, left, right }
        | NumOp::Shl { dst, left, right }
        | NumOp::Shr { dst, left, right }
        | NumOp::UShr { dst, left, right }
        | NumOp::Lt { dst, left, right }
        | NumOp::Le { dst, left, right }
        | NumOp::Gt { dst, left, right }
        | NumOp::Ge { dst, left, right }
        | NumOp::Eq { dst, left, right }
        | NumOp::Ne { dst, left, right }
        | NumOp::Other {
            dst, left, right, ..
        } => ([Some(left), Some(right)], Some(dst)),
        NumOp::Native {
            dst,
            first,
            second,
            arity,
            ..
        } => ([Some(first), (arity == 2).then_some(second)], Some(dst)),
        NumOp::JumpIfFalsy { cond, .. } => ([Some(cond), None], None),
        NumOp::JumpUnless { left, right, .. } | NumOp::JumpIfAndZero { left, right, .. } => {
            ([Some(left), Some(right)], None)
        }
        NumOp::Return { src } => ([Some(src), None], None),
        NumOp::Jump { .. } | NumOp::Nop => ([None, None], None),
    }
}

/// Rewrites every register an operation reads through `map`.
fn map_reads(op: &mut NumOp, map: impl Fn(u16) -> u16) {
    match op {
        NumOp::Move { src, .. }
        | NumOp::Neg { src, .. }
        | NumOp::BitNot { src, .. }
        | NumOp::Not { src, .. }
        | NumOp::Return { src } => *src = map(*src),
        NumOp::Add { left, right, .. }
        | NumOp::Sub { left, right, .. }
        | NumOp::Mul { left, right, .. }
        | NumOp::Div { left, right, .. }
        | NumOp::BitAnd { left, right, .. }
        | NumOp::BitOr { left, right, .. }
        | NumOp::BitXor { left, right, .. }
        | NumOp::Shl { left, right, .. }
        | NumOp::Shr { left, right, .. }
        | NumOp::UShr { left, right, .. }
        | NumOp::Lt { left, right, .. }
        | NumOp::Le { left, right, .. }
        | NumOp::Gt { left, right, .. }
        | NumOp::Ge { left, right, .. }
        | NumOp::Eq { left, right, .. }
        | NumOp::Ne { left, right, .. }
        | NumOp::Other { left, right, .. }
        | NumOp::JumpUnless { left, right, .. }
        | NumOp::JumpIfAndZero { left, right, .. } => {
            *left = map(*left);
            *right = map(*right);
        }
        NumOp::Native { first, second, .. } => {
            *first = map(*first);
            *second = map(*second);
        }
        NumOp::JumpIfFalsy { cond, .. } => *cond = map(*cond),
        NumOp::Const { .. } | NumOp::Jump { .. } | NumOp::Nop => {}
    }
}

fn set_write(op: &mut NumOp, register: u16) {
    match op {
        NumOp::Const { dst, .. }
        | NumOp::Move { dst, .. }
        | NumOp::Neg { dst, .. }
        | NumOp::BitNot { dst, .. }
        | NumOp::Not { dst, .. }
        | NumOp::Add { dst, .. }
        | NumOp::Sub { dst, .. }
        | NumOp::Mul { dst, .. }
        | NumOp::Div { dst, .. }
        | NumOp::BitAnd { dst, .. }
        | NumOp::BitOr { dst, .. }
        | NumOp::BitXor { dst, .. }
        | NumOp::Shl { dst, .. }
        | NumOp::Shr { dst, .. }
        | NumOp::UShr { dst, .. }
        | NumOp::Lt { dst, .. }
        | NumOp::Le { dst, .. }
        | NumOp::Gt { dst, .. }
        | NumOp::Ge { dst, .. }
        | NumOp::Eq { dst, .. }
        | NumOp::Ne { dst, .. }
        | NumOp::Other { dst, .. }
        | NumOp::Native { dst, .. } => *dst = register,
        _ => {}
    }
}

/// The operations execution can continue at after `pc`.
fn successors(ops: &[NumOp], pc: usize) -> [Option<usize>; 2] {
    let next = (pc + 1 < ops.len()).then_some(pc + 1);
    match ops[pc] {
        NumOp::Jump { target } => [Some(target as usize), None],
        NumOp::JumpIfFalsy { target, .. }
        | NumOp::JumpUnless { target, .. }
        | NumOp::JumpIfAndZero { target, .. } => [next, Some(target as usize)],
        NumOp::Return { .. } => [None, None],
        _ => [next, None],
    }
}

/// Whether any branch lands on each operation.
fn jump_targets(ops: &[NumOp]) -> Vec<bool> {
    let mut targets = vec![false; ops.len() + 1];
    for op in ops {
        if let NumOp::Jump { target }
        | NumOp::JumpIfFalsy { target, .. }
        | NumOp::JumpUnless { target, .. }
        | NumOp::JumpIfAndZero { target, .. } = *op
            && let Some(slot) = targets.get_mut(target as usize)
        {
            *slot = true;
        }
    }
    targets
}

/// Replaces a read of a register that holds a copy of another by a read of
/// the other, within each straight-line block.
fn propagate_copies(ops: &mut [NumOp]) {
    let targets = jump_targets(ops);
    // `copy_of[r]` is the register `r` currently copies, if any.
    let mut copy_of = [u16::MAX; FILE];
    for pc in 0..ops.len() {
        if targets[pc] {
            copy_of = [u16::MAX; FILE];
        }
        let op = &mut ops[pc];
        map_reads(op, |register| {
            match copy_of.get(usize::from(register)).copied() {
                Some(source) if source != u16::MAX => source,
                _ => register,
            }
        });
        let (_, write) = operands(op);
        if let Some(written) = write {
            // Whatever copied the overwritten register no longer does.
            for source in copy_of.iter_mut() {
                if *source == written {
                    *source = u16::MAX;
                }
            }
            if let Some(slot) = copy_of.get_mut(usize::from(written)) {
                *slot = match *op {
                    NumOp::Move { dst, src } if dst != src => src,
                    _ => u16::MAX,
                };
            }
        }
        if matches!(
            op,
            NumOp::Jump { .. }
                | NumOp::JumpIfFalsy { .. }
                | NumOp::JumpUnless { .. }
                | NumOp::JumpIfAndZero { .. }
                | NumOp::Return { .. }
        ) {
            copy_of = [u16::MAX; FILE];
        }
    }
}

/// The registers live after each operation.
fn live_after(ops: &[NumOp]) -> Vec<u32> {
    let mut live_in = vec![0_u32; ops.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for pc in (0..ops.len()).rev() {
            let out = successors(ops, pc)
                .into_iter()
                .flatten()
                .filter_map(|next| live_in.get(next).copied())
                .fold(0, |live, next| live | next);
            let (reads, write) = operands(&ops[pc]);
            let mut live = out;
            if let Some(written) = write {
                live &= !(1_u32 << (usize::from(written) & (FILE - 1)));
            }
            for register in reads.into_iter().flatten() {
                live |= 1_u32 << (usize::from(register) & (FILE - 1));
            }
            if live != live_in[pc] {
                live_in[pc] = live;
                changed = true;
            }
        }
    }
    (0..ops.len())
        .map(|pc| {
            successors(ops, pc)
                .into_iter()
                .flatten()
                .filter_map(|next| live_in.get(next).copied())
                .fold(0, |live, next| live | next)
        })
        .collect()
}

fn is_live(live: u32, register: u16) -> bool {
    live & (1_u32 << (usize::from(register) & (FILE - 1))) != 0
}

/// Removes every write no later operation reads; all of them are pure.
fn remove_dead_writes(ops: &mut [NumOp]) {
    loop {
        let live = live_after(ops);
        let mut removed = false;
        for (op, live) in ops.iter_mut().zip(live) {
            if let (_, Some(written)) = operands(op)
                && !is_live(live, written)
            {
                *op = NumOp::Nop;
                removed = true;
            }
        }
        if !removed {
            return;
        }
    }
}

/// `t = f(...); x = t` with `t` dead afterwards becomes `x = f(...)`.
fn forward_results(ops: &mut [NumOp]) {
    let targets = jump_targets(ops);
    let live = live_after(ops);
    for pc in 0..ops.len().saturating_sub(1) {
        let (_, Some(temporary)) = operands(&ops[pc]) else {
            continue;
        };
        if let NumOp::Move { dst, src } = ops[pc + 1]
            && src == temporary
            && dst != temporary
            && !targets[pc + 1]
            && !is_live(live[pc + 1], temporary)
        {
            set_write(&mut ops[pc], dst);
            ops[pc + 1] = NumOp::Nop;
        }
    }
}

/// A comparison or bitwise and whose only use is the branch right after it.
fn fuse_branches(ops: &mut [NumOp]) {
    let targets = jump_targets(ops);
    let live = live_after(ops);
    for pc in 0..ops.len().saturating_sub(1) {
        let NumOp::JumpIfFalsy { cond, target } = ops[pc + 1] else {
            continue;
        };
        if targets[pc + 1] || is_live(live[pc + 1], cond) {
            continue;
        }
        let fused = match ops[pc] {
            NumOp::Lt { dst, left, right } if dst == cond => Some((Cmp::Lt, left, right)),
            NumOp::Le { dst, left, right } if dst == cond => Some((Cmp::Le, left, right)),
            NumOp::Gt { dst, left, right } if dst == cond => Some((Cmp::Gt, left, right)),
            NumOp::Ge { dst, left, right } if dst == cond => Some((Cmp::Ge, left, right)),
            NumOp::Eq { dst, left, right } if dst == cond => Some((Cmp::Eq, left, right)),
            NumOp::Ne { dst, left, right } if dst == cond => Some((Cmp::Ne, left, right)),
            NumOp::BitAnd { dst, left, right } if dst == cond => {
                ops[pc] = NumOp::Nop;
                ops[pc + 1] = NumOp::JumpIfAndZero {
                    left,
                    right,
                    target,
                };
                continue;
            }
            _ => None,
        };
        if let Some((cmp, left, right)) = fused {
            ops[pc] = NumOp::Nop;
            ops[pc + 1] = NumOp::JumpUnless {
                cmp,
                left,
                right,
                target,
            };
        }
    }
}

/// Drops the removed operations, retargeting every branch.
fn compact(ops: Vec<NumOp>) -> Vec<NumOp> {
    let mut new_index = vec![0_u32; ops.len() + 1];
    let mut next = 0_u32;
    for (index, op) in ops.iter().enumerate() {
        new_index[index] = next;
        if !matches!(op, NumOp::Nop) {
            next += 1;
        }
    }
    new_index[ops.len()] = next;
    let remap = |target: u32| new_index.get(target as usize).copied().unwrap_or(next);
    ops.into_iter()
        .filter(|op| !matches!(op, NumOp::Nop))
        .map(|mut op| {
            match &mut op {
                NumOp::Jump { target }
                | NumOp::JumpIfFalsy { target, .. }
                | NumOp::JumpUnless { target, .. }
                | NumOp::JumpIfAndZero { target, .. } => *target = remap(*target),
                _ => {}
            }
            op
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::HelperOp;
    use super::NumProgram;
    use crate::bytecode::typed_loop::Typed;
    use qjs_ast::BinaryOp;

    /// `function (n) { var s = 0; while (n > 0) { s = s + n; n = n - 1; } return s; }`
    /// in the stack-shaped form flattening produces.
    fn sum_down() -> Vec<HelperOp> {
        vec![
            HelperOp::Const {
                dst: 3,
                value: Typed::Number(0.0),
            },
            HelperOp::Move { dst: 1, src: 3 },
            HelperOp::Move { dst: 3, src: 0 },
            HelperOp::Const {
                dst: 4,
                value: Typed::Number(0.0),
            },
            HelperOp::Binary {
                dst: 3,
                op: BinaryOp::Gt,
                left: 3,
                right: 4,
            },
            HelperOp::JumpIfFalsy {
                cond: 3,
                target: 17,
            },
            HelperOp::Move { dst: 3, src: 1 },
            HelperOp::Move { dst: 4, src: 0 },
            HelperOp::Binary {
                dst: 3,
                op: BinaryOp::Add,
                left: 3,
                right: 4,
            },
            HelperOp::Move { dst: 1, src: 3 },
            HelperOp::Move { dst: 3, src: 0 },
            HelperOp::Const {
                dst: 4,
                value: Typed::Number(1.0),
            },
            HelperOp::Binary {
                dst: 3,
                op: BinaryOp::Sub,
                left: 3,
                right: 4,
            },
            HelperOp::Move { dst: 0, src: 3 },
            HelperOp::Jump { target: 2 },
            HelperOp::Const {
                dst: 3,
                value: Typed::Undefined,
            },
            HelperOp::Return { src: 3 },
            HelperOp::Move { dst: 3, src: 1 },
            HelperOp::Return { src: 3 },
        ]
    }

    #[test]
    fn a_looping_body_runs_optimized_over_numbers() {
        // The `return undefined` no path reaches does not count as a kind
        // the body returns.
        let ops = sum_down();
        let program = NumProgram::lower(&ops, 1).expect("the body is numeric");
        assert!(program.ops.len() < ops.len(), "{:?}", program.ops);
        assert!(matches!(program.run(&[10.0]), Some(Typed::Number(n)) if n == 55.0));
        assert!(matches!(program.run(&[0.0]), Some(Typed::Number(n)) if n == 0.0));
    }

    #[test]
    fn equality_on_a_value_that_may_not_be_a_number_keeps_the_tagged_body() {
        let ops = vec![
            HelperOp::Const {
                dst: 1,
                value: Typed::Boolean(true),
            },
            HelperOp::Binary {
                dst: 2,
                op: BinaryOp::StrictEq,
                left: 0,
                right: 1,
            },
            HelperOp::Return { src: 2 },
        ];
        assert!(NumProgram::lower(&ops, 1).is_none());
    }
}
