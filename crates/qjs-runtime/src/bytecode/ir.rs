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
    StoreLocalOrGlobalSloppy {
        slot: usize,
        name: String,
    },
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
    /// Replaces an iterable on the stack with its iterator object.
    GetIterator,
    /// Pops a `next` method and an iterator, advances the iterator one step,
    /// and pushes the step value (or undefined when exhausted). The boolean
    /// local at `done_slot` records whether the iterator is done; it is also
    /// set when the step itself fails, so abrupt completions skip the close.
    IteratorStep {
        done_slot: usize,
    },
    /// Pops a `next` method and an iterator and pushes an array of the
    /// remaining iterator values, honoring and updating `done_slot`.
    IteratorRest {
        done_slot: usize,
    },
    /// Replaces a value on the stack with an object holding its remaining
    /// own enumerable string-keyed properties, excluding the listed keys.
    ObjectRestExcluding {
        excluded: Vec<String>,
    },
    /// Throws a TypeError when the top of the stack is undefined or null.
    RequireObjectCoercible,
    GetProp,
    SetProp {
        is_strict: bool,
    },
    DeleteProp,
    Call(usize),
    CallMethod(usize),
    CallSpread,
    CallMethodSpread,
    /// Pops an iterator and calls its `return` method when present. With
    /// `swallow` set, errors from the close are ignored (the close happens
    /// while another abrupt completion is already propagating).
    IteratorClose {
        swallow: bool,
    },
    New(usize),
    NewSpread,
    NewFunction {
        name: Option<String>,
        params: FunctionParams,
        local_names: Vec<String>,
        bytecode: Rc<Bytecode>,
        constructable: bool,
        is_strict: bool,
        lexical_this: bool,
        lexical_arguments: bool,
    },
    /// Builds a class constructor function object, wires its `prototype` and
    /// `constructor` properties, and installs prototype methods. Pushes the
    /// constructor function value.
    NewClass {
        name: Option<String>,
        constructor: ClassConstructorDef,
        methods: Vec<ClassMethodDef>,
        /// Number of computed-key values pushed onto the stack before this op,
        /// in member order.
        computed_key_count: usize,
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

/// Compiled definition of a class constructor.
#[derive(Clone, Debug)]
pub(super) struct ClassConstructorDef {
    pub(super) name: Option<String>,
    pub(super) params: FunctionParams,
    pub(super) local_names: Vec<String>,
    pub(super) bytecode: Rc<Bytecode>,
}

/// Whether a class member key is a literal name or a computed expression
/// whose value is taken from the stack at class-evaluation time.
#[derive(Clone, Debug)]
pub(super) enum ClassMemberKeyDef {
    /// A statically known string key.
    Literal(String),
    /// A computed key: the value was pushed onto the stack before `NewClass`.
    Computed,
}

/// The kind of a class method member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ClassMethodKind {
    Method,
    Getter,
    Setter,
}

/// Compiled definition of a class method or accessor.
#[derive(Clone, Debug)]
pub(super) struct ClassMethodDef {
    pub(super) key: ClassMemberKeyDef,
    pub(super) method_kind: ClassMethodKind,
    pub(super) is_static: bool,
    /// Function `name`, when statically known. Computed keys derive the name
    /// from the evaluated key at runtime.
    pub(super) name: Option<String>,
    pub(super) params: FunctionParams,
    pub(super) local_names: Vec<String>,
    pub(super) bytecode: Rc<Bytecode>,
}

#[derive(Clone, Debug)]
pub(super) enum ArrayElementKind {
    Expr,
    Elision,
    Spread,
}

#[derive(Clone, Debug)]
pub(super) enum CatchScope {
    Clear { slots: Vec<usize> },
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
    sloppy_global_assignment_names: Vec<String>,
    pub(super) code: Vec<Op>,
}

impl Bytecode {
    pub(super) fn new(constants: Vec<Value>, locals: Vec<Local>, code: Vec<Op>) -> Self {
        Self {
            constants,
            local_slots: collect_local_slots(&locals),
            locals,
            global_names: collect_global_names(&code),
            sloppy_global_assignment_names: collect_sloppy_global_assignment_names(&code),
            code,
        }
    }

    pub(crate) fn global_names(&self) -> &[String] {
        &self.global_names
    }

    pub(crate) fn sloppy_global_assignment_names(&self) -> &[String] {
        &self.sloppy_global_assignment_names
    }

    pub(crate) fn local_names(&self) -> impl Iterator<Item = &str> {
        self.locals.iter().map(|local| local.name.as_str())
    }

    pub(crate) fn hoisted_local_names(&self) -> impl Iterator<Item = &str> {
        self.locals
            .iter()
            .filter(|local| local.hoisted)
            .map(|local| local.name.as_str())
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
                    | Op::CallSpread
                    | Op::CallMethodSpread
                    | Op::New(_)
                    | Op::NewSpread
                    | Op::NewFunction { .. }
                    | Op::NewClass { .. }
                    | Op::StoreGlobalStrict(_)
                    | Op::StoreLocalOrGlobalSloppy { .. }
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
            Op::StoreLocalOrGlobalSloppy { name, .. } => {
                names.insert(name.clone());
            }
            Op::NewFunction { bytecode, .. } => {
                names.extend(bytecode.global_names().iter().cloned());
            }
            Op::NewClass {
                constructor,
                methods,
                ..
            } => {
                names.extend(constructor.bytecode.global_names().iter().cloned());
                for method in methods {
                    names.extend(method.bytecode.global_names().iter().cloned());
                }
            }
            _ => {}
        }
    }
}

fn collect_sloppy_global_assignment_names(code: &[Op]) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_sloppy_global_assignment_names_from_ops(code, &mut names);
    names.into_iter().collect()
}

fn collect_sloppy_global_assignment_names_from_ops(code: &[Op], names: &mut BTreeSet<String>) {
    for op in code {
        if let Op::StoreLocalOrGlobalSloppy { name, .. } = op {
            names.insert(name.clone());
        }
    }
}
