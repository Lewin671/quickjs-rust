use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};

use qjs_ast::{BinaryOp, FunctionParams, ObjectPropertyKind, UnaryOp, UpdateOp};

use crate::Value;

#[derive(Clone, Debug)]
pub(super) enum Op {
    LoadConst(usize),
    LoadLocal(usize),
    LoadLocalOrUndefined(usize),
    LoadNewTarget,
    AppendStringLiteralLocal {
        slot: usize,
        value: String,
    },
    AppendStringLiteralGlobal {
        name: String,
        value: String,
        is_strict: bool,
    },
    StoreLocal(usize),
    AssignLocal(usize),
    ClearLocal(usize),
    DefineGlobalVar(String),
    LoadGlobal(String),
    StoreGlobalStrict(String),
    StoreGlobalSloppy(String),
    StoreLocalOrGlobalSloppy {
        slot: usize,
        name: String,
    },
    TypeofGlobal(String),
    /// Pushes the with-object on top of the with-object stack. The object is
    /// popped from the operand stack. Used when entering a `with` body.
    EnterWith,
    /// Pops the innermost with-object off the with-object stack. Used when
    /// leaving a `with` body (normally or via break/continue/return).
    ExitWith,
    /// Loads an identifier from inside a `with` body: consults the with-object
    /// stack (honoring `Symbol.unscopables` and the prototype chain) first, then
    /// falls back to the local slot when present, otherwise the global scope.
    LoadIdentWith {
        name: String,
        slot: Option<usize>,
        /// Strict-mode flag of the reading code: GetBindingValue throws a
        /// ReferenceError (rather than yielding undefined) when the with-object
        /// no longer has the binding. Compile-time because a nested strict
        /// function reads a binding from an enclosing sloppy `with`.
        is_strict: bool,
    },
    /// Resolves an identifier assignment target inside a `with` body before
    /// the RHS runs. Stores the selected with-object, or undefined when the
    /// assignment should fall back to the ordinary lexical/global target.
    ResolveIdentWith {
        name: String,
        slot: Option<usize>,
        object_slot: usize,
    },
    /// Loads from the target previously captured by `ResolveIdentWith`.
    LoadResolvedIdentWith {
        name: String,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
    },
    /// Stores to an identifier from inside a `with` body, mirroring
    /// `LoadIdentWith` resolution. `is_strict` selects strict vs sloppy global
    /// store semantics for the fallback.
    StoreIdentWith {
        name: String,
        slot: Option<usize>,
        is_strict: bool,
    },
    /// Stores to the target previously captured by `ResolveIdentWith`.
    StoreResolvedIdentWith {
        name: String,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
    },
    /// `typeof name` from inside a `with` body, never throwing for an unresolved
    /// name. Mirrors `LoadIdentWith` resolution.
    TypeofIdentWith {
        name: String,
        slot: Option<usize>,
    },
    Pop,
    Dup,
    NewArray {
        elements: Vec<ArrayElementKind>,
    },
    NewTemplateObject {
        site: usize,
        cooked: Vec<Option<String>>,
        raw: Vec<String>,
    },
    NewObjectLiteral,
    /// Opens a `using` disposal scope: subsequent register ops add to it until
    /// the matching `DisposeScope`.
    EnterDisposableScope,
    /// Registers the value on top of the stack (a `using` initializer result,
    /// left in place) as a disposable resource in the current scope: resolves
    /// `Symbol.dispose` once. `null`/`undefined` are ignored; a non-object or a
    /// missing/non-callable `dispose` is a TypeError.
    RegisterDisposable,
    /// Registers an `await using` initializer result in the current scope:
    /// resolves `Symbol.asyncDispose` first, falling back to `Symbol.dispose`.
    RegisterAsyncDisposable,
    /// Closes the current disposal scope, disposing its resources LIFO. A
    /// dispose failure while a throw is already propagating is wrapped in a
    /// `SuppressedError`. Async scopes leave the final awaited value on the
    /// stack so the finally body can suspend before completing.
    DisposeScope {
        await_async: bool,
    },
    /// Names an anonymous object-literal function/accessor from its computed
    /// key. Stack (unchanged): `[..., key, function]`. A symbol key yields
    /// `[description]`; accessors are prefixed with `get `/`set `.
    SetComputedFunctionName(ComputedNameKind),
    DefineObjectProperty(ObjectPropertyMeta),
    CopyObjectSpread,
    EnumerateKeys,
    ForInKeyIsEnumerable,
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
        excluded: Vec<ObjectRestExclusion>,
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
    /// `delete obj[key]`. In strict mode a failed deletion of a
    /// non-configurable property throws a TypeError instead of returning false.
    DeleteProp {
        is_strict: bool,
    },
    /// `delete identifier` in non-strict mode: attempts to delete the binding
    /// from the global object. Returns false for non-configurable or
    /// undeletable bindings (var declarations), true if successfully deleted.
    DeleteIdent(String),
    /// `delete identifier` inside a `with` body in non-strict mode: consults
    /// the with-object stack first, then falls back to global deletion.
    DeleteIdentWith {
        name: String,
        slot: Option<usize>,
    },
    /// Throws a TypeError when the top stack value is not callable. Tagged
    /// templates use this after resolving the tag and before evaluating
    /// substitutions.
    RequireCallable,
    Call(usize),
    CallDirectEval {
        argc: usize,
        is_strict: bool,
    },
    CallSpread,
    CallDirectEvalSpread {
        is_strict: bool,
    },
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
        has_name_binding: bool,
        immutable_name_binding: bool,
        params: FunctionParams,
        local_names: Vec<String>,
        lexical_captures: Vec<(String, usize)>,
        bytecode: Rc<Bytecode>,
        constructable: bool,
        is_strict: bool,
        lexical_this: bool,
        lexical_arguments: bool,
        is_generator: bool,
        is_async: bool,
        /// The function's original source text (for `Function.prototype
        /// .toString`), or `None` to fall back to the `[native code]` form.
        source_text: Option<Rc<str>>,
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
        /// Computed member keys in source order. Most are pre-evaluated by the
        /// surrounding bytecode and left on the stack; keys that need the class
        /// private environment are deferred until `NewClass` runs.
        computed_keys: Vec<ClassComputedKeyDef>,
        /// Whether the class has an `extends` heritage clause. When set, the
        /// heritage value was pushed onto the stack before this op.
        has_heritage: bool,
    },
    /// Reads `super.<key>`: looks the property up on the current method's home
    /// object prototype, using `this` as the receiver. Pushes the value.
    SuperGet {
        key: String,
    },
    /// Captures the current super receiver and lookup base for `super[expr]`
    /// before evaluating the computed key expression. Pushes
    /// `[receiver, lookup_base]`.
    SuperReference,
    /// Reads `super[expr]`: pops the key from the stack, then behaves like
    /// `SuperGet` using the previously captured `[receiver, lookup_base]`.
    SuperGetComputed,
    /// Writes `super.<key> = value`: pops the value, resolves the current
    /// home object's prototype as the target, and uses current `this` as the
    /// receiver. Pushes the assigned value.
    SuperSet {
        key: String,
        is_strict: bool,
    },
    /// Writes `super[expr] = value`: pops value then key, then behaves like
    /// `SuperSet`.
    SuperSetComputed {
        is_strict: bool,
    },
    /// Loads `super.<key>` as a method, pushing the current `this` (receiver)
    /// then the resolved callee, so a following `CallResolved` invokes it with
    /// the right receiver.
    SuperMethod {
        key: String,
    },
    /// Like `SuperMethod` but pops the computed key and previously captured
    /// `[receiver, lookup_base]` from the stack first.
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
    ToPropertyKey,
    ToNumeric,
    Unary(UnaryOp),
    Update(UpdateOp),
    Binary(BinaryOp),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    JumpIfNotNullish(usize),
    /// A break/continue that must route through a finally block before
    /// reaching its target. The VM pops the try frame, sets a pending jump
    /// target, and transfers control to the finally block. EndFinally then
    /// resumes the jump.
    AbruptJump(usize),
    /// Creates fresh upvalue cells for per-iteration bindings in
    /// `for (let/const ...)` loops. Closures created after this point capture
    /// independent copies of the listed slots, so each iteration has its own
    /// binding.
    FreshIterationScope(Vec<usize>),
    /// Starts a try frame after loop setup.
    EnterTry {
        catch: Option<usize>,
        finally: Option<usize>,
        catch_scope: Option<CatchScope>,
        cleanup_slots: Vec<usize>,
    },
    ExitTry,
    EndFinally,
    /// Clears any pending throw or return when an abrupt completion (break,
    /// continue, or return) exits a finally block. Without this the stale
    /// pending state would be picked up by the next `EndFinally`.
    DiscardPendingAbrupt,
    Return,
    Throw,
    /// Throws a `ReferenceError` with the given message at runtime, without
    /// evaluating any operands. Emitted for `delete super.x` / `delete
    /// super[expr]`: deleting a SuperReference is a runtime ReferenceError and
    /// the property-key expression is never evaluated.
    ThrowReferenceError(String),
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
        async_delegate: bool,
    },
    /// Dynamic `import(specifier)`. The specifier value is on top of the stack
    /// (and the options argument below it when `has_options` is set). Coerces
    /// the specifier to a string, builds a Promise capability, schedules a host
    /// load job, and leaves the capability's promise on the stack. A coercion
    /// failure rejects the promise rather than throwing (IfAbruptRejectPromise).
    ImportCall {
        has_options: bool,
    },
    /// `import.meta`. Only meaningful in module code; reported as a SyntaxError
    /// in a script (where no module host is installed).
    ImportMeta,
}

