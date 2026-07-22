use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ObjectRef, Property, RuntimeError, Value, has_property, is_truthy, object_prototype,
    property_value,
};

pub(crate) fn resolve_property_definition(
    existing: Option<Property>,
    descriptor: PropertyDescriptor,
) -> Option<Property> {
    match existing {
        Some(existing) => resolve_existing_property_definition(existing, descriptor),
        None => Some(descriptor.complete_new_property()),
    }
}

fn resolve_existing_property_definition(
    existing: Property,
    descriptor: PropertyDescriptor,
) -> Option<Property> {
    if !existing.configurable && !descriptor.is_compatible_with_non_configurable(&existing) {
        return None;
    }

    if descriptor.is_accessor_descriptor() && !existing.is_accessor() {
        return Some(Property::accessor(
            descriptor.get.unwrap_or(None),
            descriptor.set.unwrap_or(None),
            descriptor.enumerable.unwrap_or(existing.enumerable),
            descriptor.configurable.unwrap_or(existing.configurable),
        ));
    }
    if descriptor.is_data_descriptor() && existing.is_accessor() {
        return Some(Property::data(
            descriptor.value.unwrap_or(Value::Undefined),
            descriptor.enumerable.unwrap_or(existing.enumerable),
            descriptor.writable.unwrap_or(false),
            descriptor.configurable.unwrap_or(existing.configurable),
        ));
    }

    let mut property = existing;
    if let Some(enumerable) = descriptor.enumerable {
        property.enumerable = enumerable;
    }
    if let Some(configurable) = descriptor.configurable {
        property.configurable = configurable;
    }
    if property.is_accessor() {
        if let Some(get) = descriptor.get {
            property.set_getter(get);
        }
        if let Some(set) = descriptor.set {
            property.set_setter(set);
        }
    } else {
        if let Some(value) = descriptor.value {
            property.value = value;
        }
        if let Some(writable) = descriptor.writable {
            property.writable = writable;
        }
    }
    Some(property)
}

pub(crate) fn to_property_descriptor_record(
    value: Value,
    env: &mut CallEnv,
) -> Result<PropertyDescriptor, RuntimeError> {
    // Symbols are represented as `Value::Object` wrappers, but a Symbol is a
    // primitive and is not a valid property descriptor object.
    let is_object = match &value {
        Value::Object(object) => !crate::symbol::is_symbol_primitive(object),
        Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    };
    if !is_object {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor must be an object".to_owned(),
        });
    }

    let has_enumerable = has_property(value.clone(), env, "enumerable")?;
    let enumerable = if has_enumerable {
        Some(is_truthy(&property_value(
            value.clone(),
            "enumerable",
            env,
        )?))
    } else {
        None
    };
    let has_configurable = has_property(value.clone(), env, "configurable")?;
    let configurable = if has_configurable {
        Some(is_truthy(&property_value(
            value.clone(),
            "configurable",
            env,
        )?))
    } else {
        None
    };
    let has_value = has_property(value.clone(), env, "value")?;
    let descriptor_value = if has_value {
        Some(property_value(value.clone(), "value", env)?)
    } else {
        None
    };
    let has_writable = has_property(value.clone(), env, "writable")?;
    let writable = if has_writable {
        Some(is_truthy(&property_value(value.clone(), "writable", env)?))
    } else {
        None
    };
    let has_get = has_property(value.clone(), env, "get")?;
    let get = if has_get {
        Some(accessor_function(
            property_value(value.clone(), "get", env)?,
            "get",
        )?)
    } else {
        None
    };
    let has_set = has_property(value.clone(), env, "set")?;
    let set = if has_set {
        Some(accessor_function(
            property_value(value.clone(), "set", env)?,
            "set",
        )?)
    } else {
        None
    };

    if (has_get || has_set) && (has_value || has_writable) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor cannot mix accessor and data fields".to_owned(),
        });
    }
    Ok(PropertyDescriptor {
        value: descriptor_value,
        writable,
        get,
        set,
        enumerable,
        configurable,
    })
}

#[derive(Clone)]
pub(crate) struct PropertyDescriptor {
    pub(super) value: Option<Value>,
    pub(super) writable: Option<bool>,
    get: Option<Option<Value>>,
    set: Option<Option<Value>>,
    enumerable: Option<bool>,
    configurable: Option<bool>,
}

