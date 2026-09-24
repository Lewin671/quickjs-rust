//! Numeric plans lowered straight from bytecode, for bodies outside the
//! compact tier.
//!
//! A hash table's `computeHashCode(key)` is a `switch (typeof key)` whose
//! number case is `(key | 0) == key ? key : ...`, and its `equals(a, b)`
//! compares `typeof a` with `typeof b` before `a == b`. Neither is a shape the
//! compact tier admits, so each call ran on the wide tier at about 2,500
//! instructions. A plan's precondition, though, is that every argument is a
//! number: under it `typeof key` is the constant `"number"`, every `case`
//! comparison of it folds, and what is left reachable is a few numeric
//! operations.
//!
//! This lowering interprets the body abstractly under that precondition,
//! folding whatever the kinds decide, and ends each reachable path it cannot
//! represent with [`NumOp::Bail`]. A plan writes nothing but its registers, so
//! handing a call back at a `Bail` -- before anything observable happened --
//! and running it again on the ordinary path is not observable. Code no path
//! reaches is never lowered.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::{BOOLEAN, NUMBER, NumericPlan, UNDEFINED, admitted, is_comparison, is_equality};
use crate::Value;
use crate::bytecode::ir::{Bytecode, Op};
use crate::bytecode::typed_loop::helper_graph::numeric::{
    NumOp, binary as lower_binary, first_free_register, optimize,
};
use crate::value::JsString;

/// Longest body considered; lowering walks it to a fixed point.
const MAX_CODE: usize = 512;
/// Widest register file a lowered body may name.
const MAX_REGISTERS: usize = 128;

/// What one local or operand-stack entry holds on entry to an instruction.
#[derive(Clone, Debug, PartialEq)]
enum Abs {
    /// A number, boolean or `undefined` -- one of `kinds` -- held in its
    /// register; `known` (as bits) when every path agrees on the value.
    Num { kinds: u8, known: Option<u64> },
    /// A string every path agrees on, never materialized: `typeof` results
    /// and string constants, which only fold.
    Str(JsString),
    /// A value the encoding cannot hold; using it hands the call back.
    Opaque,
    /// A binding that may be uninitialized: reading it may throw, so a read
    /// hands the call back.
    Unset,
}

