//! Array, template, and object literal construction for the bytecode VM.
//!
//! These ops pop their element/property values off the operand stack and push
//! the constructed literal. They live apart from the main dispatch loop in
//! `vm.rs` to keep that file focused on opcode dispatch and control flow.

use std::collections::HashMap;

use qjs_ast::ObjectPropertyKind;

use crate::{
    ArrayRef, ObjectRef, Property, PropertyKey, Prototype, RuntimeError, Value,
    array::iterable_values_with_env, object, to_property_key_value, value::ObjectLiteralShape,
};

use super::ir::{ArrayElementKind, ComputedNameKind, ObjectPropertyMeta};
use super::util::stack_underflow;
use super::vm::Vm;
use super::vm_class::function_name_from_property_key;

impl Vm<'_> {
    pub(super) fn new_array(&mut self, elements: &[ArrayElementKind]) -> Result<(), RuntimeError> {
        let value_count = elements
            .iter()
            .filter(|element| !matches!(element, ArrayElementKind::Elision))
            .count();
        let mut element_values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            element_values.push(self.pop()?);
        }
        element_values.reverse();

        if value_count == elements.len()
            && elements
                .iter()
                .all(|element| matches!(element, ArrayElementKind::Expr))
        {
            self.stack.push(Value::Array(ArrayRef::new(element_values)));
            return Ok(());
        }

        let mut values = Vec::new();
        let mut holes = Vec::new();
        let mut next_value = element_values.into_iter();
        for element in elements {
            match element {
                ArrayElementKind::Expr => {
                    values.push(next_value.next().ok_or_else(stack_underflow)?);
                }
                ArrayElementKind::Elision => {
                    holes.push(values.len());
                    values.push(Value::Undefined);
                }
                ArrayElementKind::Spread => {
                    let value = next_value.next().ok_or_else(stack_underflow)?;
                    let mut env = self.current_env();
                    let result = iterable_values_with_env(value, "array spread", &mut env);
                    self.apply_env(env);
                    // Route an iterator error through the try-handler stack so a
                    // throw during `[...iterable]` is catchable, instead of
                    // escaping the VM loop. On a handled throw, stop building the
                    // array; the catch handler resets the stack.
                    match self.handle_runtime_result(result)? {
                        Some(spread_values) => values.extend(spread_values),
                        None => return Ok(()),
                    }
                }
            }
        }
        self.stack
            .push(Value::Array(ArrayRef::new_sparse(values, holes)));
        Ok(())
    }

    pub(super) fn new_template_object(
        &mut self,
        site: usize,
        cooked: &[Option<String>],
        raw: &[String],
    ) {
        if let Some(value) = self.bytecode.template_objects.borrow().get(&site).cloned() {
            self.stack.push(value);
            return;
        }

        let cooked_values = cooked
            .iter()
            .cloned()
            .map(|s| match s {
                Some(s) => Value::String(s.into()),
                None => Value::Undefined,
            })
            .collect::<Vec<_>>();
        let raw_values = raw
            .iter()
            .cloned()
            .map(|s| Value::String(s.into()))
            .collect::<Vec<_>>();
        let cooked_array = ArrayRef::new(cooked_values);
        let raw_array = ArrayRef::new(raw_values);
        raw_array.freeze();
        cooked_array.define_property(
            "raw".to_owned(),
            Property::fixed_non_enumerable(Value::Array(raw_array)),
        );
        cooked_array.freeze();
        let template_object = Value::Array(cooked_array);
        self.bytecode
            .template_objects
            .borrow_mut()
            .insert(site, template_object.clone());
        self.stack.push(template_object);
    }

    pub(super) fn new_object_literal(&mut self) {
        let prototype = self.cached_object_prototype();
        self.stack.push(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            prototype,
        )));
    }

    pub(super) fn new_object_data_literal(
        &mut self,
        shape: std::rc::Rc<ObjectLiteralShape>,
    ) -> Result<(), RuntimeError> {
        if shape.input_len() == 2 && shape.unique_len() == 2 {
            let second = self.pop()?;
            let first = self.pop()?;
            let home_functions = [&first, &second].map(|value| match value {
                Value::Function(function) if !function.constructable => Some(function.clone()),
                _ => None,
            });
            let prototype = self.cached_object_prototype();
            let object = ObjectRef::with_literal_pair(shape, [first, second], prototype);
            for function in home_functions.into_iter().flatten() {
                function.set_home_object(Value::Object(object.clone()));
            }
            self.stack.push(Value::Object(object));
            return Ok(());
        }

        let mut values = Vec::with_capacity(shape.input_len());
        for _ in 0..shape.input_len() {
            values.push(self.pop()?);
        }
        values.reverse();
        let home_functions = values
            .iter()
            .filter_map(|value| match value {
                Value::Function(function) if !function.constructable => Some(function.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let prototype = self.cached_object_prototype();
        let object = ObjectRef::with_literal_properties(shape, values, prototype);
        for function in home_functions {
            function.set_home_object(Value::Object(object.clone()));
        }
        self.stack.push(Value::Object(object));
        Ok(())
    }

    /// Names an anonymous object-literal function/accessor from its computed
    /// key (`[k]() {}`, `get [k]() {}`, `{ [k]: () => {} }`). The key is
    /// converted to a property key once here and the normalized primitive is
    /// pushed back so the following `DefineObjectProperty` does not re-run any
    /// key coercion side effects.
    pub(super) fn set_computed_function_name(
        &mut self,
        kind: ComputedNameKind,
    ) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key_value = self.pop()?;
        let mut key_env = self.current_env();
        let key = to_property_key_value(key_value, &mut key_env)?;
        self.apply_env(key_env);
        if let Value::Function(ref function) = value {
            let base = function_name_from_property_key(&key).unwrap_or_default();
            let name = match kind {
                ComputedNameKind::Plain => base,
                ComputedNameKind::Getter => format!("get {base}"),
                ComputedNameKind::Setter => format!("set {base}"),
            };
            function.define_property(
                "name".to_owned(),
                Property::data(Value::String(name.into()), false, false, true),
            );
        }
        self.stack.push(key.into_value());
        self.stack.push(value);
        Ok(())
    }

    pub(super) fn define_object_property(
        &mut self,
        meta: ObjectPropertyMeta,
    ) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key_value = self.pop()?;
        let object = self.object_literal_target()?;
        if meta.is_proto_setter {
            apply_object_literal_proto(&object, value, &self.env)?;
            return Ok(());
        }
        // Methods defined on object literals need their home object set so
        // that `super.x` resolves the prototype chain correctly.
        if let Value::Function(ref function) = value {
            if !function.constructable {
                function.set_home_object(Value::Object(object.clone()));
            }
        }
        let mut key_env = self.current_env();
        let key = to_property_key_value(key_value, &mut key_env)?;
        self.apply_env(key_env);
        let descriptor = object_property_descriptor(meta.kind, value)?;
        define_object_literal_property(&object, key, descriptor, &mut self.realm_env())
    }

    pub(super) fn copy_object_spread(&mut self) -> Result<(), RuntimeError> {
        let source = self.pop()?;
        if matches!(source, Value::Null | Value::Undefined) {
            self.object_literal_target()?;
            return Ok(());
        }
        let object = self.object_literal_target()?;
        let mut env = self.current_env();
        let result = object::enumerable_property_entries_with_symbols(source, &mut env);
        self.apply_env(env);
        // A getter invoked while gathering `{...source}` properties may throw;
        // route it through the try-handler stack so it is catchable.
        let Some(entries) = self.handle_runtime_result(result)? else {
            return Ok(());
        };
        for (key, value) in entries {
            define_object_literal_property(
                &object,
                key,
                Property::enumerable(value),
                &mut self.realm_env(),
            )?;
        }
        Ok(())
    }

    fn object_literal_target(&self) -> Result<ObjectRef, RuntimeError> {
        match self.stack.last() {
            Some(Value::Object(object)) => Ok(object.clone()),
            _ => Err(RuntimeError {
                thrown: None,
                message: "object literal target missing".to_owned(),
            }),
        }
    }
}

