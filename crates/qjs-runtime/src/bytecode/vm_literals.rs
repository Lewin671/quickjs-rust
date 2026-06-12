//! Array, template, and object literal construction for the bytecode VM.
//!
//! These ops pop their element/property values off the operand stack and push
//! the constructed literal. They live apart from the main dispatch loop in
//! `vm.rs` to keep that file focused on opcode dispatch and control flow.

use std::collections::HashMap;

use qjs_ast::ObjectPropertyKind;

use crate::{
    ArrayRef, ObjectRef, Property, RuntimeError, Value, array::iterable_values_with_env, object,
    object_prototype, to_property_key_value,
};

use super::ir::ArrayElementKind;
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

    pub(super) fn new_object(&mut self, kinds: &[ObjectPropertyKind]) -> Result<(), RuntimeError> {
        let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.env));
        let mut entries = Vec::with_capacity(kinds.len());
        for kind in kinds.iter().rev() {
            let value = self.pop()?;
            let key_value = self.pop()?;
            let mut key_env = self.current_env();
            let key = to_property_key_value(key_value, &mut key_env)?;
            self.apply_env(key_env);
            let descriptor = match kind {
                ObjectPropertyKind::Data => Property::enumerable(value),
                ObjectPropertyKind::Getter => Property::accessor(Some(value), None, true, true),
                ObjectPropertyKind::Setter => Property::accessor(None, Some(value), true, true),
            };
            entries.push((key, descriptor));
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
