use std::rc::Rc;

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::ir::{Bytecode, Op};

/// Fixed-width opcode for the first compact-dispatch slice.
///
/// Completely lowered direct-leaf frames execute this representation in the
/// existing VM/frame scheduler rather than introducing another VM.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum CompactOpcode {
    FunctionPrologueEnd,
    LoadConst,
    LoadLocal,
    LoadLocalOrUndefined,
    StoreLocal,
    AssignLocal,
    ClearLocal,
    LoadGlobal,
    Pop,
    Dup,
    Typeof,
    ToString,
    ToNumeric,
    Unary,
    Update,
    Binary,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    JumpIfNotNullish,
    Call,
    CallResolved,
    Return,
}

/// One compact instruction corresponding to exactly one source [`Op`].
///
/// Branch operands remain source-op indices. The three generic operands leave
/// room for later measured slices without changing the 16-byte format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub(super) struct CompactInstr {
    pub(super) opcode: CompactOpcode,
    pub(super) flags: u8,
    reserved: u16,
    pub(super) a: u32,
    pub(super) b: u32,
    pub(super) c: u32,
}

const _: [(); 16] = [(); std::mem::size_of::<CompactInstr>()];

impl CompactInstr {
    const fn new(opcode: CompactOpcode) -> Self {
        Self {
            opcode,
            flags: 0,
            reserved: 0,
            a: 0,
            b: 0,
            c: 0,
        }
    }

    const fn with_a(opcode: CompactOpcode, a: u32) -> Self {
        Self {
            opcode,
            flags: 0,
            reserved: 0,
            a,
            b: 0,
            c: 0,
        }
    }

    const fn with_operator(opcode: CompactOpcode, flags: u8) -> Self {
        Self {
            opcode,
            flags,
            reserved: 0,
            a: 0,
            b: 0,
            c: 0,
        }
    }
}

/// Fully lowered compact form of one bytecode body.
///
/// Lowering is all-or-nothing. Unsupported operations, invalid operands, and
/// inconsistent stack depths reject the whole body before execution can
/// observe any compact operation.
#[derive(Debug)]
pub(super) struct CompactProgram {
    pub(super) instructions: Box<[CompactInstr]>,
    pub(super) names: Box<[Rc<str>]>,
}

impl CompactProgram {
    pub(super) fn try_compile(bytecode: &Bytecode) -> Option<Self> {
        // Every instruction index must remain representable in branch
        // operands. The executor will continue to use source Op indices as its
        // only instruction-pointer coordinate.
        u32::try_from(bytecode.code.len()).ok()?;

        let mut instructions = Vec::with_capacity(bytecode.code.len());
        let mut names = Vec::new();
        for op in &bytecode.code {
            instructions.push(lower_op(bytecode, op, &mut names)?);
        }

        validate_stack_and_control_flow(&instructions)?;
        debug_assert_eq!(instructions.len(), bytecode.code.len());
        Some(Self {
            instructions: instructions.into_boxed_slice(),
            names: names.into_boxed_slice(),
        })
    }
}

impl Bytecode {
    pub(super) fn compact_program(&self) -> Option<&CompactProgram> {
        self.compact_program
            .get_or_init(|| CompactProgram::try_compile(self).map(Rc::new))
            .as_deref()
    }
}

