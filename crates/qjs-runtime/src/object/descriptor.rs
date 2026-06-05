use std::collections::HashMap;

use crate::{
    ObjectRef, Property, RuntimeError, Value, function_own_property_descriptor, has_property,
    is_truthy, object_prototype, property_value, to_number_with_env, to_property_key,
    to_property_key_with_env, to_uint32_number,
};

use super::enumeration::{enumerable_property_entries, own_property_names};

#[derive(Clone, Debug)]
pub(crate) struct Descriptor {
    value: Option<Value>,
    get: Option<Option<Value>>,
    set: Option<Option<Value>>,
    writable: Option<bool>,
    enumerable: Option<bool>,
    configurable: Option<bool>,
}

impl Descriptor {
    fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    fn is_data(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
    }

    fn complete(self) -> Property {
        if self.is_accessor() {
            return Property::accessor(
                self.get.flatten(),
                self.set.flatten(),
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
}

pub(crate) fn native_object_define_property(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key_with_env(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let descriptor = to_property_descriptor(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    if !define_property_on_value(target.clone(), key, descriptor, env)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty failed".to_owned(),
        });
    }
    Ok(target)
}

pub(crate) fn native_object_define_properties(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if matches!(descriptors, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors, env)? {
        let descriptor = to_property_descriptor(descriptor_value, env)?;
        if !define_property_on_value(target.clone(), key, descriptor, env)? {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.defineProperties failed".to_owned(),
            });
        }
    }
    Ok(target)
}

pub(crate) fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let Some(property) = own_property_descriptor(target, &key)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

pub(crate) fn native_object_get_own_property_descriptors(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertyDescriptors target must not be null or undefined"
                .to_owned(),
        });
    }

    let prototype = object_prototype(env);
    let mut descriptors = HashMap::new();
    for key in own_property_names(target.clone()) {
        if let Some(property) = own_property_descriptor(target.clone(), &key)? {
            descriptors.insert(
                key,
                Value::Object(property_descriptor_object(property, prototype.clone())),
            );
        }
    }

    Ok(Value::Object(ObjectRef::with_prototype(
        descriptors,
        prototype,
    )))
}

pub(super) fn own_property_descriptor(
    value: Value,
    key: &str,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.own_property(key)),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key)),
        Value::Array(elements) => Ok(crate::array_own_property_descriptor(&elements, key)),
        Value::String(value) => Ok(crate::string::string_own_property_descriptor(&value, key)),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

pub(crate) fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Descriptor,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    match &target {
        Value::Object(object) => {
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            let property = define_ordinary_property(object.own_property(&key), descriptor)?;
            let Some(property) = property else {
                return Ok(false);
            };
            object.define_property(key, property);
            Ok(true)
        }
        Value::Function(function) => {
            let existing = function_own_property_descriptor(function, &key);
            if existing.is_none() && !function.is_extensible() {
                return Ok(false);
            }
            let property = define_ordinary_property(existing, descriptor)?;
            let Some(property) = property else {
                return Ok(false);
            };
            function.properties.borrow_mut().insert(key, property);
            Ok(true)
        }
        Value::Array(elements) => {
            let existing = crate::array_own_property_descriptor(elements, &key);
            if existing.is_none() && !elements.is_extensible() {
                return Ok(false);
            }
            if key == "length" {
                let new_length = descriptor
                    .value
                    .clone()
                    .map(|value| array_length_from_descriptor_value(value, env))
                    .transpose()?;
                let Some(property) = define_ordinary_property(existing, descriptor)? else {
                    return Ok(false);
                };
                if let Some(new_length) = new_length {
                    if new_length != elements.len() && !elements.try_set_len(new_length) {
                        elements.set_length_writable(property.writable);
                        return Ok(false);
                    }
                }
                elements.set_length_writable(property.writable);
            } else {
                if let Ok(index) = key.parse::<usize>()
                    && index >= elements.len()
                    && !elements.is_length_writable()
                {
                    return Ok(false);
                }
                let Some(property) = define_ordinary_property(existing, descriptor)? else {
                    return Ok(false);
                };
                elements.define_property(key, property);
            }
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            unreachable!("define property target validation should reject unsupported values")
        }
    }
}

