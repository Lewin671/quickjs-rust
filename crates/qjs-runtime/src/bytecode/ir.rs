use std::{
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use qjs_ast::{BinaryOp, FunctionParams, ObjectPropertyKind, UnaryOp, UpdateOp};

use crate::Value;

#[derive(Clone, Debug)]
pub(super) enum Op {
    LoadConst(usize),
    LoadLocal(usize),
    LoadLocalOrUndefined(usize),
    StoreLocal(usize),
    ClearLocal(usize),
    DefineGlobalVar(String),
    LoadGlobal(String),
    StoreGlobalStrict(String),
    TypeofGlobal(String),
    Pop,
    Dup,
    NewArray {
        elements: Vec<ArrayElementKind>,
    },
    NewTemplateObject {
        cooked: Vec<String>,
        raw: Vec<String>,
    },
    NewObject(Vec<ObjectPropertyKind>),
    EnumerateKeys,
    GetProp,
    SetProp {
        is_strict: bool,
    },
    DeleteProp,
    Call(usize),
    CallMethod(usize),
    New(usize),
    NewFunction {
        name: Option<String>,
        params: FunctionParams,
        local_names: Vec<String>,
        bytecode: Rc<Bytecode>,
        constructable: bool,
        is_strict: bool,
        lexical_this: bool,
    },
    Typeof,
    ToString,
    ToNumeric,
    Unary(UnaryOp),
    Update(UpdateOp),
    Binary(BinaryOp),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    JumpIfNotNullish(usize),
    EnterTry {
        catch: Option<usize>,
        finally: Option<usize>,
        catch_scope: Option<CatchScope>,
    },
    ExitTry,
    EndFinally,
    Return,
    Throw,
}

#[derive(Clone, Debug)]
pub(super) enum ArrayElementKind {
    Expr,
    Elision,
    Spread,
}

#[derive(Clone, Debug)]
pub(super) enum CatchScope {
    Clear { slot: usize },
}

#[derive(Clone, Debug)]
pub(super) struct Local {
    pub(super) name: String,
    pub(super) hoisted: bool,
    pub(super) mutable: bool,
    pub(super) from_env: bool,
}

/// Compiled bytecode for a script.
#[derive(Clone, Debug)]
pub struct Bytecode {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    local_slots: HashMap<String, usize>,
    global_names: Vec<String>,
    pub(super) code: Vec<Op>,
}

impl Bytecode {
    pub(super) fn new(constants: Vec<Value>, locals: Vec<Local>, code: Vec<Op>) -> Self {
        Self {
            constants,
            local_slots: collect_local_slots(&locals),
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

    pub(crate) fn local_slot(&self, name: &str) -> Option<usize> {
        self.local_slots.get(name).copied()
    }

    pub(crate) fn requires_scope_call_bindings(&self) -> bool {
        self.code.iter().any(|op| {
            matches!(
                op,
                Op::Call(_)
                    | Op::CallMethod(_)
                    | Op::New(_)
                    | Op::NewFunction { .. }
                    | Op::StoreGlobalStrict(_)
            )
            || matches!(op, Op::StoreLocal(slot) if self.locals.get(*slot).is_some_and(|local| local.from_env))
        })
    }
}

fn collect_local_slots(locals: &[Local]) -> HashMap<String, usize> {
    let mut slots = HashMap::new();
    for (slot, local) in locals.iter().enumerate() {
        slots.entry(local.name.clone()).or_insert(slot);
    }
    slots
}

fn collect_global_names(code: &[Op]) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_global_names_from_ops(code, &mut names);
    names.into_iter().collect()
}

fn collect_global_names_from_ops(code: &[Op], names: &mut BTreeSet<String>) {
    for op in code {
        match op {
            Op::LoadGlobal(name) | Op::StoreGlobalStrict(name) | Op::TypeofGlobal(name) => {
                names.insert(name.clone());
            }
            Op::NewFunction { bytecode, .. } => {
                names.extend(bytecode.global_names().iter().cloned());
            }
            _ => {}
        }
    }
}
