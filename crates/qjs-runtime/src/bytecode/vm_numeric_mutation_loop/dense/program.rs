//! Compact mixed Number/word programs for bitwise-heavy dense loops.
//!
//! The translator's `NumberInstruction` remains the authoritative, complete
//! fallback. This module performs a second, def-use-aware lowering after store
//! sinking and local-slot remapping. Values stay in a `u32` bank across chains
//! of ECMAScript bitwise operations and are materialized back to `f64` only
//! when a Number consumer needs them.

use super::*;

const INLINE_COMPACT_REGISTERS: usize = 64;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompactOpcode {
    Constant,
    LoadLocal,
    LoadInvariant,
    DenseLoad,
    DenseStore,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Plus,
    Minus,
    Increment,
    Decrement,
    MathRound,
    ToInt32,
    ToUint32,
    Shl,
    Shr,
    UShr,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    BitwiseNot,
    MaterializeI32,
    MaterializeU32,
}

/// Every hot instruction is exactly two machine words on 32-bit targets and
/// one machine word on 64-bit targets. Register operands are bytes because the
/// source translator admits at most 256 SSA values.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CompactInstruction {
    opcode: CompactOpcode,
    destination: u8,
    left: u8,
    right: u8,
    immediate: u32,
}

impl CompactInstruction {
    fn new(
        opcode: CompactOpcode,
        destination: usize,
        left: usize,
        right: usize,
        immediate: usize,
    ) -> Option<Self> {
        Some(Self {
            opcode,
            destination: u8::try_from(destination).ok()?,
            left: u8::try_from(left).ok()?,
            right: u8::try_from(right).ok()?,
            immediate: u32::try_from(immediate).ok()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeKind {
    Number,
    I32,
    U32,
}

#[derive(Clone, Copy, Debug, Default)]
struct RegisterUses {
    number: bool,
    int32: bool,
    uint32: bool,
}

#[derive(Clone, Copy)]
enum UseKind {
    Number,
    Int32,
    Uint32,
}

fn require_use(
    uses: &mut [RegisterUses],
    source: Register,
    before: Register,
    kind: UseKind,
) -> Option<()> {
    if source >= before {
        return None;
    }
    let uses = uses.get_mut(source)?;
    match kind {
        UseKind::Number => uses.number = true,
        UseKind::Int32 => uses.int32 = true,
        UseKind::Uint32 => uses.uint32 = true,
    }
    Some(())
}

#[derive(Clone, Debug)]
pub(super) struct CompactProgram {
    instructions: Vec<CompactInstruction>,
    constants: Vec<f64>,
    number_registers: usize,
    word_registers: usize,
    number_outputs: Vec<Option<u8>>,
    writes: Vec<LocalWrite>,
    sunk_store: Option<SunkDenseStore>,
}

pub(super) struct CompactScratch {
    inline_numbers: [f64; INLINE_COMPACT_REGISTERS],
    inline_words: [u32; INLINE_COMPACT_REGISTERS],
    large_numbers: Option<Vec<f64>>,
    large_words: Option<Vec<u32>>,
}

impl CompactScratch {
    pub(super) fn new(program: &CompactProgram) -> Self {
        Self {
            inline_numbers: [0.0; INLINE_COMPACT_REGISTERS],
            inline_words: [0; INLINE_COMPACT_REGISTERS],
            large_numbers: (program.number_registers > INLINE_COMPACT_REGISTERS)
                .then(|| vec![0.0; program.number_registers]),
            large_words: (program.word_registers > INLINE_COMPACT_REGISTERS)
                .then(|| vec![0; program.word_registers]),
        }
    }

    pub(super) fn banks(&mut self, program: &CompactProgram) -> (&mut [f64], &mut [u32]) {
        let numbers = match &mut self.large_numbers {
            Some(registers) => registers.as_mut_slice(),
            None => &mut self.inline_numbers[..program.number_registers],
        };
        let words = match &mut self.large_words {
            Some(registers) => registers.as_mut_slice(),
            None => &mut self.inline_words[..program.word_registers],
        };
        (numbers, words)
    }
}

impl CompactProgram {
    /// Lowers only programs that contain a bitwise result. Returning `None`
    /// leaves the complete f64 executor selected for the plan.
    pub(super) fn lower(
        operations: &[NumberInstruction],
        writes: &[LocalWrite],
        sunk_store: Option<SunkDenseStore>,
        required_number_outputs: &[Register],
    ) -> Option<Self> {
        if operations.is_empty() || operations.len() > usize::from(u8::MAX) + 1 {
            return None;
        }

        let mut kinds = Vec::with_capacity(operations.len());
        let mut uses = vec![RegisterUses::default(); operations.len()];
        let mut has_word_result = false;
        for (register, operation) in operations.iter().enumerate() {
            let kind = match *operation {
                NumberInstruction::Constant(_)
                | NumberInstruction::LoadLocal(_)
                | NumberInstruction::LoadInvariant(_)
                | NumberInstruction::DenseLoad { .. }
                | NumberInstruction::DenseStore { .. }
                | NumberInstruction::Update { .. }
                | NumberInstruction::MathRound { .. } => NativeKind::Number,
                NumberInstruction::Binary { operation, .. } => match operation {
                    BinaryOp::Shl
                    | BinaryOp::Shr
                    | BinaryOp::BitwiseAnd
                    | BinaryOp::BitwiseXor
                    | BinaryOp::BitwiseOr => NativeKind::I32,
                    BinaryOp::UShr => NativeKind::U32,
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem => NativeKind::Number,
                    _ => return None,
                },
                NumberInstruction::Unary { operation, .. } => match operation {
                    UnaryOp::Plus | UnaryOp::Minus => NativeKind::Number,
                    UnaryOp::BitwiseNot => NativeKind::I32,
                    _ => return None,
                },
            };
            has_word_result |= !matches!(kind, NativeKind::Number);
            kinds.push(kind);

            match *operation {
                NumberInstruction::Constant(_)
                | NumberInstruction::LoadLocal(_)
                | NumberInstruction::LoadInvariant(_) => {}
                NumberInstruction::DenseLoad { index, .. } => {
                    require_use(&mut uses, index, register, UseKind::Number)?
                }
                NumberInstruction::DenseStore { index, value, .. } => {
                    require_use(&mut uses, index, register, UseKind::Number)?;
                    require_use(&mut uses, value, register, UseKind::Number)?;
                }
                NumberInstruction::Binary {
                    operation,
                    left,
                    right,
                } => match operation {
                    BinaryOp::Shl | BinaryOp::Shr => {
                        require_use(&mut uses, left, register, UseKind::Int32)?;
                        require_use(&mut uses, right, register, UseKind::Uint32)?;
                    }
                    BinaryOp::UShr => {
                        require_use(&mut uses, left, register, UseKind::Uint32)?;
                        require_use(&mut uses, right, register, UseKind::Uint32)?;
                    }
                    BinaryOp::BitwiseAnd | BinaryOp::BitwiseXor | BinaryOp::BitwiseOr => {
                        require_use(&mut uses, left, register, UseKind::Int32)?;
                        require_use(&mut uses, right, register, UseKind::Int32)?;
                    }
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem => {
                        require_use(&mut uses, left, register, UseKind::Number)?;
                        require_use(&mut uses, right, register, UseKind::Number)?;
                    }
                    _ => return None,
                },
                NumberInstruction::Unary { operation, value } => match operation {
                    UnaryOp::Plus | UnaryOp::Minus => {
                        require_use(&mut uses, value, register, UseKind::Number)?
                    }
                    UnaryOp::BitwiseNot => require_use(&mut uses, value, register, UseKind::Int32)?,
                    _ => return None,
                },
                NumberInstruction::Update { value, .. }
                | NumberInstruction::MathRound { value } => {
                    require_use(&mut uses, value, register, UseKind::Number)?
                }
            }
        }
        if !has_word_result {
            return None;
        }

        for write in writes {
            uses.get_mut(write.value)?.number = true;
        }
        if let Some(store) = sunk_store {
            uses.get_mut(store.index)?.number = true;
            uses.get_mut(store.value)?.number = true;
        }
        for register in required_number_outputs {
            uses.get_mut(*register)?.number = true;
        }

        let mut number_outputs = vec![None; operations.len()];
        let mut word_outputs = vec![None; operations.len()];
        let mut next_number = 0usize;
        let mut next_word = 0usize;
        for register in 0..operations.len() {
            let needs_number =
                matches!(kinds[register], NativeKind::Number) || uses[register].number;
            let needs_word = !matches!(kinds[register], NativeKind::Number)
                || uses[register].int32
                || uses[register].uint32;
            if needs_number {
                number_outputs[register] = Some(u8::try_from(next_number).ok()?);
                next_number += 1;
            }
            if needs_word {
                word_outputs[register] = Some(u8::try_from(next_word).ok()?);
                next_word += 1;
            }
        }

        let number = |register: Register| -> Option<usize> {
            Some(usize::from(*number_outputs.get(register)?.as_ref()?))
        };
        let word = |register: Register| -> Option<usize> {
            Some(usize::from(*word_outputs.get(register)?.as_ref()?))
        };
        let mut instructions = Vec::with_capacity(operations.len() * 2);
        let mut constants = Vec::new();
        for (register, operation) in operations.iter().enumerate() {
            let native = kinds[register];
            let destination = match native {
                NativeKind::Number => number(register)?,
                NativeKind::I32 | NativeKind::U32 => word(register)?,
            };
            let instruction = match *operation {
                NumberInstruction::Constant(value) => {
                    let constant = constants.len();
                    constants.push(value);
                    CompactInstruction::new(CompactOpcode::Constant, destination, 0, 0, constant)?
                }
                NumberInstruction::LoadLocal(local) => {
                    CompactInstruction::new(CompactOpcode::LoadLocal, destination, 0, 0, local)?
                }
                NumberInstruction::LoadInvariant(source) => CompactInstruction::new(
                    CompactOpcode::LoadInvariant,
                    destination,
                    0,
                    0,
                    source,
                )?,
                NumberInstruction::DenseLoad { receiver, index } => CompactInstruction::new(
                    CompactOpcode::DenseLoad,
                    destination,
                    number(index)?,
                    0,
                    receiver,
                )?,
                NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                } => CompactInstruction::new(
                    CompactOpcode::DenseStore,
                    destination,
                    number(index)?,
                    number(value)?,
                    receiver,
                )?,
                NumberInstruction::Binary {
                    operation,
                    left,
                    right,
                } => {
                    let (opcode, left, right) = match operation {
                        BinaryOp::Add => (CompactOpcode::Add, number(left)?, number(right)?),
                        BinaryOp::Sub => (CompactOpcode::Sub, number(left)?, number(right)?),
                        BinaryOp::Mul => (CompactOpcode::Mul, number(left)?, number(right)?),
                        BinaryOp::Div => (CompactOpcode::Div, number(left)?, number(right)?),
                        BinaryOp::Rem => (CompactOpcode::Rem, number(left)?, number(right)?),
                        BinaryOp::Shl => (CompactOpcode::Shl, word(left)?, word(right)?),
                        BinaryOp::Shr => (CompactOpcode::Shr, word(left)?, word(right)?),
                        BinaryOp::UShr => (CompactOpcode::UShr, word(left)?, word(right)?),
                        BinaryOp::BitwiseAnd => {
                            (CompactOpcode::BitwiseAnd, word(left)?, word(right)?)
                        }
                        BinaryOp::BitwiseXor => {
                            (CompactOpcode::BitwiseXor, word(left)?, word(right)?)
                        }
                        BinaryOp::BitwiseOr => {
                            (CompactOpcode::BitwiseOr, word(left)?, word(right)?)
                        }
                        _ => return None,
                    };
                    CompactInstruction::new(opcode, destination, left, right, 0)?
                }
                NumberInstruction::Unary { operation, value } => {
                    let (opcode, value) = match operation {
                        UnaryOp::Plus => (CompactOpcode::Plus, number(value)?),
                        UnaryOp::Minus => (CompactOpcode::Minus, number(value)?),
                        UnaryOp::BitwiseNot => (CompactOpcode::BitwiseNot, word(value)?),
                        _ => return None,
                    };
                    CompactInstruction::new(opcode, destination, value, 0, 0)?
                }
                NumberInstruction::Update { operation, value } => {
                    let opcode = match operation {
                        UpdateOp::Increment => CompactOpcode::Increment,
                        UpdateOp::Decrement => CompactOpcode::Decrement,
                    };
                    CompactInstruction::new(opcode, destination, number(value)?, 0, 0)?
                }
                NumberInstruction::MathRound { value } => CompactInstruction::new(
                    CompactOpcode::MathRound,
                    destination,
                    number(value)?,
                    0,
                    0,
                )?,
            };
            instructions.push(instruction);

            if matches!(native, NativeKind::Number)
                && (uses[register].int32 || uses[register].uint32)
            {
                // ToInt32 and ToUint32 have the same 32 payload bits. When a
                // value has consumers of both kinds, one ToInt32 conversion is
                // sufficient and is intentionally shared by every consumer.
                let opcode = if uses[register].int32 {
                    CompactOpcode::ToInt32
                } else {
                    CompactOpcode::ToUint32
                };
                instructions.push(CompactInstruction::new(
                    opcode,
                    word(register)?,
                    number(register)?,
                    0,
                    0,
                )?);
            }
            if !matches!(native, NativeKind::Number) && uses[register].number {
                let opcode = match native {
                    NativeKind::I32 => CompactOpcode::MaterializeI32,
                    NativeKind::U32 => CompactOpcode::MaterializeU32,
                    NativeKind::Number => unreachable!(),
                };
                instructions.push(CompactInstruction::new(
                    opcode,
                    number(register)?,
                    word(register)?,
                    0,
                    0,
                )?);
            }
        }

        let writes = writes
            .iter()
            .map(|write| {
                Some(LocalWrite {
                    local: write.local,
                    value: number(write.value)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let sunk_store = match sunk_store {
            Some(store) => Some(SunkDenseStore {
                receiver: store.receiver,
                index: number(store.index)?,
                value: number(store.value)?,
            }),
            None => None,
        };

        Some(Self {
            instructions,
            constants,
            number_registers: next_number,
            word_registers: next_word,
            number_outputs,
            writes,
            sunk_store,
        })
    }

    #[inline(always)]
    pub(super) fn run_iteration<A, F>(
        &self,
        access: &mut A,
        mut load_local: F,
        invariant_numbers: &[f64],
        numbers: &mut [f64],
        words: &mut [u32],
    ) -> bool
    where
        A: DenseAccess,
        F: FnMut(usize) -> Option<f64>,
    {
        for instruction in &self.instructions {
            let destination = usize::from(instruction.destination);
            let left = usize::from(instruction.left);
            let right = usize::from(instruction.right);
            match instruction.opcode {
                CompactOpcode::Constant => {
                    numbers[destination] = self.constants[instruction.immediate as usize];
                }
                CompactOpcode::LoadLocal => {
                    let Some(value) = load_local(instruction.immediate as usize) else {
                        return false;
                    };
                    numbers[destination] = value;
                }
                CompactOpcode::LoadInvariant => {
                    let Some(value) = invariant_numbers.get(instruction.immediate as usize) else {
                        return false;
                    };
                    numbers[destination] = *value;
                }
                CompactOpcode::DenseLoad => {
                    let Some(index) = array_index_from_number(numbers[left]) else {
                        return false;
                    };
                    let Some(value) = access.load_number(instruction.immediate as usize, index)
                    else {
                        return false;
                    };
                    numbers[destination] = value;
                }
                CompactOpcode::DenseStore => {
                    let Some(index) = array_index_from_number(numbers[left]) else {
                        return false;
                    };
                    let value = numbers[right];
                    if !access.stage_store(instruction.immediate as usize, index, value) {
                        return false;
                    }
                    numbers[destination] = value;
                }
                CompactOpcode::Add => numbers[destination] = numbers[left] + numbers[right],
                CompactOpcode::Sub => numbers[destination] = numbers[left] - numbers[right],
                CompactOpcode::Mul => numbers[destination] = numbers[left] * numbers[right],
                CompactOpcode::Div => numbers[destination] = numbers[left] / numbers[right],
                CompactOpcode::Rem => numbers[destination] = numbers[left] % numbers[right],
                CompactOpcode::Plus => numbers[destination] = numbers[left],
                CompactOpcode::Minus => numbers[destination] = -numbers[left],
                CompactOpcode::Increment => numbers[destination] = numbers[left] + 1.0,
                CompactOpcode::Decrement => numbers[destination] = numbers[left] - 1.0,
                CompactOpcode::MathRound => {
                    record_math_round_operation();
                    numbers[destination] = crate::math::round_number(numbers[left]);
                }
                CompactOpcode::ToInt32 => {
                    words[destination] = to_int32_number(numbers[left]) as u32;
                }
                CompactOpcode::ToUint32 => {
                    words[destination] = to_uint32_number(numbers[left]);
                }
                CompactOpcode::Shl => {
                    words[destination] = words[left] << (words[right] & 0x1f);
                }
                CompactOpcode::Shr => {
                    words[destination] = ((words[left] as i32) >> (words[right] & 0x1f)) as u32;
                }
                CompactOpcode::UShr => {
                    words[destination] = words[left] >> (words[right] & 0x1f);
                }
                CompactOpcode::BitwiseAnd => {
                    words[destination] = words[left] & words[right];
                }
                CompactOpcode::BitwiseXor => {
                    words[destination] = words[left] ^ words[right];
                }
                CompactOpcode::BitwiseOr => {
                    words[destination] = words[left] | words[right];
                }
                CompactOpcode::BitwiseNot => words[destination] = !words[left],
                CompactOpcode::MaterializeI32 => {
                    numbers[destination] = f64::from(words[left] as i32);
                }
                CompactOpcode::MaterializeU32 => {
                    numbers[destination] = f64::from(words[left]);
                }
            }
        }
        true
    }

    pub(super) fn writes(&self) -> &[LocalWrite] {
        &self.writes
    }

    pub(super) fn sunk_store(&self) -> Option<SunkDenseStore> {
        self.sunk_store
    }

    pub(super) fn number_output(&self, register: Register) -> Option<usize> {
        Some(usize::from(*self.number_outputs.get(register)?.as_ref()?))
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::*;

    #[test]
    fn compact_instruction_is_eight_bytes() {
        assert_eq!(size_of::<CompactInstruction>(), 8);
    }

    #[test]
    fn conversions_preserve_ecmascript_boundaries_and_materialization_kind() {
        for (value, expected) in [
            (f64::NAN, 0_u32),
            (f64::INFINITY, 0),
            (-1.0, u32::MAX),
            (4_294_967_297.0, 1),
            (-4_294_967_297.0, u32::MAX),
        ] {
            assert_eq!(to_int32_number(value) as u32, expected, "{value}");
            assert_eq!(to_uint32_number(value), expected, "{value}");
        }
        let sign_bit = 0x8000_0000_u32;
        assert_eq!(f64::from(sign_bit as i32), -2_147_483_648.0);
        assert_eq!(f64::from(sign_bit), 2_147_483_648.0);
    }

    #[test]
    fn lowering_deduplicates_number_to_word_conversions() {
        let operations = vec![
            NumberInstruction::LoadLocal(0),
            NumberInstruction::Constant(1.0),
            NumberInstruction::Binary {
                operation: BinaryOp::BitwiseAnd,
                left: 0,
                right: 1,
            },
            NumberInstruction::Binary {
                operation: BinaryOp::UShr,
                left: 0,
                right: 2,
            },
        ];
        let program =
            CompactProgram::lower(&operations, &[LocalWrite { local: 0, value: 3 }], None, &[])
                .expect("bitwise graph should lower");
        let conversions = program
            .instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.opcode,
                    CompactOpcode::ToInt32 | CompactOpcode::ToUint32
                )
            })
            .count();
        assert_eq!(conversions, 2, "one conversion per Number producer");
        assert_eq!(
            program
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction.opcode, CompactOpcode::ToInt32))
                .count(),
            2,
            "mixed signed/unsigned consumers share the producer's payload"
        );
        assert_eq!(
            program
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction.opcode, CompactOpcode::MaterializeU32))
                .count(),
            1
        );
    }
}
