//! Array, template, and object literal construction for the bytecode VM.
//!
//! These ops pop their element/property values off the operand stack and push
//! the constructed literal. They live apart from the main dispatch loop in
//! `vm.rs` to keep that file focused on opcode dispatch and control flow.

use std::collections::HashMap;

use qjs_ast::ObjectPropertyKind;

use crate::{
    ArrayRef, ObjectRef, Property, Prototype, RuntimeError, Value, array::iterable_values_with_env,
    object, object_prototype, to_property_key_value,
};

use super::ir::{ArrayElementKind, ObjectPropertyMeta};
use super::util::stack_underflow;
use super::vm::Vm;

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
                    let spread_values = iterable_values_with_env(value, "array spread", &mut env)?;
                    self.apply_env(env);
                    values.extend(spread_values);
                }
            }
        }
        self.stack
            .push(Value::Array(ArrayRef::new_sparse(values, holes)));
        Ok(())
    }

    pub(super) fn new_template_object(&mut self, cooked: &[String], raw: &[String]) {
        let cooked_values = cooked
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        let raw_values = raw.iter().cloned().map(Value::String).collect::<Vec<_>>();
        let cooked_array = ArrayRef::new(cooked_values);
        let raw_array = ArrayRef::new(raw_values);
        raw_array.freeze();
        cooked_array.define_property(
            "raw".to_owned(),
            Property::fixed_non_enumerable(Value::Array(raw_array)),
        );
        cooked_array.freeze();
        self.stack.push(Value::Array(cooked_array));
    }

    pub(super) fn new_object(&mut self, kinds: &[ObjectPropertyMeta]) -> Result<(), RuntimeError> {
        let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.env));
        // Proto-setter entries (`{ __proto__: expr }`) set [[Prototype]] in
        // source order rather than defining an own property; collect them
        // separately so ordinary properties keep their stack-popping order.
        let mut entries = Vec::with_capacity(kinds.len());
        let mut proto_assignments = Vec::new();
        for meta in kinds.iter().rev() {
            let value = self.pop()?;
            let key_value = self.pop()?;
            if meta.is_proto_setter {
                proto_assignments.push(value);
                continue;
            }
            let mut key_env = self.current_env();
            let key = to_property_key_value(key_value, &mut key_env)?;
            self.apply_env(key_env);
            let descriptor = match meta.kind {
                ObjectPropertyKind::Data => Property::enumerable(value),
                ObjectPropertyKind::Getter => Property::accessor(Some(value), None, true, true),
                ObjectPropertyKind::Setter => Property::accessor(None, Some(value), true, true),
            };
            entries.push((key, descriptor));
        }
        // Apply prototype assignments in source order (reverse of pop order).
        for proto in proto_assignments.into_iter().rev() {
            let prototype = match proto {
                Value::Null => Some(None),
                Value::Object(object) if crate::symbol::is_symbol_primitive(&object) => None,
                Value::Object(object) => Some(Some(Prototype::Object(object))),
                Value::Function(function) => Some(Some(Prototype::Function(function))),
                Value::Array(array) => Some(Some(Prototype::Object(
                    crate::array_as_object_prototype(&array, &self.env),
                ))),
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
        }
        for (key, mut descriptor) in entries.into_iter().rev() {
            if descriptor.is_accessor()
                && let Some(existing) = match &key {
                    crate::PropertyKey::String(key) => object.own_property(key),
                    crate::PropertyKey::Symbol(symbol) => object.own_symbol_property(symbol),
                }
                && existing.is_accessor()
            {
                descriptor.get = descriptor.get.or(existing.get);
                descriptor.set = descriptor.set.or(existing.set);
            }
            let mut prop_env = self.realm_env();
            let success = object::define_property_on_value_key(
                Value::Object(object.clone()),
                key,
                descriptor,
                &mut prop_env,
            )?;
            if !success {
                return Err(RuntimeError {
                    thrown: None,
                    message: "object literal property definition failed".to_owned(),
                });
            }
        }
        self.stack.push(Value::Object(object));
        Ok(())
    }
}
