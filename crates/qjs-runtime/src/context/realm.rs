//! Realm ownership: canonical global cells, stable intrinsics, construction
//! schemas and immutable eval/regexp plans. No activation-local names live here.
mod construction;
use super::eval_cache::DirectEvalCache;
use crate::{
    Bytecode, Function, JsString, ObjectRef, Value,
    function::{BindingCell, DynamicBindings},
    property::atom::AtomDomain,
};
use qjs_parser::EvalParseContext;
use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};
const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

/// Shared realm state: intrinsics, true globals, and captured-global cells.
///
/// `bindings` is the single source of truth for every realm-level name: it is
/// a name-to-[`BindingCell`] map from the start, not a plain `Value` map with a
/// separate lazily-populated cell registry layered on top. A name that never
/// gets captured or hoisted-global-cached still costs exactly one hash lookup
/// to read or write (through its cell); a name that does get a cell no longer
/// needs a *second* hash lookup into a separate registry to keep that cell in
/// sync, which the old two-map design required on every store.
pub(crate) struct RealmState {
    bindings: DynamicBindings,
    group: super::RuntimeGroup,
    root_creation: RefCell<std::rc::Weak<crate::context::CreationContext>>,
    /// Stable realm intrinsics are separate from the mutable global bindings.
    /// Rebinding `Object` or `Array` must not change the default prototype used
    /// by literals, classes, or abstract operations that name an intrinsic.
    object_prototype: OnceCell<ObjectRef>,
    array_prototype: OnceCell<ObjectRef>,
    /// Intrinsic arguments schemas contain only descriptor metadata and this
    /// realm's well-known symbols; their actual values/accessors stay per object.
    arguments_layouts: RefCell<Vec<Rc<crate::value::layout::Layout>>>,
    array_named_seed: OnceCell<Rc<crate::value::layout::NamedStorage>>,
    function_prototype: OnceCell<crate::Prototype>,
    function_constructor: OnceCell<Function>,
    /// Stable `%String.prototype%` identity used by primitive String reads.
    /// The mutable global `String` binding is not the intrinsic authority.
    string_prototype: OnceCell<ObjectRef>,
    string_wrapper_layout: OnceCell<crate::value::layout::StringWrapperLayout>,
    /// Stable `%Number.prototype%` identity used by primitive Number reads.
    number_prototype: OnceCell<ObjectRef>,
    other_primitive_prototypes: [OnceCell<ObjectRef>; 3],
    errors: [OnceCell<super::intrinsics::ErrorIntrinsic>; 9],
    /// The realm's well-known symbols, in `symbol::WELL_KNOWN_SYMBOL_NAMES`
    /// order.
    well_known_symbols: OnceCell<Vec<ObjectRef>>,
    /// Property keys every boxed String installs, shared instead of allocated.
    string_object_keys: crate::string::StringObjectKeys,
    /// Canonical empty copy-on-write name set for ordinary frames. Sharing it
    /// avoids allocating catch/eval metadata that most calls never mutate.
    global_this: Option<Value>,
    dynamic_function_realm_global: RefCell<Option<ObjectRef>>,
    /// Parsed/compiled blueprints for small, declaration-free direct eval
    /// sources. Entries share only immutable bytecode; each invocation still
    /// receives an independent VM frame and runtime values.
    direct_eval_cache: RefCell<DirectEvalCache>,
    /// Compiled regular-expression programs and validated patterns, shared
    /// by every RegExp object with the same source and flags. Bounded; holds
    /// no objects or realm references.
    regexp_programs: RefCell<crate::regexp::ProgramCache>,
    /// The property-name atom domain. A realm derived from another (a
    /// dynamic-function scope) shares its parent's domain because the two
    /// exchange objects; an independent root owns a fresh one.
    property_owner: crate::value::layout::PropertyOwner,
}

impl RealmState {
    /// Canonical realm cell table; callers never copy its values into frames.
    pub(crate) fn group(&self) -> &super::RuntimeGroup {
        &self.group
    }

    pub(crate) fn binding_cells(&self) -> &DynamicBindings {
        &self.bindings
    }

    pub(crate) fn cached_root_creation(&self) -> Option<Rc<crate::context::CreationContext>> {
        self.root_creation.borrow().upgrade()
    }
    pub(crate) fn cache_root_creation(
        &self,
        creation: std::rc::Weak<crate::context::CreationContext>,
    ) {
        *self.root_creation.borrow_mut() = creation;
    }

