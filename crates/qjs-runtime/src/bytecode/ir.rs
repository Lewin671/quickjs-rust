use std::rc::Rc;

use qjs_ast::{BinaryOp, Stmt, UnaryOp};

use crate::Value;

#[derive(Clone, Debug)]
pub(super) enum Op {
    LoadConst(usize),
    LoadLocal(usize),
    StoreLocal(usize),
    LoadGlobal(String),
    Pop,
    Dup,
    NewArray(usize),
    NewObject(usize),
    GetProp,
    SetProp,
    Call(usize),
    CallMethod(usize),
    New(usize),
    NewFunction {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        bytecode: Rc<Bytecode>,
        constructable: bool,
    },
    Typeof,
    Unary(UnaryOp),
    Binary(BinaryOp),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    JumpIfNotNullish(usize),
    Return,
    Throw,
}

#[derive(Clone, Debug)]
pub(super) struct Local {
    pub(super) name: String,
    pub(super) hoisted: bool,
}

/// Compiled bytecode for a script.
#[derive(Clone, Debug)]
pub struct Bytecode {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    pub(super) code: Vec<Op>,
}
