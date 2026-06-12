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
    NewObject(Vec<ObjectPropertyMeta>),
    EnumerateKeys,
    /// Replaces an iterable on the stack with its iterator object.
    GetIterator,
    /// Replaces an iterable on the stack with its async iterator object,
    /// following GetIterator(obj, async): looks up `Symbol.asyncIterator`, and
    /// when absent wraps the sync iterator via CreateAsyncFromSyncIterator. Used
    /// by `for await ... of`.
    GetAsyncIterator,
    /// Processes the awaited result of an async iterator `next()` call (the
    /// result object is on top of the stack after an `Op::Await`). Validates it
    /// is an object, records `done` in `done_slot`, and pushes the `value`. Used
    /// by `for await ... of`.
    AsyncIteratorComplete {
        done_slot: usize,
    },
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
    /// Reads a private member `obj.#name`: pops the object, resolves `#name`
    /// against the current home object's private environment, and pushes the
    /// field value, the shared method, or the result of the getter. Throws a
    /// TypeError when the object lacks the private name's brand.
    GetPrivate(String),
    /// Writes a private member `obj.#name = value`: pops the value and object,
    /// stores the field or runs the setter. Throws a TypeError when the object
    /// lacks the brand or the member is read-only (method/getter-only).
    SetPrivate(String),
    /// Evaluates `#name in obj`: pops the object and pushes a boolean brand
    /// check. Never throws.
    PrivateIn(String),
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
        is_generator: bool,
        is_async: bool,
    },
    /// Builds a class constructor function object, wires its `prototype` and
    /// `constructor` properties, and installs prototype methods. Pushes the
    /// constructor function value.
    NewClass {
        name: Option<String>,
        constructor: ClassConstructorDef,
        /// Class elements (methods, accessors, and fields) in source order.
        elements: Vec<ClassElementDef>,
        /// Private elements (fields, methods, accessors) in source order. These
        /// are not ordinary properties; they install into per-object private
        /// storage keyed by fresh per-evaluation private-name identities.
        private_elements: Vec<ClassPrivateElementDef>,
        /// Number of computed-key values pushed onto the stack before this op,
        /// in member order.
        computed_key_count: usize,
        /// Whether the class has an `extends` heritage clause. When set, the
        /// heritage value was pushed onto the stack before the computed keys.
        has_heritage: bool,
    },
    /// Reads `super.<key>`: looks the property up on the current method's home
    /// object prototype, using `this` as the receiver. Pushes the value.
    SuperGet {
        key: String,
    },
    /// Reads `super[expr]`: pops the key from the stack, then behaves like
    /// `SuperGet`.
    SuperGetComputed,
    /// Loads `super.<key>` as a method, pushing the current `this` (receiver)
    /// then the resolved callee, so a following `CallResolved` invokes it with
    /// the right receiver.
    SuperMethod {
        key: String,
    },
    /// Like `SuperMethod` but pops the computed key from the stack first.
    SuperMethodComputed,
    /// Calls a pre-resolved callee. The stack holds `[receiver, callee,
    /// args...]`; pops the arguments, callee, and receiver, then calls.
    CallResolved(usize),
    /// Like `CallResolved` but takes the arguments from an array on the stack:
    /// `[receiver, callee, args_array]`.
    CallResolvedSpread,
    /// Calls the super constructor with the given fixed argument count, binds
    /// the result as `this`, and pushes it. Enforces the derived-constructor
    /// `this` TDZ.
    SuperCall(usize),
    /// Like `SuperCall` but takes the arguments from an array on the stack.
    SuperCallSpread,
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
    /// Marks the boundary between parameter instantiation and the function body.
    /// Emitted once per function, after the parameter-binding prologue. Ordinary
    /// function and script runs treat it as a no-op; when starting a generator or
    /// async generator the VM runs to this op and suspends, so a parameter-binding
    /// error throws synchronously at the call (before the generator object is
    /// created) per `FunctionDeclarationInstantiation`.
    FunctionPrologueEnd,
    /// Suspends a generator body, yielding the value on top of the stack. When
    /// the generator is resumed, the resume value (or an injected
    /// return/throw completion) is delivered at this point.
    Yield,
    /// Suspends an async function or async generator body at an `await`,
    /// awaiting the value on top of the stack. Distinct from `Op::Yield` so an
    /// async generator can tell a consumer-facing `yield` (driven by
    /// next/return/throw) apart from an `await` (driven by a promise reaction).
    /// Plain generators never emit this op; plain async functions treat
    /// `Await` and `Yield` suspensions identically.
    Await,
    /// Delegates to an inner iterable (`yield* expr`) per ES2023 14.4.14. The
    /// iterable is on top of the stack on first entry; the op gets its
    /// iterator and `next` method (stored in the two slots so they survive a
    /// suspension), then drives the inner iterator: each non-done inner result
    /// suspends the OUTER generator yielding that result object unwrapped, and
    /// an `next`/`return`/`throw` resume is forwarded to the inner iterator.
    /// When the inner iterator is done the op leaves the inner result's `value`
    /// on the stack as the `yield*` expression value and execution continues.
    YieldDelegate {
        iterator_slot: usize,
        next_slot: usize,
    },
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