impl Abs {
    fn number(kinds: u8, known: Option<f64>) -> Self {
        Self::Num {
            kinds,
            known: known.map(f64::to_bits),
        }
    }

    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            _ if self == other => self.clone(),
            (Self::Unset, _) | (_, Self::Unset) => Self::Unset,
            (
                Self::Num { kinds, known },
                Self::Num {
                    kinds: other_kinds,
                    known: other_known,
                },
            ) => Self::Num {
                kinds: kinds | other_kinds,
                known: if known == other_known { *known } else { None },
            },
            _ => Self::Opaque,
        }
    }

    /// Its truthiness, when every path agrees on it.
    fn truthiness(&self) -> Option<bool> {
        match self {
            Self::Num {
                known: Some(bits), ..
            } => {
                let value = f64::from_bits(*bits);
                Some(!(value == 0.0 || value.is_nan()))
            }
            Self::Str(text) => Some(!text.is_empty()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct State {
    locals: Vec<Abs>,
    stack: Vec<Abs>,
}

impl State {
    fn merge(&self, other: &Self) -> Option<Self> {
        if self.stack.len() != other.stack.len() {
            return None;
        }
        let pair = |a: &[Abs], b: &[Abs]| a.iter().zip(b).map(|(a, b)| a.merge(b)).collect();
        Some(Self {
            locals: pair(&self.locals, &other.locals),
            stack: pair(&self.stack, &other.stack),
        })
    }
}

/// Where control goes after one instruction.
enum Flow {
    Next,
    Jump(usize),
    /// Falls through or goes to the target.
    Branch(usize),
    /// Returns or hands the call back.
    End,
}

struct Lowering<'a> {
    bytecode: &'a Bytecode,
    local_count: usize,
    /// Kinds every reachable `Return` returns.
    returns: u8,
}

impl Lowering<'_> {
    fn stack_register(&self, depth: usize) -> Option<u16> {
        u16::try_from(self.local_count.checked_add(depth)?).ok()
    }

    /// Whether `slot` is this frame's own, indexed, writable binding.
    fn writable(&self, slot: usize) -> bool {
        slot < u128::BITS as usize
            && self.bytecode.authoritative_mask_clean() & (1_u128 << slot) != 0
            && self.bytecode.locals.get(slot).is_some_and(|local| {
                local.mutable && !local.sloppy_global_fallback && !local.is_received_upvalue()
            })
    }

    /// Applies `op` at `ip` to `state`, pushing what it lowers to onto
    /// `emit` with branch targets still bytecode indices. `None` declines the
    /// whole body.
    fn step(&mut self, ip: usize, state: &mut State, emit: &mut Vec<NumOp>) -> Option<Flow> {
        let op = self.bytecode.code.get(ip)?;
        let depth = state.stack.len();
        macro_rules! bail {
            () => {{
                emit.push(NumOp::Bail);
                return Some(Flow::End);
            }};
        }
        match op {
            Op::FunctionPrologueEnd if ip == 0 => {}
            Op::LoadConst(index) => {
                let dst = self.stack_register(depth)?;
                let value = match self.bytecode.constants.get(*index)? {
                    Value::Number(number) => Some((NUMBER, *number)),
                    Value::Boolean(value) => Some((BOOLEAN, if *value { 1.0 } else { 0.0 })),
                    Value::Undefined => Some((UNDEFINED, f64::NAN)),
                    _ => None,
                };
                match (value, self.bytecode.constants.get(*index)?) {
                    (Some((kind, value)), _) => {
                        emit.push(NumOp::Const { dst, value });
                        state.stack.push(Abs::number(kind, Some(value)));
                    }
                    (None, Value::String(text)) => state.stack.push(Abs::Str(text.clone())),
                    (None, _) => state.stack.push(Abs::Opaque),
                }
            }
            Op::LoadLocal(slot) => {
                let value = state.locals.get(*slot)?.clone();
                match value {
                    Abs::Unset => bail!(),
                    Abs::Num { .. } => emit.push(NumOp::Move {
                        dst: self.stack_register(depth)?,
                        src: u16::try_from(*slot).ok()?,
                    }),
                    Abs::Str(_) | Abs::Opaque => {}
                }
                state.stack.push(value);
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                if !self.writable(*slot) {
                    bail!();
                }
                // Assigning a binding that may still be in its dead zone
                // throws; initializing one does not.
                if matches!(op, Op::AssignLocal(_)) && *state.locals.get(*slot)? == Abs::Unset {
                    bail!();
                }
                let value = state.stack.pop()?;
                if let Abs::Num { .. } = value {
                    emit.push(NumOp::Move {
                        dst: u16::try_from(*slot).ok()?,
                        src: self.stack_register(depth - 1)?,
                    });
                }
                *state.locals.get_mut(*slot)? = value;
            }
            Op::Pop => {
                state.stack.pop()?;
            }
            Op::Dup => {
                let value = state.stack.last()?.clone();
                if let Abs::Num { .. } = value {
                    emit.push(NumOp::Move {
                        dst: self.stack_register(depth)?,
                        src: self.stack_register(depth - 1)?,
                    });
                }
                state.stack.push(value);
            }
            Op::Typeof => {
                let text = match state.stack.pop()? {
                    Abs::Num { kinds: NUMBER, .. } => "number",
                    Abs::Num { kinds: BOOLEAN, .. } => "boolean",
                    Abs::Num {
                        kinds: UNDEFINED, ..
                    } => "undefined",
                    Abs::Str(_) => "string",
                    _ => bail!(),
                };
                state.stack.push(Abs::Str(JsString::from(text)));
            }
            Op::Binary(binary) => {
                let right = state.stack.pop()?;
                let left = state.stack.pop()?;
                let dst = self.stack_register(depth - 2)?;
                let result = match (&left, &right) {
                    (Abs::Num { kinds: l, .. }, Abs::Num { kinds: r, .. }) => {
                        // `NaN` encodes `undefined` too, so only numbers
                        // compare by the encoding.
                        if !admitted(*binary)
                            || (is_equality(*binary) && (*l != NUMBER || *r != NUMBER))
                        {
                            bail!();
                        }
                        emit.push(lower_binary(dst, *binary, dst, dst + 1));
                        let kind = if is_comparison(*binary) {
                            BOOLEAN
                        } else {
                            NUMBER
                        };
                        Abs::number(kind, None)
                    }
                    (Abs::Str(l), Abs::Str(r)) => {
                        let same = l == r;
                        let value = match binary {
                            BinaryOp::Eq | BinaryOp::StrictEq => same,
                            BinaryOp::Ne | BinaryOp::StrictNe => !same,
                            _ => bail!(),
                        };
                        folded(emit, dst, value)
                    }
                    (Abs::Str(_), Abs::Num { .. }) | (Abs::Num { .. }, Abs::Str(_)) => match binary
                    {
                        BinaryOp::StrictEq => folded(emit, dst, false),
                        BinaryOp::StrictNe => folded(emit, dst, true),
                        _ => bail!(),
                    },
                    _ => bail!(),
                };
                state.stack.push(result);
            }
            Op::Unary(unary) => {
                let value = state.stack.pop()?;
                let register = self.stack_register(depth - 1)?;
                let result = match (&value, unary) {
                    (Abs::Num { .. }, UnaryOp::Minus) => {
                        emit.push(NumOp::Neg {
                            dst: register,
                            src: register,
                        });
                        Abs::number(NUMBER, None)
                    }
                    (Abs::Num { known, .. }, UnaryOp::Plus) => Abs::Num {
                        kinds: NUMBER,
                        known: *known,
                    },
                    (Abs::Num { .. }, UnaryOp::BitwiseNot) => {
                        emit.push(NumOp::BitNot {
                            dst: register,
                            src: register,
                        });
                        Abs::number(NUMBER, None)
                    }
                    (Abs::Num { .. }, UnaryOp::Not) => {
                        emit.push(NumOp::Not {
                            dst: register,
                            src: register,
                        });
                        Abs::number(BOOLEAN, None)
                    }
                    (Abs::Str(_), UnaryOp::Not) => folded(emit, register, !value.truthiness()?),
                    _ => bail!(),
                };
                state.stack.push(result);
            }
            Op::ToNumeric => match state.stack.last_mut()? {
                // `true`, `false` and `undefined` are encoded as their
                // `ToNumber` already.
                Abs::Num { kinds, .. } => *kinds = NUMBER,
                _ => bail!(),
            },
            Op::Update(update) => {
                if !matches!(state.stack.last()?, Abs::Num { .. }) {
                    bail!();
                }
                let register = self.stack_register(depth - 1)?;
                let one = self.stack_register(depth)?;
                emit.push(NumOp::Const {
                    dst: one,
                    value: 1.0,
                });
                emit.push(match update {
                    UpdateOp::Increment => NumOp::Add {
                        dst: register,
                        left: register,
                        right: one,
                    },
                    UpdateOp::Decrement => NumOp::Sub {
                        dst: register,
                        left: register,
                        right: one,
                    },
                });
                *state.stack.last_mut()? = Abs::number(NUMBER, None);
            }
            Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => {
                let jump_if = matches!(op, Op::JumpIfTrue(_));
                let condition = state.stack.last()?;
                if let Some(truthy) = condition.truthiness() {
                    return Some(if truthy == jump_if {
                        emit.push(NumOp::Jump {
                            target: u32::try_from(*target).ok()?,
                        });
                        Flow::Jump(*target)
                    } else {
                        Flow::Next
                    });
                }
                if !matches!(condition, Abs::Num { .. }) {
                    bail!();
                }
                let mut cond = self.stack_register(depth - 1)?;
                if jump_if {
                    let negated = self.stack_register(depth)?;
                    emit.push(NumOp::Not {
                        dst: negated,
                        src: cond,
                    });
                    cond = negated;
                }
                emit.push(NumOp::JumpIfFalsy {
                    cond,
                    target: u32::try_from(*target).ok()?,
                });
                return Some(Flow::Branch(*target));
            }
            Op::Jump(target) => {
                emit.push(NumOp::Jump {
                    target: u32::try_from(*target).ok()?,
                });
                return Some(Flow::Jump(*target));
            }
            Op::Return => match state.stack.pop()? {
                Abs::Num { kinds, .. } if kinds == NUMBER || kinds == BOOLEAN => {
                    self.returns |= kinds;
                    emit.push(NumOp::Return {
                        src: self.stack_register(depth - 1)?,
                    });
                    return Some(Flow::End);
                }
                _ => bail!(),
            },
            _ => bail!(),
        }
        Some(Flow::Next)
    }
}

/// A boolean every path agrees on, written to `dst`.
fn folded(emit: &mut Vec<NumOp>, dst: u16, value: bool) -> Abs {
    let value = if value { 1.0 } else { 0.0 };
    emit.push(NumOp::Const { dst, value });
    Abs::number(BOOLEAN, Some(value))
}

/// Lowers `bytecode` on the precondition that every argument is a number, or
/// `None` when no path returns or the body is outside what this models.
pub(super) fn lower(bytecode: &Bytecode) -> Option<NumericPlan> {
    let code = &bytecode.code;
    if code.is_empty()
        || code.len() > MAX_CODE
        || bytecode.global_scope
        || bytecode.contains_direct_eval()
        || bytecode.contains_with()
        || bytecode.needs_arguments_object()
    {
        return None;
    }
    let local_count = bytecode.locals.len();
    let parameters: Vec<u16> = bytecode
        .parameter_slots()
        .iter()
        .map(|&slot| u16::try_from(slot).ok())
        .collect::<Option<_>>()?;
    let mut entry = State {
        locals: vec![Abs::Unset; local_count],
        stack: Vec::new(),
    };
    for &slot in bytecode.hoisted_slots() {
        *entry.locals.get_mut(slot as usize)? = Abs::number(UNDEFINED, Some(f64::NAN));
    }
    for &slot in &parameters {
        *entry.locals.get_mut(usize::from(slot))? = Abs::number(NUMBER, None);
    }
    // Received cells are the enclosing scope's bindings, not this frame's.
    for (slot, local) in bytecode.locals.iter().enumerate() {
        if local.is_received_upvalue() {
            *entry.locals.get_mut(slot)? = Abs::Unset;
        }
    }

    let mut lowering = Lowering {
        bytecode,
        local_count,
        returns: 0,
    };
    // The state on entry to each instruction, to a fixed point.
    let mut states: Vec<Option<State>> = vec![None; code.len()];
    states[0] = Some(entry);
    let mut work = vec![0_usize];
    let mut scratch = Vec::new();
    let mut max_depth = 0;
    let mut steps = 0_usize;
    while let Some(ip) = work.pop() {
        steps += 1;
        if steps > code.len() * 64 {
            return None;
        }
        let mut state = states.get(ip)?.clone()?;
        max_depth = max_depth.max(state.stack.len());
        let flow = lowering.step(ip, &mut state, &mut scratch)?;
        scratch.clear();
        let successors = match flow {
            Flow::Next => [Some(ip + 1), None],
            Flow::Jump(target) => [Some(target), None],
            Flow::Branch(target) => [Some(ip + 1), Some(target)],
            Flow::End => [None, None],
        };
        for next in successors.into_iter().flatten() {
            let slot = states.get_mut(next)?;
            let merged = match slot {
                Some(existing) => existing.merge(&state)?,
                None => state.clone(),
            };
            if slot.as_ref() != Some(&merged) {
                *slot = Some(merged);
                work.push(next);
            }
        }
    }

    // Emission, in bytecode order: an instruction's lowering is contiguous
    // and a fall-through reaches the next reachable one.
    lowering.returns = 0;
    let mut ops = Vec::with_capacity(code.len());
    let mut start = vec![0_u32; code.len()];
    for ip in 0..code.len() {
        start[ip] = u32::try_from(ops.len()).ok()?;
        let Some(state) = &states[ip] else {
            continue;
        };
        let mut state = state.clone();
        lowering.step(ip, &mut state, &mut ops)?;
    }
    for op in &mut ops {
        if let NumOp::Jump { target } | NumOp::JumpIfFalsy { target, .. } = op {
            *target = *start.get(*target as usize)?;
        }
    }
    // A body that returns both numbers and booleans would need its result
    // kind at run time; one that never returns is no plan at all.
    let returns = lowering.returns;
    if returns != NUMBER && returns != BOOLEAN {
        return None;
    }
    let register_count = local_count.checked_add(max_depth)?.checked_add(1)?;
    if register_count > MAX_REGISTERS {
        return None;
    }
    let (ops, constants) = if register_count <= super::MAX_OPTIMIZED_REGISTERS
        && first_free_register(&ops) <= super::MAX_OPTIMIZED_REGISTERS
    {
        optimize(ops)
    } else {
        (ops, Vec::new())
    };
    let registers = constants
        .iter()
        .map(|&(register, _)| usize::from(register) + 1)
        .max()
        .unwrap_or(0)
        .max(register_count);
    Some(NumericPlan {
        leaf: NumericPlan::is_leaf(&ops, registers),
        ops: ops.into_boxed_slice(),
        registers,
        parameters: parameters.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        returns,
    })
}