/// Compiled definition of a class constructor.
#[derive(Clone, Debug)]
pub(super) struct ClassConstructorDef {
    pub(super) name: Option<String>,
    pub(super) params: FunctionParams,
    pub(super) local_names: Vec<String>,
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
}

/// Whether a class member key is a literal name or a computed expression.
#[derive(Clone, Debug)]
pub(super) enum ClassMemberKeyDef {
    /// A statically known string key.
    Literal(String),
    /// A computed key evaluated by `NewClass` in class-element order.
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
/// may carry a computed key evaluated by `NewClass`.
#[derive(Clone, Debug)]
pub(super) enum ClassElementDef {
    Method(ClassMethodDef),
    Field(ClassFieldDef),
    /// A private field/method/accessor placeholder kept in source order so
    /// instance initialization can interleave private and public elements.
    Private(ClassPrivateElementDef),
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
    pub(super) lexical_captures: Vec<(String, usize)>,
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

#[derive(Clone, Debug)]
pub(super) enum ClassComputedKeyDef {
    Precomputed,
    Deferred {
        local_names: Vec<String>,
        lexical_captures: Vec<(String, usize)>,
        bytecode: Rc<Bytecode>,
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
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
    pub(super) source_text: Option<Rc<str>>,
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
    pub(super) lexical_captures: Vec<(String, usize)>,
    pub(super) bytecode: Rc<Bytecode>,
}

#[derive(Clone, Debug)]
pub(super) enum ArrayElementKind {
    Expr,
    Elision,
    Spread,
}

/// How to derive an anonymous function's name from a computed property key.
#[derive(Clone, Copy, Debug)]
pub(super) enum ComputedNameKind {
    /// Data property or method: the key name verbatim.
    Plain,
    /// Getter accessor: `get ` prefix.
    Getter,
    /// Setter accessor: `set ` prefix.
    Setter,
}

/// Per-property metadata for an object literal definition.
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
    pub(super) hoisted_function: bool,
    pub(super) parameter: bool,
    pub(super) catch_binding: bool,
    pub(super) mutable: bool,
    pub(super) from_env: bool,
    pub(super) sloppy_global_fallback: bool,
}

impl Local {
    /// Whether this slot is an outer binding received through
    /// `Function.upvalues`. Function parameters and body `var` bindings are
    /// also seeded from `CallEnv` today, so `from_env` alone is not sufficient
    /// to identify an indexed upvalue.
    pub(super) fn is_received_upvalue(&self) -> bool {
        self.from_env && !self.parameter && !self.hoisted
    }
}

/// Compiled bytecode for a script.
#[derive(Clone, Debug)]
pub struct Bytecode {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    local_slots: HashMap<String, usize>,
    global_names: Vec<String>,
    global_lexical_names: Vec<String>,
    sloppy_global_assignment_names: Vec<String>,
    eval_deletable_local_names: BTreeSet<String>,
    /// Whether this bytecode is global script code (top-level scripts and
    /// eval bodies). Global `var`/function bindings live in the realm, and
    /// `this` resolves to the realm global; function bodies resolve `this`
    /// from their own frame.
    pub(super) global_scope: bool,
    /// Whether this bytecode was compiled in strict mode after applying any
    /// source prologue. Direct eval needs this to choose the correct
    /// declaration instantiation environment.
    strict: bool,
    pub(super) code: Vec<Op>,
    pub(super) template_objects: RefCell<HashMap<usize, Value>>,
    /// Per-call metadata precomputed once at construction. Each of these used to
    /// be recomputed on every call by recursively walking `code` (and nested
    /// function/class op trees) and materializing a fresh `BTreeSet`/`Vec`,
    /// which dominated call cost (`tasks/T011-call-performance.md`). A
    /// `Bytecode` is immutable after compilation and lives behind `Rc`, and
    /// nested bytecodes are fully built before their parent, so memoizing here
    /// is a pure optimization with identical results.
    cached_closure_referenced_global_names: Vec<String>,
    cached_written_binding_names: Vec<String>,
    cached_closure_written_binding_names: Vec<String>,
    cached_writes_binding_set: HashSet<String>,
    cached_creates_closures: bool,
    cached_needs_arguments_object: bool,
    cached_contains_direct_eval: bool,
    cached_contains_with: bool,
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
        Self::with_scope_and_global_lexical_names(constants, locals, code, global_scope, Vec::new())
    }