fn lower_op(bytecode: &Bytecode, op: &Op, names: &mut Vec<Rc<str>>) -> Option<CompactInstr> {
    Some(match op {
        Op::FunctionPrologueEnd => CompactInstr::new(CompactOpcode::FunctionPrologueEnd),
        Op::LoadConst(index) => CompactInstr::with_a(
            CompactOpcode::LoadConst,
            checked_table_index(*index, bytecode.constants.len())?,
        ),
        Op::LoadLocal(slot) => CompactInstr::with_a(
            CompactOpcode::LoadLocal,
            checked_table_index(*slot, bytecode.locals.len())?,
        ),
        Op::LoadLocalOrUndefined(slot) => CompactInstr::with_a(
            CompactOpcode::LoadLocalOrUndefined,
            checked_table_index(*slot, bytecode.locals.len())?,
        ),
        Op::StoreLocal(slot) => CompactInstr::with_a(
            CompactOpcode::StoreLocal,
            checked_table_index(*slot, bytecode.locals.len())?,
        ),
        Op::AssignLocal(slot) => CompactInstr::with_a(
            CompactOpcode::AssignLocal,
            checked_table_index(*slot, bytecode.locals.len())?,
        ),
        Op::ClearLocal(slot) => CompactInstr::with_a(
            CompactOpcode::ClearLocal,
            checked_table_index(*slot, bytecode.locals.len())?,
        ),
        Op::LoadGlobal(name) => {
            let name_index = u32::try_from(names.len()).ok()?;
            names.push(Rc::from(name.as_str()));
            CompactInstr::with_a(CompactOpcode::LoadGlobal, name_index)
        }
        Op::Pop => CompactInstr::new(CompactOpcode::Pop),
        Op::Dup => CompactInstr::new(CompactOpcode::Dup),
        Op::Typeof => CompactInstr::new(CompactOpcode::Typeof),
        Op::ToString => CompactInstr::new(CompactOpcode::ToString),
        Op::ToNumeric => CompactInstr::new(CompactOpcode::ToNumeric),
        Op::Unary(op) => CompactInstr::with_operator(CompactOpcode::Unary, encode_unary(*op)),
        Op::Update(op) => CompactInstr::with_operator(CompactOpcode::Update, encode_update(*op)),
        Op::Binary(op) => CompactInstr::with_operator(CompactOpcode::Binary, encode_binary(*op)),
        Op::Jump(target) => CompactInstr::with_a(
            CompactOpcode::Jump,
            checked_code_index(*target, bytecode.code.len())?,
        ),
        Op::JumpIfFalse(target) => CompactInstr::with_a(
            CompactOpcode::JumpIfFalse,
            checked_code_index(*target, bytecode.code.len())?,
        ),
        Op::JumpIfTrue(target) => CompactInstr::with_a(
            CompactOpcode::JumpIfTrue,
            checked_code_index(*target, bytecode.code.len())?,
        ),
        Op::JumpIfNotNullish(target) => CompactInstr::with_a(
            CompactOpcode::JumpIfNotNullish,
            checked_code_index(*target, bytecode.code.len())?,
        ),
        Op::Call(argc) => CompactInstr::with_a(CompactOpcode::Call, u32::try_from(*argc).ok()?),
        Op::CallResolved(argc) => {
            CompactInstr::with_a(CompactOpcode::CallResolved, u32::try_from(*argc).ok()?)
        }
        Op::Return => CompactInstr::new(CompactOpcode::Return),
        _ => return None,
    })
}

fn checked_table_index(index: usize, len: usize) -> Option<u32> {
    if index >= len {
        return None;
    }
    u32::try_from(index).ok()
}

fn checked_code_index(index: usize, code_len: usize) -> Option<u32> {
    checked_table_index(index, code_len)
}

fn validate_stack_and_control_flow(instructions: &[CompactInstr]) -> Option<()> {
    if instructions.is_empty() {
        return None;
    }

    let mut entry_depths = vec![None; instructions.len()];
    entry_depths[0] = Some(0_u32);
    let mut pending = vec![0_usize];

    while let Some(ip) = pending.pop() {
        let instruction = *instructions.get(ip)?;
        let entry_depth = entry_depths.get(ip).copied().flatten()?;
        let exit_depth = stack_depth_after(instruction, entry_depth)?;

        match instruction.opcode {
            CompactOpcode::Return => {}
            CompactOpcode::Jump => {
                record_entry_depth(
                    usize::try_from(instruction.a).ok()?,
                    exit_depth,
                    &mut entry_depths,
                    &mut pending,
                )?;
            }
            CompactOpcode::JumpIfFalse
            | CompactOpcode::JumpIfTrue
            | CompactOpcode::JumpIfNotNullish => {
                record_entry_depth(
                    usize::try_from(instruction.a).ok()?,
                    exit_depth,
                    &mut entry_depths,
                    &mut pending,
                )?;
                record_fallthrough_depth(ip, exit_depth, &mut entry_depths, &mut pending)?;
            }
            _ => {
                record_fallthrough_depth(ip, exit_depth, &mut entry_depths, &mut pending)?;
            }
        }
    }

    Some(())
}