    pub(crate) fn initialize_other_primitive_prototype(
        &self,
        kind: crate::context::PrimitiveIntrinsic,
        prototype: ObjectRef,
    ) {
        use crate::context::PrimitiveIntrinsic as P;
        let index = match kind {
            P::Boolean => 0,
            P::BigInt => 1,
            P::Symbol => 2,
            _ => unreachable!("existing dedicated intrinsic"),
        };
        assert!(
            self.other_primitive_prototypes[index]
                .set(prototype)
                .is_ok(),
            "primitive intrinsic initialized twice"
        );
    }
    pub(crate) fn primitive_prototype(
        &self,
        kind: crate::context::PrimitiveIntrinsic,
    ) -> Option<ObjectRef> {
        use crate::context::PrimitiveIntrinsic as P;
        match kind {
            P::Number => self.number_prototype(),
            P::String => self.string_prototype(),
            P::Boolean => self.other_primitive_prototypes[0].get().cloned(),
            P::BigInt => self.other_primitive_prototypes[1].get().cloned(),
            P::Symbol => self.other_primitive_prototypes[2].get().cloned(),
        }
    }
    pub(crate) fn next_variable_identity(&self) -> u64 {
        self.atoms().next_variable_identity()
    }

    fn new(bindings: HashMap<String, Value>, atoms: AtomDomain) -> Self {
        let global_this = bindings.get(crate::GLOBAL_THIS_BINDING).cloned();
        let dynamic_function_realm_global =
            bindings
                .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
                .and_then(|value| match value {
                    Value::Object(global) => Some(global.clone()),
                    _ => None,
                });
        Self {
            bindings: DynamicBindings::from_values(bindings),
            group: super::RuntimeGroup::for_atoms(&atoms),
            root_creation: RefCell::default(),
            object_prototype: OnceCell::new(),
            array_prototype: OnceCell::new(),
            arguments_layouts: RefCell::default(),
            array_named_seed: OnceCell::new(),
            function_prototype: OnceCell::new(),
            function_constructor: OnceCell::new(),
            string_prototype: OnceCell::new(),
            string_wrapper_layout: OnceCell::new(),
            number_prototype: OnceCell::new(),
            other_primitive_prototypes: std::array::from_fn(|_| OnceCell::new()),
            errors: std::array::from_fn(|_| OnceCell::new()),
            well_known_symbols: OnceCell::new(),
            string_object_keys: crate::string::StringObjectKeys::default(),
            global_this,
            dynamic_function_realm_global: RefCell::new(dynamic_function_realm_global),
            direct_eval_cache: RefCell::new(DirectEvalCache::default()),
            regexp_programs: RefCell::default(),
            property_owner: crate::value::layout::PropertyOwner::new(atoms.clone()),
        }
    }

    pub(crate) fn initialize_error_intrinsic(
        &self,
        kind: super::ErrorKind,
        constructor: Function,
        prototype: ObjectRef,
    ) {
        assert!(
            self.errors[kind.index()]
                .set(super::intrinsics::ErrorIntrinsic {
                    constructor,
                    prototype
                })
                .is_ok(),
            "error intrinsic initialized twice"
        );
    }
    pub(crate) fn error_prototype(&self, kind: super::ErrorKind) -> Option<ObjectRef> {
        self.errors[kind.index()]
            .get()
            .map(|intrinsic| intrinsic.prototype.clone())
    }
    pub(crate) fn error_constructor(&self, kind: super::ErrorKind) -> Option<Function> {
        self.errors[kind.index()]
            .get()
            .map(|intrinsic| intrinsic.constructor.clone())
    }

    pub(crate) fn string_object_keys(&self) -> &crate::string::StringObjectKeys {
        &self.string_object_keys
    }

    pub(crate) fn regexp_programs(&self) -> &RefCell<crate::regexp::ProgramCache> {
        &self.regexp_programs
    }

    /// The property-name atom domain this realm's code links against.
    #[inline]
    pub(crate) fn atoms(&self) -> &AtomDomain {
        self.property_owner.domain()
    }

    pub(crate) fn initialize_object_prototype(&self, prototype: ObjectRef) {
        assert!(
            self.object_prototype.set(prototype).is_ok(),
            "realm Object.prototype intrinsic initialized twice"
        );
    }

    pub(crate) fn initialize_array_prototype(&self, prototype: ObjectRef) {
        assert!(
            self.array_prototype.set(prototype).is_ok(),
            "realm Array.prototype intrinsic initialized twice"
        );
    }

    pub(crate) fn initialize_function_prototype(&self, prototype: crate::Prototype) {
        assert!(
            self.function_prototype.set(prototype).is_ok(),
            "Function.prototype initialized twice"
        );
    }
    pub(crate) fn function_prototype(&self) -> Option<crate::Prototype> {
        self.function_prototype.get().cloned()
    }
    pub(crate) fn initialize_function_constructor(&self, constructor: Function) {
        assert!(
            self.function_constructor.set(constructor).is_ok(),
            "Function constructor initialized twice"
        );
    }
    pub(crate) fn function_constructor(&self) -> Option<Function> {
        self.function_constructor.get().cloned()
    }
    pub(crate) fn initialize_number_prototype(&self, prototype: ObjectRef) {
        assert!(
            self.number_prototype.set(prototype).is_ok(),
            "realm Number.prototype intrinsic initialized twice"
        );
    }

    pub(crate) fn initialize_well_known_symbols(&self, symbols: Vec<ObjectRef>) {
        assert!(
            self.well_known_symbols.set(symbols).is_ok(),
            "realm well-known symbols initialized twice"
        );
    }