    pub(super) fn with_scope_and_global_lexical_names(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
        global_lexical_names: Vec<String>,
    ) -> Self {
        Self::with_scope_global_lexical_names_and_strict(
            constants,
            locals,
            code,
            global_scope,
            global_lexical_names,
            false,
        )
    }

    pub(super) fn with_scope_global_lexical_names_and_strict(
        constants: Vec<Value>,
        locals: Vec<Local>,
        code: Vec<Op>,
        global_scope: bool,
        global_lexical_names: Vec<String>,
        strict: bool,
    ) -> Self {
        let mut bytecode = Self {
            constants,
            local_slots: collect_local_slots(&locals),
            locals,
            global_names: collect_global_names(&code),
            global_lexical_names,
            sloppy_global_assignment_names: collect_sloppy_global_assignment_names(&code),
            eval_deletable_local_names: BTreeSet::new(),
            global_scope,
            strict,
            code,
            template_objects: RefCell::new(HashMap::new()),
            cached_closure_referenced_global_names: Vec::new(),
            cached_written_binding_names: Vec::new(),
            cached_closure_written_binding_names: Vec::new(),
            cached_writes_binding_set: HashSet::new(),
            cached_creates_closures: false,
            cached_needs_arguments_object: false,
            cached_contains_direct_eval: false,
            cached_contains_with: false,
        };
        // Order matters: closure/arguments metadata reads the simpler caches
        // (written-binding names, creates-closures) computed just above. Nested
        // bytecodes are already fully built, so their accessors return cached
        // values here too.
        bytecode.cached_closure_referenced_global_names =
            bytecode.compute_closure_referenced_global_names();
        bytecode.cached_written_binding_names = bytecode.compute_written_binding_names();
        bytecode.cached_closure_written_binding_names =
            bytecode.compute_closure_written_binding_names();
        bytecode.cached_writes_binding_set = bytecode.compute_writes_binding_set();
        bytecode.cached_creates_closures = bytecode.compute_creates_closures();
        bytecode.cached_needs_arguments_object = bytecode.compute_needs_arguments_object();
        bytecode.cached_contains_direct_eval = bytecode.code.iter().any(|op| {
            matches!(
                op,
                Op::CallDirectEval { .. } | Op::CallDirectEvalSpread { .. }
            )
        });
        bytecode.cached_contains_with = bytecode
            .code
            .iter()
            .any(|op| matches!(op, Op::EnterWith | Op::ExitWith));
        bytecode
    }