/// Builds a plain descriptor object carrying only the fields that the
/// descriptor record actually specifies. Used to forward a `defineProperty`
/// request to a Proxy handler trap, which receives a descriptor object rather
/// than a record.
pub(crate) fn property_descriptor_record_object(
    descriptor: &PropertyDescriptor,
    env: &CallEnv,
) -> ObjectRef {
    let result = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    if let Some(value) = &descriptor.value {
        result.define_property("value".to_owned(), Property::enumerable(value.clone()));
    }
    if let Some(writable) = descriptor.writable {
        result.define_property(
            "writable".to_owned(),
            Property::enumerable(Value::Boolean(writable)),
        );
    }
    if let Some(get) = &descriptor.get {
        result.define_property(
            "get".to_owned(),
            Property::enumerable(get.clone().unwrap_or(Value::Undefined)),
        );
    }
    if let Some(set) = &descriptor.set {
        result.define_property(
            "set".to_owned(),
            Property::enumerable(set.clone().unwrap_or(Value::Undefined)),
        );
    }
    if let Some(enumerable) = descriptor.enumerable {
        result.define_property(
            "enumerable".to_owned(),
            Property::enumerable(Value::Boolean(enumerable)),
        );
    }
    if let Some(configurable) = descriptor.configurable {
        result.define_property(
            "configurable".to_owned(),
            Property::enumerable(Value::Boolean(configurable)),
        );
    }
    result
}

impl PropertyDescriptor {
    pub(crate) fn data(value: Value, writable: bool, enumerable: bool, configurable: bool) -> Self {
        Self {
            value: Some(value),
            writable: Some(writable),
            get: None,
            set: None,
            enumerable: Some(enumerable),
            configurable: Some(configurable),
        }
    }

    pub(crate) fn accessor_get(get: Value, enumerable: bool, configurable: bool) -> Self {
        Self {
            value: None,
            writable: None,
            get: Some(Some(get)),
            set: None,
            enumerable: Some(enumerable),
            configurable: Some(configurable),
        }
    }

    pub(crate) fn accessor_set(set: Value, enumerable: bool, configurable: bool) -> Self {
        Self {
            value: None,
            writable: None,
            get: None,
            set: Some(Some(set)),
            enumerable: Some(enumerable),
            configurable: Some(configurable),
        }
    }

    pub(crate) fn from_complete_property(property: Property) -> Self {
        if property.is_accessor() {
            let enumerable = property.enumerable;
            let configurable = property.configurable;
            let (get, set) = property
                .into_accessor_parts()
                .expect("accessor properties have accessor state");
            return Self {
                value: None,
                writable: None,
                get: Some(get),
                set: Some(set),
                enumerable: Some(enumerable),
                configurable: Some(configurable),
            };
        }
        Self::data(
            property.value,
            property.writable,
            property.enumerable,
            property.configurable,
        )
    }

    /// The partial descriptor SetIntegrityLevel applies when sealing
    /// (`{ [[Configurable]]: false }`), or when freezing an accessor property.
    pub(crate) fn integrity_non_configurable() -> Self {
        Self {
            value: None,
            writable: None,
            get: None,
            set: None,
            enumerable: None,
            configurable: Some(false),
        }
    }

    /// The partial descriptor SetIntegrityLevel applies when freezing a data
    /// property (`{ [[Configurable]]: false, [[Writable]]: false }`).
    pub(crate) fn integrity_frozen_data() -> Self {
        Self {
            value: None,
            writable: Some(false),
            get: None,
            set: None,
            enumerable: None,
            configurable: Some(false),
        }
    }

    /// A partial data descriptor carrying only `[[Value]]`, as required when
    /// OrdinarySetWithOwnDescriptor updates an existing receiver property.
    pub(crate) fn data_value(value: Value) -> Self {
        Self {
            value: Some(value),
            writable: None,
            get: None,
            set: None,
            enumerable: None,
            configurable: None,
        }
    }

    pub(crate) fn is_accessor_descriptor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    pub(crate) fn complete_accessor_halves(&mut self) {
        if self.is_accessor_descriptor() {
            self.get.get_or_insert(None);
            self.set.get_or_insert(None);
        }
    }

    fn is_data_descriptor(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
    }