fn array_length_from_descriptor_value(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let length = to_uint32_number(number);
    if number != f64::from(length) {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    Ok(length as usize)
}

fn define_ordinary_property(
    existing: Option<Property>,
    descriptor: Descriptor,
) -> Result<Option<Property>, RuntimeError> {
    let Some(existing) = existing else {
        return Ok(Some(descriptor.complete()));
    };
    if !is_compatible_descriptor(&existing, &descriptor) {
        return Ok(None);
    }
    Ok(Some(merge_property(existing, descriptor)))
}

fn is_compatible_descriptor(existing: &Property, descriptor: &Descriptor) -> bool {
    if existing.configurable {
        return true;
    }
    if descriptor.configurable == Some(true) {
        return false;
    }
    if let Some(enumerable) = descriptor.enumerable
        && enumerable != existing.enumerable
    {
        return false;
    }
    if descriptor.is_accessor() != existing.is_accessor() {
        return false;
    }
    if existing.is_accessor() {
        if let Some(get) = &descriptor.get
            && !same_optional_value(get.as_ref(), existing.get.as_ref())
        {
            return false;
        }
        if let Some(set) = &descriptor.set
            && !same_optional_value(set.as_ref(), existing.set.as_ref())
        {
            return false;
        }
        return true;
    }
    if descriptor.writable == Some(true) && !existing.writable {
        return false;
    }
    if !existing.writable
        && let Some(value) = &descriptor.value
        && !value.same_value(&existing.value)
    {
        return false;
    }
    true
}

fn same_optional_value(left: Option<&Value>, right: Option<&Value>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.same_value(right),
        _ => false,
    }
}

fn merge_property(existing: Property, descriptor: Descriptor) -> Property {
    if descriptor.is_accessor() {
        return Property::accessor(
            descriptor.get.unwrap_or(existing.get),
            descriptor.set.unwrap_or(existing.set),
            descriptor.enumerable.unwrap_or(existing.enumerable),
            descriptor.configurable.unwrap_or(existing.configurable),
        );
    }
    if descriptor.is_data() || !existing.is_accessor() {
        return Property::data(
            descriptor.value.unwrap_or(existing.value),
            descriptor.enumerable.unwrap_or(existing.enumerable),
            descriptor.writable.unwrap_or(existing.writable),
            descriptor.configurable.unwrap_or(existing.configurable),
        );
    }
    Property::data(
        descriptor.value.unwrap_or(Value::Undefined),
        descriptor.enumerable.unwrap_or(existing.enumerable),
        descriptor.writable.unwrap_or(false),
        descriptor.configurable.unwrap_or(existing.configurable),
    )
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) | Value::Array(_) => Ok(()),
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty primitive targets are not implemented".to_owned(),
        }),
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn to_property_descriptor(
    descriptor: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Descriptor, RuntimeError> {
    if !matches!(
        descriptor,
        Value::Object(_) | Value::Function(_) | Value::Array(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor must be an object".to_owned(),
        });
    }

    let has_get = has_property(descriptor.clone(), env, "get")?;
    let has_set = has_property(descriptor.clone(), env, "set")?;
    if has_get || has_set {
        if has_property(descriptor.clone(), env, "value")?
            || has_property(descriptor.clone(), env, "writable")?
        {
            return Err(RuntimeError {
                thrown: None,
                message: "property descriptor cannot mix accessor and data fields".to_owned(),
            });
        }
        return Ok(Descriptor {
            value: None,
            get: descriptor_accessor(descriptor.clone(), env, "get")?,
            set: descriptor_accessor(descriptor.clone(), env, "set")?,
            writable: None,
            enumerable: descriptor_bool(descriptor.clone(), env, "enumerable")?,
            configurable: descriptor_bool(descriptor, env, "configurable")?,
        });
    }

    Ok(Descriptor {
        value: descriptor_value(descriptor.clone(), env, "value")?,
        get: None,
        set: None,
        writable: descriptor_bool(descriptor.clone(), env, "writable")?,
        enumerable: descriptor_bool(descriptor.clone(), env, "enumerable")?,
        configurable: descriptor_bool(descriptor, env, "configurable")?,
    })
}

fn descriptor_bool(
    descriptor: Value,
    env: &mut HashMap<String, Value>,
    key: &str,
) -> Result<Option<bool>, RuntimeError> {
    if !has_property(descriptor.clone(), env, key)? {
        return Ok(None);
    }
    Ok(Some(is_truthy(&property_value(descriptor, key, env)?)))
}

fn descriptor_value(
    descriptor: Value,
    env: &mut HashMap<String, Value>,
    key: &str,
) -> Result<Option<Value>, RuntimeError> {
    if !has_property(descriptor.clone(), env, key)? {
        return Ok(None);
    }
    Ok(Some(property_value(descriptor, key, env)?))
}

fn descriptor_accessor(
    descriptor: Value,
    env: &mut HashMap<String, Value>,
    key: &str,
) -> Result<Option<Option<Value>>, RuntimeError> {
    if !has_property(descriptor.clone(), env, key)? {
        return Ok(None);
    }
    Ok(Some(accessor_function(
        property_value(descriptor, key, env)?,
        key,
    )?))
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

fn property_descriptor_object(property: Property, prototype: Option<ObjectRef>) -> ObjectRef {
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