    pub(crate) fn is_strict(&self) -> bool {
        self.strict
    }

    pub(crate) fn global_names(&self) -> &[String] {
        &self.global_names
    }

    pub(crate) fn referenced_global_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        for name in &self.global_names {
            names.insert(name.clone());
        }
        for local in &self.locals {
            if !local.sloppy_global_fallback && !local.from_env {
                names.remove(&local.name);
            }
        }
        names.into_iter().collect()
    }

    pub(crate) fn closure_referenced_global_names(&self) -> Vec<String> {
        self.cached_closure_referenced_global_names.clone()
    }

    fn compute_closure_referenced_global_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        for name in self.referenced_global_names() {
            names.insert(name);
        }
        super::ir_names::collect_nested_global_names_from_ops(&self.code, &mut names);
        names.into_iter().collect()
    }

    pub(crate) fn written_binding_names(&self) -> Vec<String> {
        self.cached_written_binding_names.clone()
    }

    fn compute_written_binding_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        collect_written_binding_names_from_ops(self, &self.code, &mut names);
        for local in &self.locals {
            if !local.sloppy_global_fallback && !local.from_env {
                names.remove(&local.name);
            }
        }
        names.into_iter().collect()
    }

    pub(crate) fn closure_written_binding_names(&self) -> Vec<String> {
        self.cached_closure_written_binding_names.clone()
    }

    fn compute_closure_written_binding_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        for name in self.written_binding_names() {
            names.insert(name);
        }
        super::ir_names::collect_nested_written_binding_names_from_ops(&self.code, &mut names);
        names.into_iter().collect()
    }

    pub(crate) fn global_lexical_names(&self) -> &[String] {
        &self.global_lexical_names
    }

    pub(crate) fn sloppy_global_assignment_names(&self) -> &[String] {
        &self.sloppy_global_assignment_names
    }

    pub(crate) fn local_names(&self) -> impl Iterator<Item = &str> {
        self.locals.iter().map(|local| local.name.as_str())
    }

    pub(crate) fn eval_lexical_local_names(&self) -> impl Iterator<Item = &str> {
        self.locals
            .iter()
            .filter(|local| {
                !local.hoisted
                    && !local.sloppy_global_fallback
                    && !local.name.starts_with('\0')
                    && !self
                        .locals
                        .iter()
                        .any(|candidate| candidate.hoisted && candidate.name == local.name)
            })
            .map(|local| local.name.as_str())
    }

    pub(crate) fn hoisted_local_names(&self) -> impl Iterator<Item = &str> {
        self.locals
            .iter()
            .filter(|local| local.hoisted)
            .map(|local| local.name.as_str())
    }

    pub(crate) const fn is_global_scope(&self) -> bool {
        self.global_scope
    }

    pub(crate) fn hoisted_function_names(&self) -> impl Iterator<Item = &str> {
        self.locals
            .iter()
            .filter(|local| local.hoisted_function)
            .map(|local| local.name.as_str())
    }

    pub(crate) fn local_slot(&self, name: &str) -> Option<usize> {
        self.local_slots.get(name).copied()
    }

    pub(crate) fn local_name_at(&self, slot: usize) -> Option<&str> {
        self.locals.get(slot).map(|local| local.name.as_str())
    }

    pub(crate) fn local_is_mutable(&self, slot: usize) -> bool {
        self.locals.get(slot).is_some_and(|local| local.mutable)
    }

    pub(crate) fn mark_eval_deletable_locals<I>(&mut self, names: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.eval_deletable_local_names.extend(names);
    }

    pub(crate) fn local_is_eval_deletable(&self, slot: usize) -> bool {
        self.locals
            .get(slot)
            .is_some_and(|local| self.eval_deletable_local_names.contains(&local.name))
    }

    pub(crate) fn local_is_sloppy_global_fallback(&self, slot: usize) -> bool {
        self.locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback)
    }

    pub(crate) fn local_is_body_hoist_only(&self, slot: usize) -> bool {
        self.locals
            .get(slot)
            .is_some_and(|local| local.hoisted && !local.parameter)
    }

    pub(crate) fn local_is_parameter(&self, slot: usize) -> bool {
        self.locals.get(slot).is_some_and(|local| local.parameter)
    }

    pub(crate) fn local_is_from_env(&self, slot: usize) -> bool {
        self.locals.get(slot).is_some_and(|local| local.from_env)
    }

    pub(crate) fn received_upvalue_names(&self) -> impl Iterator<Item = &str> {
        self.locals
            .iter()
            .filter(|local| local.is_received_upvalue())
            .map(|local| local.name.as_str())
    }

    /// Whether the body can create a nested closure, class, generator, or async
    /// function. Used when deciding whether an `arguments` object is observable.
    pub(crate) fn creates_closures(&self) -> bool {
        self.cached_creates_closures
    }

    fn compute_creates_closures(&self) -> bool {
        self.code
            .iter()
            .any(|op| matches!(op, Op::NewFunction { .. } | Op::NewClass { .. }))
    }

    /// Whether this body contains a top-level `await` (`Op::Await`). Nested
    /// function/closure bodies compile to their own [`Bytecode`] constants, so a
    /// scan of this code detects only awaits at this body's own level — exactly
    /// the top-level-await marker the module driver needs.
    pub(crate) fn contains_top_level_await(&self) -> bool {
        self.code.iter().any(|op| matches!(op, Op::Await))
    }

    pub(crate) fn uses_lexical_this(&self) -> bool {
        self.code.iter().any(|op| {
            matches!(
                op,
                Op::LoadGlobal(name) if name == "this"
            ) || matches!(
                op,
                Op::SuperCall(_)
                    | Op::SuperCallSpread
                    | Op::SuperGet { .. }
                    | Op::SuperReference
                    | Op::SuperGetComputed
                    | Op::SuperSet { .. }
                    | Op::SuperSetComputed { .. }
            )
        })
    }

    pub(crate) fn contains_direct_eval(&self) -> bool {
        self.cached_contains_direct_eval
    }

    pub(crate) fn contains_with(&self) -> bool {
        self.cached_contains_with
    }

    pub(crate) fn needs_arguments_object(&self) -> bool {
        self.cached_needs_arguments_object
    }

    fn compute_needs_arguments_object(&self) -> bool {
        if self.global_names.iter().any(|name| name == "arguments") {
            return true;
        }
        if self.code.iter().any(|op| {
            matches!(
                op,
                Op::CallDirectEval { .. } | Op::CallDirectEvalSpread { .. }
            )
        }) {
            return true;
        }
        if self.code.iter().any(|op| {
            matches!(op, Op::DeleteIdent(name) if name == "arguments")
                || matches!(op, Op::DeleteIdentWith { name, .. } if name == "arguments")
        }) {
            return true;
        }
        let Some(arguments_slot) = self.local_slot("arguments") else {
            return false;
        };
        self.creates_closures()
            || self.code.iter().any(|op| match op {
                Op::LoadLocal(slot) | Op::LoadLocalOrUndefined(slot) => *slot == arguments_slot,
                Op::LoadIdentWith {
                    slot: Some(slot), ..
                }
                | Op::LoadResolvedIdentWith {
                    slot: Some(slot), ..
                }
                | Op::TypeofIdentWith {
                    slot: Some(slot), ..
                } => *slot == arguments_slot,
                _ => false,
            })
    }

    pub(crate) fn writes_binding(&self, binding_name: &str) -> bool {
        self.cached_writes_binding_set.contains(binding_name)
    }

    /// Builds the set of every binding name written anywhere in this body,
    /// including nested function and class bodies. `writes_binding(name)` is
    /// then a `HashSet` membership test. The direct (this-level) store names are
    /// exactly what `collect_written_binding_names_from_ops` gathers; nested
    /// contributions come from already-cached child sets.
    fn compute_writes_binding_set(&self) -> HashSet<String> {
        let mut direct = BTreeSet::new();
        collect_written_binding_names_from_ops(self, &self.code, &mut direct);
        let mut set: HashSet<String> = direct.into_iter().collect();
        for op in &self.code {
            match op {
                Op::NewFunction { bytecode, .. } => {
                    set.extend(bytecode.cached_writes_binding_set.iter().cloned());
                }
                Op::NewClass {
                    constructor,
                    elements,
                    ..
                } => {
                    set.extend(
                        constructor
                            .bytecode
                            .cached_writes_binding_set
                            .iter()
                            .cloned(),
                    );
                    for element in elements {
                        collect_class_element_writes_binding(element, &mut set);
                    }
                }
                _ => {}
            }
        }
        set
    }
}