fn object_property_descriptor(
    kind: ObjectPropertyKind,
    value: Value,
) -> Result<Property, RuntimeError> {
    match kind {
        ObjectPropertyKind::Data => Ok(Property::enumerable(value)),
        ObjectPropertyKind::Getter => Ok(Property::accessor(Some(value), None, true, true)),
        ObjectPropertyKind::Setter => Ok(Property::accessor(None, Some(value), true, true)),
        ObjectPropertyKind::Spread => Err(RuntimeError {
            thrown: None,
            message: "object spread is not a keyed property".to_owned(),
        }),
    }
}

fn apply_object_literal_proto(
    object: &ObjectRef,
    proto: Value,
    env: &crate::CallEnv,
) -> Result<(), RuntimeError> {
    let prototype = match proto {
        Value::Null => Some(None),
        Value::Object(object) if crate::symbol::is_symbol_primitive(&object) => None,
        Value::Object(object) => Some(Some(Prototype::Object(object))),
        Value::Function(function) => Some(Some(Prototype::Function(function))),
        Value::Array(array) => Some(Some(Prototype::Object(crate::array_as_object_prototype(
            &array, env,
        )))),
        Value::Map(map) => Some(Some(Prototype::Object(map.object()))),
        Value::Set(set) => Some(Some(Prototype::Object(set.object()))),
        // Proxy and primitive proto values are ignored by the special form.
        _ => None,
    };
    if let Some(prototype) = prototype {
        object
            .set_prototype_slot(prototype)
            .map_err(|()| RuntimeError {
                thrown: None,
                message: "object literal __proto__ assignment failed".to_owned(),
            })?;
    }
    Ok(())
}

fn define_object_literal_property(
    object: &ObjectRef,
    key: PropertyKey,
    mut descriptor: Property,
    env: &mut crate::CallEnv,
) -> Result<(), RuntimeError> {
    if descriptor.is_accessor()
        && let Some(existing) = match &key {
            PropertyKey::String(key) => object.own_property(key),
            PropertyKey::Symbol(symbol) => object.own_symbol_property(symbol),
        }
        && existing.is_accessor()
    {
        descriptor.get = descriptor.get.or(existing.get);
        descriptor.set = descriptor.set.or(existing.set);
    }
    let success =
        object::define_property_on_value_key(Value::Object(object.clone()), key, descriptor, env)?;
    if !success {
        return Err(RuntimeError {
            thrown: None,
            message: "object literal property definition failed".to_owned(),
        });
    }
    Ok(())
}