    /// Completes a descriptor returned by a Proxy `getOwnPropertyDescriptor`
    /// trap into a full property, filling absent fields with their defaults.
    pub(crate) fn complete_for_get_own(self) -> Property {
        self.complete_new_property()
    }

    pub(crate) fn configurable_field(&self) -> Option<bool> {
        self.configurable
    }

    pub(crate) fn enumerable_field(&self) -> Option<bool> {
        self.enumerable
    }

    pub(crate) fn writable_field(&self) -> Option<bool> {
        self.writable
    }

    pub(crate) fn value_field(&self) -> Option<&Value> {
        self.value.as_ref()
    }

    /// Whether this descriptor (as returned by a Proxy `defineProperty` request)
    /// is compatible with an existing target property under the
    /// IsCompatiblePropertyDescriptor rules, given the target's extensibility.
    pub(crate) fn is_compatible_for_proxy_define(
        &self,
        existing: Option<&Property>,
        extensible: bool,
    ) -> bool {
        match existing {
            None => extensible,
            Some(existing) => {
                if existing.configurable {
                    return true;
                }
                self.is_compatible_with_non_configurable(existing)
            }
        }
    }

    fn complete_new_property(self) -> Property {
        if self.is_accessor_descriptor() {
            return Property::accessor(
                self.get.unwrap_or(None),
                self.set.unwrap_or(None),
                self.enumerable.unwrap_or(false),
                self.configurable.unwrap_or(false),
            );
        }
        Property::data(
            self.value.unwrap_or(Value::Undefined),
            self.enumerable.unwrap_or(false),
            self.writable.unwrap_or(false),
            self.configurable.unwrap_or(false),
        )
    }

    fn is_compatible_with_non_configurable(&self, existing: &Property) -> bool {
        if self.configurable == Some(true) {
            return false;
        }
        if let Some(enumerable) = self.enumerable
            && enumerable != existing.enumerable
        {
            return false;
        }
        if self.is_accessor_descriptor() != existing.is_accessor()
            && (self.is_accessor_descriptor() || self.is_data_descriptor())
        {
            return false;
        }
        if existing.is_accessor() {
            if let Some(get) = &self.get
                && !same_optional_value(get.as_ref(), existing.getter())
            {
                return false;
            }
            if let Some(set) = &self.set
                && !same_optional_value(set.as_ref(), existing.setter())
            {
                return false;
            }
            return true;
        }
        if !existing.writable {
            if self.writable == Some(true) {
                return false;
            }
            if let Some(value) = &self.value
                && !value.same_value(&existing.value)
            {
                return false;
            }
        }
        true
    }
}

fn same_optional_value(left: Option<&Value>, right: Option<&Value>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.same_value(right),
        (None, None) => true,
        _ => false,
    }
}

fn accessor_function(value: Value, name: &str) -> Result<Option<Value>, RuntimeError> {
    match value {
        Value::Undefined => Ok(None),
        Value::Function(_) => Ok(Some(value)),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("property descriptor {name} must be callable or undefined"),
        }),
    }
}

pub(crate) fn property_descriptor_object(
    property: Property,
    prototype: Option<ObjectRef>,
) -> ObjectRef {
    let result = ObjectRef::with_prototype(HashMap::new(), prototype);
    if property.is_accessor() {
        let enumerable = property.enumerable;
        let configurable = property.configurable;
        let (get, set) = property
            .into_accessor_parts()
            .expect("accessor properties have accessor state");
        result.define_property(
            "get".to_owned(),
            Property::enumerable(get.unwrap_or(Value::Undefined)),
        );
        result.define_property(
            "set".to_owned(),
            Property::enumerable(set.unwrap_or(Value::Undefined)),
        );
        result.define_property(
            "enumerable".to_owned(),
            Property::enumerable(Value::Boolean(enumerable)),
        );
        result.define_property(
            "configurable".to_owned(),
            Property::enumerable(Value::Boolean(configurable)),
        );
        return result;
    }
    result.define_property("value".to_owned(), Property::enumerable(property.value));
    result.define_property(
        "writable".to_owned(),
        Property::enumerable(Value::Boolean(property.writable)),
    );
    result.define_property(
        "enumerable".to_owned(),
        Property::enumerable(Value::Boolean(property.enumerable)),
    );
    result.define_property(
        "configurable".to_owned(),
        Property::enumerable(Value::Boolean(property.configurable)),
    );
    result
}