fn collect_local_slots(locals: &[Local]) -> HashMap<String, usize> {
    let mut slots = HashMap::new();
    for (slot, local) in locals.iter().enumerate() {
        slots.entry(local.name.clone()).or_insert(slot);
    }
    slots
}

fn collect_class_element_writes_binding(element: &ClassElementDef, set: &mut HashSet<String>) {
    match element {
        ClassElementDef::Method(def) => {
            set.extend(def.bytecode.cached_writes_binding_set.iter().cloned());
        }
        ClassElementDef::Field(def) => {
            if let Some(initializer) = def.initializer.as_ref() {
                set.extend(
                    initializer
                        .bytecode
                        .cached_writes_binding_set
                        .iter()
                        .cloned(),
                );
            }
        }
        ClassElementDef::Private(def) => collect_private_class_element_writes_binding(def, set),
        ClassElementDef::StaticBlock(def) => {
            set.extend(def.bytecode.cached_writes_binding_set.iter().cloned());
        }
    }
}

fn collect_private_class_element_writes_binding(
    element: &ClassPrivateElementDef,
    set: &mut HashSet<String>,
) {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(initializer) = initializer.as_ref() {
                set.extend(
                    initializer
                        .bytecode
                        .cached_writes_binding_set
                        .iter()
                        .cloned(),
                );
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => {
            set.extend(def.bytecode.cached_writes_binding_set.iter().cloned());
        }
    }
}