    pub(crate) fn well_known_symbol(&self, index: usize) -> Option<ObjectRef> {
        self.well_known_symbols.get()?.get(index).cloned()
    }

    pub(crate) fn number_prototype(&self) -> Option<ObjectRef> {
        self.number_prototype.get().cloned()
    }

    pub(crate) fn initialize_string_prototype(&self, prototype: ObjectRef) {
        assert!(
            self.string_prototype.set(prototype).is_ok(),
            "realm String.prototype intrinsic initialized twice"
        );
    }

    pub(crate) fn object_prototype(&self) -> Option<ObjectRef> {
        self.object_prototype.get().cloned()
    }

    pub(crate) fn array_prototype(&self) -> Option<ObjectRef> {
        self.array_prototype.get().cloned()
    }

    pub(crate) fn string_prototype(&self) -> Option<ObjectRef> {
        self.string_prototype.get().cloned()
    }

    pub(crate) fn get_value(&self, name: &str) -> Option<Value> {
        self.bindings.get(name)
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub(crate) fn cell(&self, name: &str) -> Option<BindingCell> {
        self.bindings.cell(name)
    }

    pub(crate) fn insert_value(&self, name: String, value: Value) -> Option<Value> {
        self.bindings.insert(name, value)
    }

    pub(crate) fn remove_value(&self, name: &str) -> Option<Value> {
        // Removed object routes may still be retained by closures or suspended
        // frames. Invalidate at this common boundary, including descriptor and
        // property deletion paths that do not call Context::remove_realm.
        let previous = self.bindings.get(name);
        if let Some(cell) = self.bindings.cell(name) {
            cell.invalidate_object_route();
        }
        self.bindings.remove(name);
        previous
    }

    pub(crate) fn entry_or_insert_value(&self, name: String, value: Value) {
        if !self.bindings.contains_key(&name) {
            self.bindings.insert(name, value);
        }
    }

    pub(crate) fn global_this(&self) -> Option<Value> {
        self.global_this.clone()
    }

    /// Whether `object` is this realm's global object, without cloning it.
    pub(crate) fn is_global_object(&self, object: &ObjectRef) -> bool {
        matches!(&self.global_this, Some(Value::Object(global)) if global.ptr_eq(object))
    }

    /// Returns the internal global object used by dynamically constructed
    /// functions without hashing its private binding name for every closure.
    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        self.dynamic_function_realm_global.borrow().clone()
    }

    pub(crate) fn has_dynamic_function_realm_global(&self) -> bool {
        self.dynamic_function_realm_global.borrow().is_some()
    }

    pub(crate) fn cached_direct_eval_bytecode(
        &self,
        source: &JsString,
        context: &EvalParseContext,
    ) -> Option<Rc<Bytecode>> {
        self.direct_eval_cache.borrow().lookup(source, context)
    }

    pub(crate) fn cache_direct_eval_bytecode(
        &self,
        source: JsString,
        context: EvalParseContext,
        bytecode: Rc<Bytecode>,
    ) {
        self.direct_eval_cache
            .borrow_mut()
            .insert(source, context, bytecode);
    }

    #[cfg(test)]
    pub(crate) fn direct_eval_cache_len(&self) -> usize {
        self.direct_eval_cache.borrow().len()
    }

    pub(crate) fn sync_dynamic_function_realm_binding(&self, name: &str, value: Option<&Value>) {
        if name != DYNAMIC_FUNCTION_REALM_GLOBAL {
            return;
        }
        *self.dynamic_function_realm_global.borrow_mut() = value.and_then(|value| match value {
            Value::Object(global) => Some(global.clone()),
            _ => None,
        });
    }

    /// Rebuilds cached internal metadata after a bulk initialization path has
    /// edited the binding map directly.
    pub(crate) fn refresh_dynamic_function_realm_global(&self) {
        let global =
            self.bindings
                .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
                .and_then(|value| match value {
                    Value::Object(global) => Some(global.clone()),
                    _ => None,
                });
        *self.dynamic_function_realm_global.borrow_mut() = global;
    }

    pub(crate) fn is_binding_cell(&self, name: &str, cell: &BindingCell) -> bool {
        self.bindings.cell(name).is_some_and(|candidate| {
            candidate.metadata().kind != crate::binding::BindingKind::Lexical
                && candidate.ptr_eq(cell)
        })
    }
}

impl fmt::Debug for RealmState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RealmState")
            .field("bindings", &self.bindings.snapshot())
            .finish_non_exhaustive()
    }
}

pub(crate) type Realm = Rc<RealmState>;

/// A realm for an independent root, with its own atom domain.
#[cfg(test)]
pub(crate) fn new_realm(bindings: HashMap<String, Value>) -> Realm {
    new_realm_sharing_atoms(bindings, AtomDomain::new())
}

/// A realm that exchanges objects with code using `atoms`, and so shares it.
pub(crate) fn new_realm_sharing_atoms(
    bindings: HashMap<String, Value>,
    atoms: AtomDomain,
) -> Realm {
    let realm = Rc::new(RealmState::new(bindings, atoms));
    realm.group.register(&realm);
    realm
}
