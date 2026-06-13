//! Private class names (`#x`), their per-class-evaluation identities, and the
//! per-object storage that backs private fields, methods, and accessors.
//!
//! Private names are not ordinary properties: they live in a side table keyed
//! by a unique [`PrivateName`] identity created fresh on every evaluation of a
//! class definition. A [`PrivateEnvironment`] maps the source-level name text
//! (without the `#`) to the identity and, for methods/accessors, the bound
//! function values shared by every instance of the class. Member access is
//! resolved lexically: a method, field initializer, or constructor reaches its
//! class's [`PrivateEnvironment`] through its `[[HomeObject]]`.

use std::{cell::RefCell, rc::Rc};

use crate::Value;

/// A unique private-name identity. Two [`PrivateName`]s are equal only when
/// they share the same allocation, so each class evaluation produces distinct
/// identities even for the same source text.
#[derive(Clone)]
pub(crate) struct PrivateName {
    inner: Rc<PrivateNameData>,
}

struct PrivateNameData {
    /// Source name without the leading `#`, used only for diagnostics.
    name: String,
}

impl PrivateName {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            inner: Rc::new(PrivateNameData {
                name: name.to_owned(),
            }),
        }
    }

    pub(crate) fn description(&self) -> &str {
        &self.inner.name
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

/// What a private name maps to within a class: an instance field placeholder, a
/// shared method, or a shared accessor pair. Fields are installed per instance
/// during field initialization; methods and accessors are shared across all
/// instances and only branded onto each instance.
#[derive(Clone)]
pub(crate) enum PrivateKind {
    /// An instance or static field. The value is stored per object.
    Field,
    /// A shared method function value, boxed so the enum stays small.
    Method(Box<Value>),
    /// A shared accessor pair, boxed so the field/method variants stay small.
    Accessor(Box<PrivateAccessor>),
}

/// The getter and setter function values for a private accessor.
#[derive(Clone)]
pub(crate) struct PrivateAccessor {
    pub(crate) get: Option<Value>,
    pub(crate) set: Option<Value>,
}

/// A binding declared by a class for one private name: its identity and kind.
#[derive(Clone)]
pub(crate) struct PrivateBinding {
    pub(crate) id: PrivateName,
    pub(crate) kind: PrivateKind,
}

/// The set of private names declared by one class evaluation, keyed by source
/// text. Shared (via `Rc`) by the class prototype, the constructor, and every
/// method/field/constructor function so a `#x` reference resolves to the same
/// identity. Accessor halves declared separately merge into one binding.
#[derive(Clone)]
pub(crate) struct PrivateEnvironment {
    bindings: Rc<RefCell<Vec<(String, PrivateBinding)>>>,
    /// The lexically enclosing class's private environment, so a member body
    /// inside a nested class can resolve a private name declared by an outer
    /// class. Resolution walks this chain outward.
    outer: Option<Rc<PrivateEnvironment>>,
}

impl PrivateEnvironment {
    /// Creates a private environment nested inside `outer`, so unresolved names
    /// fall through to the enclosing class.
    pub(crate) fn with_outer(outer: Option<PrivateEnvironment>) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(Vec::new())),
            outer: outer.map(Rc::new),
        }
    }

    /// Declares a fresh private-name identity for `name`, returning it. A field,
    /// method, or the first half of an accessor pair creates a new binding.
    pub(crate) fn declare_field(&self, name: &str) -> PrivateName {
        if let Some(id) = self.local_id(name) {
            return id;
        }
        let id = PrivateName::new(name);
        self.bindings.borrow_mut().push((
            name.to_owned(),
            PrivateBinding {
                id: id.clone(),
                kind: PrivateKind::Field,
            },
        ));
        id
    }

    /// Predeclares a private name before class element evaluation has built the
    /// final field/method/accessor metadata.
    pub(crate) fn declare_placeholder(&self, name: &str) -> PrivateName {
        self.declare_field(name)
    }

    /// Declares a shared method binding, returning its identity.
    pub(crate) fn declare_method(&self, name: &str, function: Value) -> PrivateName {
        if let Some(id) =
            self.replace_local_kind(name, PrivateKind::Method(Box::new(function.clone())))
        {
            return id;
        }
        let id = PrivateName::new(name);
        self.bindings.borrow_mut().push((
            name.to_owned(),
            PrivateBinding {
                id: id.clone(),
                kind: PrivateKind::Method(Box::new(function)),
            },
        ));
        id
    }

    /// Declares or extends a shared accessor binding for `name`. A getter and a
    /// setter for the same name (already verified to be a legal pair by the
    /// parser) merge into a single binding sharing one identity.
    pub(crate) fn declare_accessor(
        &self,
        name: &str,
        get: Option<Value>,
        set: Option<Value>,
    ) -> PrivateName {
        let mut bindings = self.bindings.borrow_mut();
        if let Some((_, binding)) = bindings.iter_mut().find(|(existing, binding)| {
            existing == name
                && matches!(binding.kind, PrivateKind::Accessor(_) | PrivateKind::Field)
        }) {
            match &mut binding.kind {
                PrivateKind::Accessor(accessor) => {
                    if get.is_some() {
                        accessor.get = get;
                    }
                    if set.is_some() {
                        accessor.set = set;
                    }
                }
                PrivateKind::Field => {
                    binding.kind = PrivateKind::Accessor(Box::new(PrivateAccessor { get, set }));
                }
                PrivateKind::Method(_) => unreachable!("parser rejects private duplicate names"),
            }
            return binding.id.clone();
        }
        let id = PrivateName::new(name);
        bindings.push((
            name.to_owned(),
            PrivateBinding {
                id: id.clone(),
                kind: PrivateKind::Accessor(Box::new(PrivateAccessor { get, set })),
            },
        ));
        id
    }

    fn local_id(&self, name: &str) -> Option<PrivateName> {
        self.bindings
            .borrow()
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, binding)| binding.id.clone())
    }

    fn replace_local_kind(&self, name: &str, kind: PrivateKind) -> Option<PrivateName> {
        let mut bindings = self.bindings.borrow_mut();
        let (_, binding) = bindings.iter_mut().find(|(existing, _)| existing == name)?;
        binding.kind = kind;
        Some(binding.id.clone())
    }

    /// Resolves a private-name reference by source text to its binding, walking
    /// outward through enclosing class environments.
    pub(crate) fn resolve(&self, name: &str) -> Option<PrivateBinding> {
        if let Some(binding) = self
            .bindings
            .borrow()
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, binding)| binding.clone())
        {
            return Some(binding);
        }
        self.outer.as_ref().and_then(|outer| outer.resolve(name))
    }

    pub(crate) fn visible_names(&self) -> Vec<String> {
        let mut names = self
            .outer
            .as_ref()
            .map_or_else(Vec::new, |outer| outer.visible_names());
        for (name, _) in self.bindings.borrow().iter() {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.clone());
            }
        }
        names
    }
}

