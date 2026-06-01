use std::{collections::BTreeSet, rc::Rc};

use qjs_ast::{BinaryOp, UnaryOp};

use crate::Value;

#[derive(Clone, Debug)]
pub(super) enum Op {
    LoadConst(usize),
    LoadLocal(usize),
    StoreLocal(usize),
    LoadGlobal(String),
    TypeofGlobal(String),
    Pop,
    Dup,
    NewArray(usize),
    NewObject(usize),
    EnumerateKeys,
    GetProp,
    SetProp,
    DeleteProp,
    Call(usize),
    CallMethod(usize),
    New(usize),
    NewFunction {
        name: Option<String>,
        params: Vec<String>,
        local_names: Vec<String>,
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
    EnterTry {
        catch: Option<usize>,
        finally: Option<usize>,
    },
    ExitTry,
    EndFinally,
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
    global_names: Vec<String>,
    pub(super) code: Vec<Op>,
}

impl Bytecode {
    pub(super) fn new(constants: Vec<Value>, locals: Vec<Local>, code: Vec<Op>) -> Self {
        Self {
            constants,
            locals,
            global_names: collect_global_names(&code),
            code,
        }
    }

    pub(crate) fn global_names(&self) -> &[String] {
        &self.global_names
    }

    pub(crate) fn local_names(&self) -> impl Iterator<Item = &str> {
        self.locals.iter().map(|local| local.name.as_str())
    }
}

fn collect_global_names(code: &[Op]) -> Vec<String> {
    code.iter()
        .filter_map(|op| match op {
            Op::LoadGlobal(name) | Op::TypeofGlobal(name) => Some(name.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
