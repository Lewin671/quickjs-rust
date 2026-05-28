use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use crate::Function;

/// A JavaScript value supported by the current runtime subset.
#[derive(Clone)]
pub enum Value {
    /// Number value.
    Number(f64),
    /// String value.
    String(String),
    /// Boolean value.
    Boolean(bool),
    /// Null value.
    Null,
    /// Undefined value.
    Undefined,
    /// User-defined function.
    Function(Function),
    /// Array object value.
    Array(ArrayRef),
    /// Object value.
    Object(ObjectRef),
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => formatter.debug_tuple("Number").field(value).finish(),
            Self::String(value) => formatter.debug_tuple("String").field(value).finish(),
            Self::Boolean(value) => formatter.debug_tuple("Boolean").field(value).finish(),
            Self::Null => formatter.write_str("Null"),
            Self::Undefined => formatter.write_str("Undefined"),
            Self::Function(function) => formatter.debug_tuple("Function").field(function).finish(),
            Self::Array(elements) => formatter.debug_tuple("Array").field(elements).finish(),
            Self::Object(object) => formatter.debug_tuple("Object").field(object).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl Value {
    pub(crate) fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => {
                (left.is_nan() && right.is_nan()) || left.to_bits() == right.to_bits()
            }
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

/// Array storage reference.
#[derive(Clone)]
pub struct ArrayRef {
    elements: Rc<RefCell<Vec<Value>>>,
    extensible: Rc<Cell<bool>>,
}

impl ArrayRef {
    pub(crate) fn new(elements: Vec<Value>) -> Self {
        Self {
            elements: Rc::new(RefCell::new(elements)),
            extensible: Rc::new(Cell::new(true)),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.elements, &other.elements)
    }

    pub(crate) fn len(&self) -> usize {
        self.elements.borrow().len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.elements.borrow().is_empty()
    }

    pub(crate) fn get(&self, index: usize) -> Option<Value> {
        self.elements.borrow().get(index).cloned()
    }

    pub(crate) fn to_vec(&self) -> Vec<Value> {
        self.elements.borrow().clone()
    }

    pub(crate) fn push(&self, value: Value) -> usize {
        let mut elements = self.elements.borrow_mut();
        elements.push(value);
        elements.len()
    }

    pub(crate) fn pop(&self) -> Option<Value> {
        self.elements.borrow_mut().pop()
    }

    pub(crate) fn shift(&self) -> Option<Value> {
        let mut elements = self.elements.borrow_mut();
        if elements.is_empty() {
            None
        } else {
            Some(elements.remove(0))
        }
    }

    pub(crate) fn unshift(&self, values: &[Value]) -> usize {
        let mut elements = self.elements.borrow_mut();
        elements.splice(0..0, values.iter().cloned());
        elements.len()
    }

    pub(crate) fn reverse(&self) {
        self.elements.borrow_mut().reverse();
    }

    pub(crate) fn fill(&self, start: usize, end: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        for element in &mut elements[start..end] {
            *element = value.clone();
        }
    }

    pub(crate) fn set(&self, index: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        if index >= elements.len() {
            if !self.extensible.get() {
                return;
            }
            elements.resize(index + 1, Value::Undefined);
        }
        elements[index] = value;
    }

    pub(crate) fn set_len(&self, length: usize) {
        let mut elements = self.elements.borrow_mut();
        if length > elements.len() && !self.extensible.get() {
            return;
        }
        elements.resize(length, Value::Undefined);
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }
}

impl fmt::Debug for ArrayRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArrayRef")
            .field("len", &self.elements.borrow().len())
            .finish()
    }
}

/// Object storage reference.
#[derive(Clone)]
pub struct ObjectRef {
    properties: Rc<RefCell<HashMap<String, Property>>>,
    extensible: Rc<Cell<bool>>,
    prototype: Option<Box<ObjectRef>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) value: Value,
    pub(crate) enumerable: bool,
    pub(crate) writable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    pub(crate) fn data(value: Value, enumerable: bool, writable: bool, configurable: bool) -> Self {
        Self {
            value,
            enumerable,
            writable,
            configurable,
        }
    }

    pub(crate) fn enumerable(value: Value) -> Self {
        Self {
            value,
            enumerable: true,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn non_enumerable(value: Value) -> Self {
        Self {
            value,
            enumerable: false,
            writable: true,
            configurable: true,
        }
    }
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.properties.borrow().len())
            .field("has_prototype", &self.prototype.is_some())
            .finish()
    }
}

impl ObjectRef {
    pub(crate) fn new(properties: HashMap<String, Value>) -> Self {
        Self::with_prototype(properties, None)
    }

    pub(crate) fn with_prototype(
        properties: HashMap<String, Value>,
        prototype: Option<ObjectRef>,
    ) -> Self {
        Self {
            properties: Rc::new(RefCell::new(
                properties
                    .into_iter()
                    .map(|(key, value)| (key, Property::enumerable(value)))
                    .collect(),
            )),
            extensible: Rc::new(Cell::new(true)),
            prototype: prototype.map(Box::new),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.properties, &other.properties)
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.properties
            .borrow()
            .get(key)
            .map(|property| property.value.clone())
            .or_else(|| self.prototype.as_deref().and_then(|proto| proto.get(key)))
    }

    pub(crate) fn set(&self, key: String, value: Value) {
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(&key)
            .is_some_and(|property| !property.writable)
        {
            return;
        }
        if !properties.contains_key(&key) && !self.extensible.get() {
            return;
        }
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        self.properties.borrow_mut().insert(key, property);
    }

    pub(crate) fn define_non_enumerable(&self, key: String, value: Value) {
        self.properties
            .borrow_mut()
            .insert(key, Property::non_enumerable(value));
    }

    pub(crate) fn contains_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
            || self
                .prototype
                .as_deref()
                .is_some_and(|proto| proto.contains_property(key))
    }

    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn delete_own_property(&self, key: &str) {
        self.properties.borrow_mut().remove(key);
    }

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self
            .properties
            .borrow()
            .iter()
            .filter(|(_, property)| property.enumerable)
            .map(|(key, _)| key.clone())
            .collect();
        keys.sort();
        keys
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.properties.borrow().keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn has_prototype(&self, prototype: &ObjectRef) -> bool {
        self.prototype
            .as_deref()
            .is_some_and(|proto| proto.ptr_eq(prototype) || proto.has_prototype(prototype))
    }

    pub(crate) fn prototype(&self) -> Option<ObjectRef> {
        self.prototype.as_deref().cloned()
    }
}