/// The private-name state carried by a class constructor or prototype: the
/// per-object brand/field storage, the private environment its members resolve
/// `#x` references through, and (for a constructor) the instance private
/// elements applied at construction. Combined behind one allocation so adding
/// private-name support keeps the inline `Function`/`ObjectRef` size unchanged.
#[derive(Default)]
pub(crate) struct PrivateState {
    pub(crate) storage: Option<PrivateStorage>,
    pub(crate) environment: Option<PrivateEnvironment>,
    pub(crate) instance_elements: Vec<crate::function::InstancePrivateElement>,
}

/// Per-object private storage: the names branded onto the object, each with its
/// stored field value (methods/accessors are branded with no per-object value).
/// Brand presence is what `#x in obj` and access checks test.
#[derive(Clone)]
pub(crate) struct PrivateStorage {
    slots: Rc<RefCell<Vec<PrivateSlot>>>,
}

struct PrivateSlot {
    id: PrivateName,
    /// The field value for a field slot; `None` for a method/accessor brand.
    value: Option<Value>,
}

impl PrivateStorage {
    pub(crate) fn new() -> Self {
        Self {
            slots: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Reports whether the object carries a brand or field for `id`.
    pub(crate) fn has(&self, id: &PrivateName) -> bool {
        self.slots.borrow().iter().any(|slot| slot.id.same(id))
    }

    /// Brands the object with a method/accessor private name (no stored value).
    pub(crate) fn add_brand(&self, id: PrivateName) {
        if self.has(&id) {
            return;
        }
        self.slots
            .borrow_mut()
            .push(PrivateSlot { id, value: None });
    }

    /// Installs an instance/static private field value. Returns `false` when a
    /// field of that identity is already present (a re-initialization, which
    /// the field installation path treats as an error).
    pub(crate) fn add_field(&self, id: PrivateName, value: Value) -> bool {
        if self.has(&id) {
            return false;
        }
        self.slots.borrow_mut().push(PrivateSlot {
            id,
            value: Some(value),
        });
        true
    }

    /// Reads a stored field value. Returns `None` when the slot is absent or is
    /// a method/accessor brand (which has no stored value).
    pub(crate) fn get_field(&self, id: &PrivateName) -> Option<Value> {
        self.slots
            .borrow()
            .iter()
            .find(|slot| slot.id.same(id))
            .and_then(|slot| slot.value.clone())
    }

    /// Writes a stored field value. Returns `false` when no field slot exists.
    pub(crate) fn set_field(&self, id: &PrivateName, value: Value) -> bool {
        let mut slots = self.slots.borrow_mut();
        if let Some(slot) = slots.iter_mut().find(|slot| slot.id.same(id)) {
            if slot.value.is_some() {
                slot.value = Some(value);
                return true;
            }
        }
        false
    }
}