fn stack_depth_after(instruction: CompactInstr, entry_depth: u32) -> Option<u32> {
    let (required, popped, pushed) = match instruction.opcode {
        CompactOpcode::FunctionPrologueEnd | CompactOpcode::ClearLocal | CompactOpcode::Jump => {
            (0, 0, 0)
        }
        CompactOpcode::LoadConst
        | CompactOpcode::LoadLocal
        | CompactOpcode::LoadLocalOrUndefined
        | CompactOpcode::LoadGlobal => (0, 0, 1),
        CompactOpcode::StoreLocal | CompactOpcode::AssignLocal | CompactOpcode::Pop => (1, 1, 0),
        CompactOpcode::Dup => (1, 0, 1),
        CompactOpcode::Typeof
        | CompactOpcode::ToString
        | CompactOpcode::ToNumeric
        | CompactOpcode::Unary
        | CompactOpcode::Update
        | CompactOpcode::JumpIfFalse
        | CompactOpcode::JumpIfTrue
        | CompactOpcode::JumpIfNotNullish => (1, 0, 0),
        CompactOpcode::Binary => (2, 2, 1),
        CompactOpcode::Call => {
            let consumed = instruction.a.checked_add(1)?;
            (consumed, consumed, 1)
        }
        CompactOpcode::CallResolved => {
            let consumed = instruction.a.checked_add(2)?;
            (consumed, consumed, 1)
        }
        // Return is terminal and the ordinary VM deliberately permits an
        // empty stack, producing undefined in that case.
        CompactOpcode::Return => (0, 0, 0),
    };

    if entry_depth < required {
        return None;
    }
    entry_depth.checked_sub(popped)?.checked_add(pushed)
}

fn record_fallthrough_depth(
    ip: usize,
    depth: u32,
    entry_depths: &mut [Option<u32>],
    pending: &mut Vec<usize>,
) -> Option<()> {
    let next = ip.checked_add(1)?;
    if next >= entry_depths.len() {
        return None;
    }
    record_entry_depth(next, depth, entry_depths, pending)
}

fn record_entry_depth(
    ip: usize,
    depth: u32,
    entry_depths: &mut [Option<u32>],
    pending: &mut Vec<usize>,
) -> Option<()> {
    let entry = entry_depths.get_mut(ip)?;
    match *entry {
        Some(existing) if existing != depth => None,
        Some(_) => Some(()),
        None => {
            *entry = Some(depth);
            pending.push(ip);
            Some(())
        }
    }
}

fn encode_update(op: UpdateOp) -> u8 {
    match op {
        UpdateOp::Increment => 0,
        UpdateOp::Decrement => 1,
    }
}

pub(super) fn decode_update(tag: u8) -> Option<UpdateOp> {
    Some(match tag {
        0 => UpdateOp::Increment,
        1 => UpdateOp::Decrement,
        _ => return None,
    })
}

fn encode_unary(op: UnaryOp) -> u8 {
    match op {
        UnaryOp::Plus => 0,
        UnaryOp::Minus => 1,
        UnaryOp::Not => 2,
        UnaryOp::BitwiseNot => 3,
        UnaryOp::Typeof => 4,
        UnaryOp::Void => 5,
        UnaryOp::Delete => 6,
    }
}

pub(super) fn decode_unary(tag: u8) -> Option<UnaryOp> {
    Some(match tag {
        0 => UnaryOp::Plus,
        1 => UnaryOp::Minus,
        2 => UnaryOp::Not,
        3 => UnaryOp::BitwiseNot,
        4 => UnaryOp::Typeof,
        5 => UnaryOp::Void,
        6 => UnaryOp::Delete,
        _ => return None,
    })
}

fn encode_binary(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Add => 0,
        BinaryOp::Sub => 1,
        BinaryOp::Mul => 2,
        BinaryOp::Div => 3,
        BinaryOp::Rem => 4,
        BinaryOp::Pow => 5,
        BinaryOp::Shl => 6,
        BinaryOp::Shr => 7,
        BinaryOp::UShr => 8,
        BinaryOp::Eq => 9,
        BinaryOp::StrictEq => 10,
        BinaryOp::Ne => 11,
        BinaryOp::StrictNe => 12,
        BinaryOp::BitwiseAnd => 13,
        BinaryOp::BitwiseXor => 14,
        BinaryOp::BitwiseOr => 15,
        BinaryOp::Lt => 16,
        BinaryOp::Le => 17,
        BinaryOp::Gt => 18,
        BinaryOp::Ge => 19,
        BinaryOp::In => 20,
        BinaryOp::Instanceof => 21,
        BinaryOp::LogicalAnd => 22,
        BinaryOp::LogicalOr => 23,
        BinaryOp::NullishCoalescing => 24,
    }
}

