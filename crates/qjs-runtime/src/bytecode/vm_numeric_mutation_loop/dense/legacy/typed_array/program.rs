//! Region-entry lowering for the fixed Number TypedArray dense interpreter.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::super::{LegacyDynamicDensePlan, NumberInstruction};
use super::{NumberViewKind, ViewGeometry};

/// A single-level dispatch stream specialized to the leased view codecs.
/// Memory opcodes carry their exact Number TypedArray codec, while arithmetic
/// opcodes carry the already-decoded operation. The hot loop therefore never
/// dispatches on `NativeFunction` or constructs a temporary `Value`.
#[derive(Clone, Copy)]
pub(super) enum TypedInstruction {
    Constant(f64),
    LoadLocal(usize),
    LoadU8 {
        receiver: usize,
        index: usize,
    },
    LoadI8 {
        receiver: usize,
        index: usize,
    },
    LoadU16 {
        receiver: usize,
        index: usize,
    },
    LoadI16 {
        receiver: usize,
        index: usize,
    },
    LoadU32 {
        receiver: usize,
        index: usize,
    },
    LoadI32 {
        receiver: usize,
        index: usize,
    },
    LoadF32 {
        receiver: usize,
        index: usize,
    },
    LoadF64 {
        receiver: usize,
        index: usize,
    },
    StoreU8 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreI8 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreU8Clamped {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreU16 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreI16 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreU32 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreI32 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreF32 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    StoreF64 {
        receiver: usize,
        index: usize,
        value: usize,
    },
    Add {
        left: usize,
        right: usize,
    },
    Sub {
        left: usize,
        right: usize,
    },
    Mul {
        left: usize,
        right: usize,
    },
    Div {
        left: usize,
        right: usize,
    },
    Rem {
        left: usize,
        right: usize,
    },
    Shl {
        left: usize,
        right: usize,
    },
    Shr {
        left: usize,
        right: usize,
    },
    UShr {
        left: usize,
        right: usize,
    },
    BitwiseAnd {
        left: usize,
        right: usize,
    },
    BitwiseXor {
        left: usize,
        right: usize,
    },
    BitwiseOr {
        left: usize,
        right: usize,
    },
    Plus {
        value: usize,
    },
    Minus {
        value: usize,
    },
    BitwiseNot {
        value: usize,
    },
    Increment {
        value: usize,
    },
    Decrement {
        value: usize,
    },
}

#[derive(Clone, Copy)]
pub(super) struct TypedSunkStore {
    pub(super) kind: NumberViewKind,
    pub(super) receiver: usize,
    pub(super) index: usize,
    pub(super) value: usize,
}

pub(super) struct TypedProgram {
    pub(super) operations: Vec<TypedInstruction>,
    pub(super) constant_count: usize,
    pub(super) dynamic_start: usize,
    pub(super) sunk_store: Option<TypedSunkStore>,
}

impl TypedProgram {
    #[inline(never)]
    pub(super) fn lower(plan: &LegacyDynamicDensePlan, views: &[ViewGeometry]) -> Option<Self> {
        let input_prefix = plan.input_prefix?;
        let dynamic_start = input_prefix.validated_dynamic_start(plan.operations.len())?;
        let mut operations = Vec::with_capacity(plan.operations.len());
        for operation in &plan.operations {
            operations.push(match *operation {
                NumberInstruction::Constant(value) => TypedInstruction::Constant(value),
                NumberInstruction::LoadLocal(local) => TypedInstruction::LoadLocal(local),
                NumberInstruction::DenseLoad { receiver, index } => {
                    match views.get(receiver)?.kind {
                        NumberViewKind::Uint8 | NumberViewKind::Uint8Clamped => {
                            TypedInstruction::LoadU8 { receiver, index }
                        }
                        NumberViewKind::Int8 => TypedInstruction::LoadI8 { receiver, index },
                        NumberViewKind::Uint16 => TypedInstruction::LoadU16 { receiver, index },
                        NumberViewKind::Int16 => TypedInstruction::LoadI16 { receiver, index },
                        NumberViewKind::Uint32 => TypedInstruction::LoadU32 { receiver, index },
                        NumberViewKind::Int32 => TypedInstruction::LoadI32 { receiver, index },
                        NumberViewKind::Float32 => TypedInstruction::LoadF32 { receiver, index },
                        NumberViewKind::Float64 => TypedInstruction::LoadF64 { receiver, index },
                    }
                }
                NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                } => match views.get(receiver)?.kind {
                    NumberViewKind::Uint8 => TypedInstruction::StoreU8 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Int8 => TypedInstruction::StoreI8 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Uint8Clamped => TypedInstruction::StoreU8Clamped {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Uint16 => TypedInstruction::StoreU16 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Int16 => TypedInstruction::StoreI16 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Uint32 => TypedInstruction::StoreU32 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Int32 => TypedInstruction::StoreI32 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Float32 => TypedInstruction::StoreF32 {
                        receiver,
                        index,
                        value,
                    },
                    NumberViewKind::Float64 => TypedInstruction::StoreF64 {
                        receiver,
                        index,
                        value,
                    },
                },
                NumberInstruction::Binary {
                    operation,
                    left,
                    right,
                } => match operation {
                    BinaryOp::Add => TypedInstruction::Add { left, right },
                    BinaryOp::Sub => TypedInstruction::Sub { left, right },
                    BinaryOp::Mul => TypedInstruction::Mul { left, right },
                    BinaryOp::Div => TypedInstruction::Div { left, right },
                    BinaryOp::Rem => TypedInstruction::Rem { left, right },
                    BinaryOp::Shl => TypedInstruction::Shl { left, right },
                    BinaryOp::Shr => TypedInstruction::Shr { left, right },
                    BinaryOp::UShr => TypedInstruction::UShr { left, right },
                    BinaryOp::BitwiseAnd => TypedInstruction::BitwiseAnd { left, right },
                    BinaryOp::BitwiseXor => TypedInstruction::BitwiseXor { left, right },
                    BinaryOp::BitwiseOr => TypedInstruction::BitwiseOr { left, right },
                    _ => return None,
                },
                NumberInstruction::Unary { operation, value } => match operation {
                    UnaryOp::Plus => TypedInstruction::Plus { value },
                    UnaryOp::Minus => TypedInstruction::Minus { value },
                    UnaryOp::BitwiseNot => TypedInstruction::BitwiseNot { value },
                    _ => return None,
                },
                NumberInstruction::Update { operation, value } => match operation {
                    UpdateOp::Increment => TypedInstruction::Increment { value },
                    UpdateOp::Decrement => TypedInstruction::Decrement { value },
                },
            });
        }
        let sunk_store = match plan.sunk_store {
            Some(store) => Some(TypedSunkStore {
                kind: views.get(store.receiver)?.kind,
                receiver: store.receiver,
                index: store.index,
                value: store.value,
            }),
            None => None,
        };
        Some(Self {
            operations,
            constant_count: input_prefix.constant_count,
            dynamic_start,
            sunk_store,
        })
    }
}
