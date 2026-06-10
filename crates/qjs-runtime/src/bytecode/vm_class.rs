use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    Function, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    function::CompiledUserFunction, object, object_prototype, to_property_key_value,
};

use super::ir::{ClassConstructorDef, ClassMemberKeyDef, ClassMethodDef, ClassMethodKind};
use super::vm::Vm;

impl Vm<'_> {
    /// Builds a class constructor function object, wires its `prototype` and
    /// `constructor` properties, and installs prototype and static members.
    ///
    /// Computed member keys were evaluated, in member order, before the
    /// `NewClass` op; they sit on the stack and are consumed here.
    pub(super) fn new_class(
        &mut self,
        name: Option<&str>,
        constructor: &ClassConstructorDef,
        methods: &[ClassMethodDef],
        computed_key_count: usize,
    ) -> Result<Value, RuntimeError> {
        // Computed keys were pushed in member order; pop them and convert to
        // property keys, preserving member order.
        let mut computed_keys = Vec::with_capacity(computed_key_count);
        for _ in 0..computed_key_count {
            let value = self.pop()?;
            computed_keys.push(to_property_key_value(value, &mut self.globals)?);
        }
        computed_keys.reverse();
        let mut computed_keys = computed_keys.into_iter();

        let constructor_env =
            self.function_capture_env(&constructor.bytecode, &constructor.local_names);
        self.refresh_captured_env(&constructor_env);
        let constructor_captured = Rc::new(RefCell::new(constructor_env.clone()));
        let constructor_function = Function::new_user_compiled(CompiledUserFunction {
            name: constructor.name.clone(),
            params: constructor.params.clone(),
            env: constructor_env,
            bytecode: constructor.bytecode.clone(),
            local_names: constructor.local_names.clone(),
            constructable: true,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_class_constructor: true,
            captured_env: constructor_captured,
        });

        let prototype = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.globals));
        constructor_function.install_class_prototype(prototype.clone());

        // A class binding is visible to its own methods and constructor under
        // the class name, so methods can reference the class recursively. The
        // binding is immutable, so each function gets its own captured env
        // seeded with the class value rather than sharing a mutable parent env.
        for method in methods {
            let key = match &method.key {
                ClassMemberKeyDef::Literal(key) => PropertyKey::String(key.clone()),
                ClassMemberKeyDef::Computed => computed_keys
                    .next()
                    .expect("computed key count matches members"),
            };

            let mut method_env = self.function_capture_env(&method.bytecode, &method.local_names);
            bind_class_inner_name(&mut method_env, name, &constructor_function);
            let method_function = Function::new_user_compiled(CompiledUserFunction {
                name: method.name.clone(),
                params: method.params.clone(),
                env: method_env.clone(),
                bytecode: method.bytecode.clone(),
                local_names: method.local_names.clone(),
                constructable: false,
                is_strict: true,
                lexical_this: false,
                lexical_arguments: false,
                is_class_constructor: false,
                captured_env: Rc::new(RefCell::new(method_env)),
            });
            let function_value = Value::Function(method_function);

            // Static members live on the constructor; instance members on the
            // prototype.
            let target = if method.is_static {
                Value::Function(constructor_function.clone())
            } else {
                Value::Object(prototype.clone())
            };

            let descriptor = match method.method_kind {
                // Methods are non-enumerable, writable, configurable.
                ClassMethodKind::Method => Property::data(function_value, false, true, true),
                // Accessors are non-enumerable, configurable; merge with an
                // existing accessor for the same key.
                ClassMethodKind::Getter => merge_accessor(
                    &target,
                    &key,
                    Property::accessor(Some(function_value), None, false, true),
                ),
                ClassMethodKind::Setter => merge_accessor(
                    &target,
                    &key,
                    Property::accessor(None, Some(function_value), false, true),
                ),
            };

            let success = object::define_property_on_value_key(target, key, descriptor)?;
            if !success {
                return Err(RuntimeError {
                    thrown: None,
                    message: "class member definition failed".to_owned(),
                });
            }
        }

        // Seed the constructor's own captured env with the inner class binding.
        bind_class_inner_name(
            &mut constructor_function.captured_env.borrow_mut(),
            name,
            &constructor_function,
        );

        Ok(Value::Function(constructor_function))
    }
}

fn bind_class_inner_name(
    env: &mut HashMap<String, Value>,
    name: Option<&str>,
    constructor: &Function,
) {
    if let Some(name) = name {
        env.insert(name.to_owned(), Value::Function(constructor.clone()));
    }
}

/// Merges a new accessor descriptor with any existing accessor for the same
/// key on `target`, so a `get`/`set` pair declared separately combines into a
/// single accessor property.
fn merge_accessor(target: &Value, key: &PropertyKey, mut descriptor: Property) -> Property {
    if let Some(existing) = own_property_for_key(target, key)
        && existing.is_accessor()
    {
        descriptor.get = descriptor.get.or(existing.get);
        descriptor.set = descriptor.set.or(existing.set);
    }
    descriptor
}

fn own_property_for_key(target: &Value, key: &PropertyKey) -> Option<Property> {
    match (target, key) {
        (Value::Function(function), PropertyKey::String(key)) => function.own_property(key),
        (Value::Function(function), PropertyKey::Symbol(symbol)) => {
            function.own_symbol_property(symbol)
        }
        (Value::Object(object), PropertyKey::String(key)) => object.own_property(key),
        (Value::Object(object), PropertyKey::Symbol(symbol)) => object.own_symbol_property(symbol),
        _ => None,
    }
}