pub(super) fn decode_binary(tag: u8) -> Option<BinaryOp> {
    Some(match tag {
        0 => BinaryOp::Add,
        1 => BinaryOp::Sub,
        2 => BinaryOp::Mul,
        3 => BinaryOp::Div,
        4 => BinaryOp::Rem,
        5 => BinaryOp::Pow,
        6 => BinaryOp::Shl,
        7 => BinaryOp::Shr,
        8 => BinaryOp::UShr,
        9 => BinaryOp::Eq,
        10 => BinaryOp::StrictEq,
        11 => BinaryOp::Ne,
        12 => BinaryOp::StrictNe,
        13 => BinaryOp::BitwiseAnd,
        14 => BinaryOp::BitwiseXor,
        15 => BinaryOp::BitwiseOr,
        16 => BinaryOp::Lt,
        17 => BinaryOp::Le,
        18 => BinaryOp::Gt,
        19 => BinaryOp::Ge,
        20 => BinaryOp::In,
        21 => BinaryOp::Instanceof,
        22 => BinaryOp::LogicalAnd,
        23 => BinaryOp::LogicalOr,
        24 => BinaryOp::NullishCoalescing,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    fn nested_function_bytecodes(source: &str) -> Vec<Rc<Bytecode>> {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode =
            super::super::compiler::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .filter_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(Rc::clone(bytecode)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn compact_instruction_layout_is_fixed_at_sixteen_bytes() {
        assert_eq!(std::mem::size_of::<CompactInstr>(), 16);
        assert_eq!(std::mem::align_of::<CompactInstr>(), 4);
    }

    #[test]
    fn lowers_controlflow_recursive_core_as_complete_programs() {
        let functions = nested_function_bytecodes(
            r#"
                function ack(m, n) {
                    if (m == 0) return n + 1;
                    if (n == 0) return ack(m - 1, 1);
                    return ack(m - 1, ack(m, n - 1));
                }
                function fib(n) {
                    if (n < 2) return 1;
                    return fib(n - 2) + fib(n - 1);
                }
                function tak(x, y, z) {
                    if (y >= x) return z;
                    return tak(tak(x - 1, y, z), tak(y - 1, z, x), tak(z - 1, x, y));
                }
            "#,
        );

        assert_eq!(functions.len(), 3);
        for bytecode in functions {
            let program = bytecode
                .compact_program()
                .expect("recursive core should lower completely");
            assert_eq!(program.instructions.len(), bytecode.code.len());
            assert!(
                program
                    .instructions
                    .iter()
                    .any(|instruction| instruction.opcode == CompactOpcode::Call)
            );
            assert!(program.instructions.iter().any(|instruction| matches!(
                instruction.opcode,
                CompactOpcode::JumpIfFalse | CompactOpcode::JumpIfTrue
            )));
        }
    }

    #[test]
    fn preserves_source_instruction_indices_for_branches() {
        let bytecode = Bytecode::new(
            vec![Value::Boolean(true)],
            Vec::new(),
            vec![
                Op::LoadConst(0),
                Op::JumpIfTrue(4),
                Op::Pop,
                Op::Jump(0),
                Op::Return,
            ],
        );
        let program = bytecode.compact_program().expect("valid loop should lower");

        assert_eq!(program.instructions.len(), bytecode.code.len());
        assert_eq!(program.instructions[1].opcode, CompactOpcode::JumpIfTrue);
        assert_eq!(program.instructions[1].a, 4);
        assert_eq!(program.instructions[3].opcode, CompactOpcode::Jump);
        assert_eq!(program.instructions[3].a, 0);
    }

    #[test]
    fn caches_a_successful_program_without_relowering() {
        let bytecode = Bytecode::new(
            vec![Value::Undefined],
            Vec::new(),
            vec![Op::LoadConst(0), Op::Return],
        );
        let first = bytecode.compact_program().expect("program should lower");
        let second = bytecode
            .compact_program()
            .expect("program should stay cached");

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn stores_global_names_in_a_side_table() {
        let bytecode = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![Op::LoadGlobal("callee".to_owned()), Op::Call(0), Op::Return],
        );
        let program = bytecode.compact_program().expect("call should lower");

        assert_eq!(program.names.as_ref(), [Rc::<str>::from("callee")]);
        assert_eq!(program.instructions[0].a, 0);
    }

    #[test]
    fn rejects_an_unsupported_op_even_when_it_is_unreachable() {
        let bytecode = Bytecode::new(
            vec![Value::Undefined],
            Vec::new(),
            vec![Op::LoadConst(0), Op::Return, Op::NewObjectLiteral],
        );

        assert!(bytecode.compact_program().is_none());
    }

    #[test]
    fn rejects_invalid_or_unrepresentable_operands() {
        let invalid_constant = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![Op::LoadConst(usize::MAX), Op::Return],
        );
        let invalid_local = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![Op::LoadLocal(usize::MAX), Op::Return],
        );
        let invalid_target = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![Op::Jump(usize::MAX), Op::Return],
        );
        let invalid_argc = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![
                Op::LoadGlobal("callee".to_owned()),
                Op::Call(usize::MAX),
                Op::Return,
            ],
        );

        assert!(invalid_constant.compact_program().is_none());
        assert!(invalid_local.compact_program().is_none());
        assert!(invalid_target.compact_program().is_none());
        assert!(invalid_argc.compact_program().is_none());
    }

    #[test]
    fn rejects_stack_underflow_join_mismatch_and_reachable_fallthrough() {
        let underflow = Bytecode::new(Vec::new(), Vec::new(), vec![Op::Pop, Op::Return]);
        let mismatched_join = Bytecode::new(
            vec![Value::Boolean(true)],
            Vec::new(),
            vec![Op::LoadConst(0), Op::JumpIfTrue(3), Op::Pop, Op::Return],
        );
        let fallthrough = Bytecode::new(vec![Value::Undefined], Vec::new(), vec![Op::LoadConst(0)]);

        assert!(underflow.compact_program().is_none());
        assert!(mismatched_join.compact_program().is_none());
        assert!(fallthrough.compact_program().is_none());
    }

    #[test]
    fn accepts_equal_depth_short_circuit_join_and_resolved_call() {
        let short_circuit = Bytecode::new(
            vec![Value::Boolean(false), Value::Number(1.0)],
            Vec::new(),
            vec![
                Op::LoadConst(0),
                Op::JumpIfFalse(4),
                Op::Pop,
                Op::LoadConst(1),
                Op::Return,
            ],
        );
        let resolved_call = Bytecode::new(
            vec![Value::Undefined],
            Vec::new(),
            vec![
                Op::LoadConst(0),
                Op::LoadGlobal("callee".to_owned()),
                Op::CallResolved(0),
                Op::Return,
            ],
        );

        assert!(short_circuit.compact_program().is_some());
        assert!(resolved_call.compact_program().is_some());
    }

    #[test]
    fn operator_tags_round_trip_without_layout_dependent_casts() {
        let updates = [UpdateOp::Increment, UpdateOp::Decrement];
        for op in updates {
            assert_eq!(decode_update(encode_update(op)), Some(op));
        }

        let unary = [
            UnaryOp::Plus,
            UnaryOp::Minus,
            UnaryOp::Not,
            UnaryOp::BitwiseNot,
            UnaryOp::Typeof,
            UnaryOp::Void,
            UnaryOp::Delete,
        ];
        for op in unary {
            assert_eq!(decode_unary(encode_unary(op)), Some(op));
        }

        let binary = [
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Rem,
            BinaryOp::Pow,
            BinaryOp::Shl,
            BinaryOp::Shr,
            BinaryOp::UShr,
            BinaryOp::Eq,
            BinaryOp::StrictEq,
            BinaryOp::Ne,
            BinaryOp::StrictNe,
            BinaryOp::BitwiseAnd,
            BinaryOp::BitwiseXor,
            BinaryOp::BitwiseOr,
            BinaryOp::Lt,
            BinaryOp::Le,
            BinaryOp::Gt,
            BinaryOp::Ge,
            BinaryOp::In,
            BinaryOp::Instanceof,
            BinaryOp::LogicalAnd,
            BinaryOp::LogicalOr,
            BinaryOp::NullishCoalescing,
        ];
        for op in binary {
            assert_eq!(decode_binary(encode_binary(op)), Some(op));
        }

        assert_eq!(decode_update(u8::MAX), None);
        assert_eq!(decode_unary(u8::MAX), None);
        assert_eq!(decode_binary(u8::MAX), None);
    }
}