/// A class element in source order: a method/accessor or a field. Both kinds
/// may carry a computed key whose value was pushed before `NewClass`.
#[derive(Clone, Debug)]
pub(super) enum ClassElementDef {
    Method(ClassMethodDef),
    Field(ClassFieldDef),
    /// A `static { ... }` initialization block, run at class definition with
    /// `this` = the constructor, in source order with static fields.
    StaticBlock(ClassStaticBlockDef),
}

/// Compiled `static { ... }` block: a parameterless thunk whose body runs with
/// `this` = the constructor (its home object is the constructor too, so
/// `super.x` resolves against the constructor's prototype).
#[derive(Clone, Debug)]
pub(super) struct ClassStaticBlockDef {
    pub(super) local_names: Vec<String>,
    pub(super) bytecode: Rc<Bytecode>,
}

/// A private class element in source order. Private names are keyed by source
/// text (`name`, without the `#`); a fresh identity is minted at class
/// evaluation. Accessor halves for the same name merge into one binding.
#[derive(Clone, Debug)]
pub(super) enum ClassPrivateElementDef {
    /// A private field. The initializer thunk runs at construction (instance)
    /// or class definition (static); `None` installs `undefined`.
    Field {
        name: String,
        is_static: bool,
        initializer: Option<ClassFieldInitializerDef>,
    },
    /// A private method shared by all instances/the constructor.
    Method {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
    /// A private getter half.
    Getter {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
    /// A private setter half.
    Setter {
        name: String,
        is_static: bool,
        def: ClassMethodDef,
    },
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
    /// Whether the method is a generator method (`*m() {}`).
    pub(super) is_generator: bool,
    /// Whether the method is an async method (`async m() {}`).
    pub(super) is_async: bool,
}

/// Compiled definition of a public class field. The initializer is compiled
/// as a thunk evaluated with `this` bound (the instance for an instance field,
/// the constructor for a static field); `None` installs `undefined`.
#[derive(Clone, Debug)]
pub(super) struct ClassFieldDef {
    pub(super) key: ClassMemberKeyDef,
    pub(super) is_static: bool,
    pub(super) initializer: Option<ClassFieldInitializerDef>,
}

/// Compiled field initializer thunk: a parameterless function body returning
/// the field value.
#[derive(Clone, Debug)]
pub(super) struct ClassFieldInitializerDef {
    pub(super) local_names: Vec<String>,
    pub(super) bytecode: Rc<Bytecode>,
}

#[derive(Clone, Debug)]
pub(super) enum ArrayElementKind {
    Expr,
    Elision,
    Spread,
}

/// Per-property metadata for an object literal, paired with each
/// key/value pair on the operand stack consumed by `Op::NewObject`.
#[derive(Clone, Copy, Debug)]
pub(super) struct ObjectPropertyMeta {
    pub(super) kind: ObjectPropertyKind,
    /// Set for the `{ __proto__: expr }` prototype special form (Annex B.3.1).
    pub(super) is_proto_setter: bool,
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
    /// Whether this bytecode is global script code (top-level scripts and
    /// eval bodies). Global `var`/function bindings live in the realm, and
    /// `this` resolves to the realm global; function bodies resolve `this`
    /// from their own frame.
    pub(super) global_scope: bool,
    pub(super) code: Vec<Op>,
}

impl Bytecode {
    pub(super) fn new(constants: Vec<Value>, locals: Vec<Local>, code: Vec<Op>) -> Self {
        Self::with_scope(constants, locals, code, false)
    }

    pub(super) fn with_scope(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
    ) -> Self {
        Self {
            constants,
            local_slots: collect_local_slots(&locals),
            locals,
            global_names: collect_global_names(&code),
            sloppy_global_assignment_names: collect_sloppy_global_assignment_names(&code),
            global_scope,
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

    /// Whether the body can create a nested closure, class, generator, or async
    /// function whose activation snapshot reads the per-call captured-env Rc. When
    /// false, the activation captured env is never read, so the caller can skip
    /// cloning the whole frame env into it.
    pub(crate) fn creates_closures(&self) -> bool {
        self.code
            .iter()
            .any(|op| matches!(op, Op::NewFunction { .. } | Op::NewClass { .. }))
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
                    | Op::SuperCall(_)
                    | Op::SuperCallSpread
                    | Op::SuperMethod { .. }
                    | Op::SuperMethodComputed
                    | Op::CallResolved(_)
                    | Op::CallResolvedSpread
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
                elements,
                private_elements,
                ..
            } => {
                names.extend(constructor.bytecode.global_names().iter().cloned());
                for element in elements {
                    match element {
                        ClassElementDef::Method(method) => {
                            names.extend(method.bytecode.global_names().iter().cloned());
                        }
                        ClassElementDef::Field(field) => {
                            if let Some(initializer) = &field.initializer {
                                names.extend(initializer.bytecode.global_names().iter().cloned());
                            }
                        }
                        ClassElementDef::StaticBlock(block) => {
                            names.extend(block.bytecode.global_names().iter().cloned());
                        }
                    }
                }
                for element in private_elements {
                    match element {
                        ClassPrivateElementDef::Field { initializer, .. } => {
                            if let Some(initializer) = initializer {
                                names.extend(initializer.bytecode.global_names().iter().cloned());
                            }
                        }
                        ClassPrivateElementDef::Method { def, .. }
                        | ClassPrivateElementDef::Getter { def, .. }
                        | ClassPrivateElementDef::Setter { def, .. } => {
                            names.extend(def.bytecode.global_names().iter().cloned());
                        }
                    }
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