fn collect_written_binding_names_from_ops(
    bytecode: &Bytecode,
    code: &[Op],
    names: &mut BTreeSet<String>,
) {
    for op in code {
        match op {
            Op::StoreGlobalStrict(name)
            | Op::StoreGlobalSloppy(name)
            | Op::AppendStringLiteralGlobal { name, .. }
            | Op::StoreLocalOrGlobalSloppy { name, .. }
            | Op::StoreIdentWith {
                name, slot: None, ..
            }
            | Op::StoreResolvedIdentWith {
                name, slot: None, ..
            } => {
                names.insert(name.clone());
            }
            Op::StoreLocal(slot)
            | Op::AssignLocal(slot)
            | Op::AppendStringLiteralLocal { slot, .. }
            | Op::StoreIdentWith {
                slot: Some(slot), ..
            }
            | Op::StoreResolvedIdentWith {
                slot: Some(slot), ..
            } => {
                if let Some(local) = bytecode.locals.get(*slot) {
                    names.insert(local.name.clone());
                }
            }
            _ => {}
        }
    }
}

fn collect_global_names(code: &[Op]) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_global_names_from_ops(code, &mut names);
    names.into_iter().collect()
}

fn collect_global_names_from_ops(code: &[Op], names: &mut BTreeSet<String>) {
    for op in code {
        match op {
            Op::LoadGlobal(name)
            | Op::StoreGlobalStrict(name)
            | Op::StoreGlobalSloppy(name)
            | Op::AppendStringLiteralGlobal { name, .. }
            | Op::TypeofGlobal(name) => {
                names.insert(name.clone());
            }
            Op::StoreLocalOrGlobalSloppy { name, .. } => {
                names.insert(name.clone());
            }
            Op::LoadIdentWith {
                name, slot: None, ..
            }
            | Op::ResolveIdentWith {
                name, slot: None, ..
            }
            | Op::LoadResolvedIdentWith {
                name, slot: None, ..
            }
            | Op::StoreIdentWith {
                name, slot: None, ..
            }
            | Op::StoreResolvedIdentWith {
                name, slot: None, ..
            }
            | Op::TypeofIdentWith { name, slot: None } => {
                names.insert(name.clone());
            }
            Op::NewFunction { bytecode, .. } => {
                names.extend(bytecode.global_names().iter().cloned());
            }
            Op::NewClass {
                constructor,
                elements,
                private_elements,
                computed_keys,
                ..
            } => {
                names.extend(constructor.bytecode.global_names().iter().cloned());
                for key in computed_keys {
                    if let ClassComputedKeyDef::Deferred { bytecode, .. } = key {
                        names.extend(bytecode.global_names().iter().cloned());
                    }
                }
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
                        ClassElementDef::Private(element) => match element {
                            ClassPrivateElementDef::Field { initializer, .. } => {
                                if let Some(initializer) = initializer {
                                    names.extend(
                                        initializer.bytecode.global_names().iter().cloned(),
                                    );
                                }
                            }
                            ClassPrivateElementDef::Method { def, .. }
                            | ClassPrivateElementDef::Getter { def, .. }
                            | ClassPrivateElementDef::Setter { def, .. } => {
                                names.extend(def.bytecode.global_names().iter().cloned());
                            }
                        },
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

/// A property key excluded from an object rest pattern.
#[derive(Clone, Debug)]
pub(super) enum ObjectRestExclusion {
    /// A statically known string key.
    Literal(String),
    /// A local slot holding an already evaluated ToPropertyKey result.
    Local(usize),
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
