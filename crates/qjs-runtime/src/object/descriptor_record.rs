use std::collections::HashMap;

use crate::{ObjectRef, Property, RuntimeError, Value, has_property, is_truthy, property_value};

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
        return Some(Property {
            value: descriptor.value.unwrap_or(Value::Undefined),
            get: None,
            set: None,
            accessor: false,
            writable: descriptor.writable.unwrap_or(false),
            enumerable: descriptor.enumerable.unwrap_or(existing.enumerable),
            configurable: descriptor.configurable.unwrap_or(existing.configurable),
        });
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
            property.get = get;
        }
        if let Some(set) = descriptor.set {
            property.set = set;
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
    env: &mut HashMap<String, Value>,
) -> Result<PropertyDescriptor, RuntimeError> {
    if !matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) {
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

impl PropertyDescriptor {
    fn is_accessor_descriptor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    fn is_data_descriptor(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
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
        Property {
            value: self.value.unwrap_or(Value::Undefined),
            get: None,
            set: None,
            accessor: false,
            writable: self.writable.unwrap_or(false),
            enumerable: self.enumerable.unwrap_or(false),
            configurable: self.configurable.unwrap_or(false),
        }
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
                && !same_optional_value(get, &existing.get)
            {
                return false;
            }
            if let Some(set) = &self.set
                && !same_optional_value(set, &existing.set)
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

fn same_optional_value(left: &Option<Value>, right: &Option<Value>) -> bool {
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
    let mut properties = HashMap::from([
        ("enumerable".to_owned(), Value::Boolean(property.enumerable)),
        (
            "configurable".to_owned(),
            Value::Boolean(property.configurable),
        ),
    ]);
    if property.is_accessor() {
        properties.insert("get".to_owned(), property.get.unwrap_or(Value::Undefined));
        properties.insert("set".to_owned(), property.set.unwrap_or(Value::Undefined));
    } else {
        properties.insert("value".to_owned(), property.value);
        properties.insert("writable".to_owned(), Value::Boolean(property.writable));
    }
    ObjectRef::with_prototype(properties, prototype)
}
