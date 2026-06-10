use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    Function, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    function::{CompiledUserFunction, InstanceFieldInitializer},
    object, object_prototype, to_property_key_value,
};
use crate::{call_function, construct_function, property_value_key_with_receiver, value_prototype};

use super::ir::{
    ClassConstructorDef, ClassElementDef, ClassFieldDef, ClassFieldInitializerDef,
    ClassMemberKeyDef, ClassMethodDef, ClassMethodKind,
};
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
        elements: &[ClassElementDef],
        private_elements: &[super::ir::ClassPrivateElementDef],
        computed_key_count: usize,
        has_heritage: bool,
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

        // The heritage value sits below the computed keys. Resolve the parent
        // constructor and the prototype the new class prototype inherits from.
        let heritage = if has_heritage {
            Some(ClassHeritage::resolve(self.pop()?, &self.globals)?)
        } else {
            None
        };
        let prototype_parent = match &heritage {
            Some(ClassHeritage::Null) => None,
            Some(ClassHeritage::Parent(parent)) => parent.prototype.clone(),
            None => object_prototype(&self.globals),
        };

        let constructor_env =
            self.function_capture_env(&constructor.bytecode, &constructor.local_names);
        self.refresh_captured_env(&constructor_env);
        let constructor_captured = Rc::new(RefCell::new(constructor_env.clone()));
        let super_constructor = match &heritage {
            Some(ClassHeritage::Parent(parent)) => Some(parent.constructor.clone()),
            _ => None,
        };
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
            is_generator: false,
            is_class_constructor: true,
            is_derived_constructor: has_heritage,
            home_object: None,
            super_constructor: super_constructor.clone(),
            captured_env: constructor_captured,
        });

        // Static-side inheritance: a subclass constructor inherits the parent
        // constructor's static members through its [[Prototype]]. The runtime
        // models a function's [[Prototype]] as an object, so we mirror the
        // parent constructor's own properties into a backing object and use it
        // as the subclass constructor's [[Prototype]]. This makes inherited
        // static methods and static `super.x` resolve. It does not provide
        // `Object.getPrototypeOf(Sub) === Super` reference identity (a known
        // limitation, tracked as a follow-up: functions cannot yet be a
        // [[Prototype]] value), and the mirror snapshots the parent's static
        // members at definition time.
        if let Some(ClassHeritage::Parent(heritage_parent)) = &heritage
            && let Value::Function(parent) = &heritage_parent.constructor
        {
            let mirror = crate::function::static_inheritance_mirror(parent);
            let _ = constructor_function.set_internal_prototype(Some(mirror));
        }

        let prototype = ObjectRef::with_prototype(HashMap::new(), prototype_parent);
        constructor_function.install_class_prototype(prototype.clone());

        // The constructor's home object is its prototype; static `super.x`
        // resolves through it.
        *constructor_function.home_object.borrow_mut() = Some(Value::Object(prototype.clone()));

        // A class binding is visible to its own methods and constructor under
        // the class name, so methods can reference the class recursively. The
        // binding is immutable, so each function gets its own captured env
        // seeded with the class value rather than sharing a mutable parent env.
        //
        // Pass 1: resolve computed keys in source order, install methods
        // immediately, and stash field definitions with their resolved keys.
        let mut pending_fields = Vec::new();
        for element in elements {
            match element {
                ClassElementDef::Method(method) => {
                    let key = resolve_element_key(&method.key, &mut computed_keys);
                    self.install_method(method, key, &prototype, &constructor_function, name)?;
                }
                ClassElementDef::Field(field) => {
                    let key = resolve_element_key(&field.key, &mut computed_keys);
                    pending_fields.push((field, key));
                }
            }
        }

        // Install private elements: register shared private methods/accessors,
        // brand the constructor with static privates (running static private
        // field initializers now), and queue instance private brands/fields for
        // construction time. Done after public methods so private static field
        // initializers can reference private methods.
        self.install_private_elements(private_elements, &prototype, &constructor_function, name)?;

        // Pass 2: instance fields become constructor initializers (run at
        // construction time); static fields are evaluated now, after all method
        // definitions, with `this` = the constructor.
        for (field, key) in pending_fields {
            let initializer =
                self.build_field_initializer(field, &prototype, &constructor_function, name);
            if field.is_static {
                let value = match &initializer {
                    Some(thunk) => self.run_field_initializer(
                        thunk,
                        Value::Function(constructor_function.clone()),
                    )?,
                    None => Value::Undefined,
                };
                install_field_value(&Value::Function(constructor_function.clone()), key, value)?;
            } else {
                constructor_function
                    .instance_fields
                    .borrow_mut()
                    .push(InstanceFieldInitializer { key, initializer });
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

    /// Builds and installs a single method/accessor on the prototype (instance)
    /// or the constructor (static).
    fn install_method(
        &self,
        method: &ClassMethodDef,
        key: PropertyKey,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let mut method_env = self.function_capture_env(&method.bytecode, &method.local_names);
        bind_class_inner_name(&mut method_env, name, constructor_function);
        // A method's home object resolves `super.x`: instance methods and
        // accessors use the prototype; static members use the constructor.
        let home_object = if method.is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
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
            is_generator: method.is_generator,
            is_class_constructor: false,
            is_derived_constructor: false,
            home_object: Some(home_object.clone()),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(method_env)),
        });
        let function_value = Value::Function(method_function);

        // Static members live on the constructor; instance members on the
        // prototype.
        let target = home_object;

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
        Ok(())
    }

    /// Builds the initializer thunk function for a field. The thunk runs with
    /// `this` bound at call time; its home object resolves `super.x` (instance
    /// fields use the prototype, static fields the constructor).
    fn build_field_initializer(
        &self,
        field: &ClassFieldDef,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Option<Function> {
        let ClassFieldInitializerDef {
            local_names,
            bytecode,
        } = field.initializer.as_ref()?;
        let mut field_env = self.function_capture_env(bytecode, local_names);
        bind_class_inner_name(&mut field_env, name, constructor_function);
        let home_object = if field.is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
        Some(Function::new_user_compiled(CompiledUserFunction {
            name: None,
            params: qjs_ast::FunctionParams::positional(Vec::new()),
            env: field_env.clone(),
            bytecode: bytecode.clone(),
            local_names: local_names.clone(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            home_object: Some(home_object),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(field_env)),
        }))
    }

    /// Runs a field initializer thunk with the given `this` value and returns
    /// its result.
    fn run_field_initializer(
        &mut self,
        thunk: &Function,
        this_value: Value,
    ) -> Result<Value, RuntimeError> {
        let mut env = self.current_env();
        let result = call_function(
            Value::Function(thunk.clone()),
            this_value,
            Vec::new(),
            &mut env,
            false,
        );
        self.apply_env(env);
        result
    }

    /// Resolves `super.<key>` (or `super[key]`): the property is looked up on
    /// the current method's home object [[Prototype]] with the current `this`
    /// as the receiver, so inherited accessors run with the right `this`.
    pub(super) fn super_get(&mut self, key: &PropertyKey) -> Result<Value, RuntimeError> {
        let receiver = self.current_this()?;
        let lookup_base = self.super_lookup_base()?;
        let mut env = self.current_env();
        let value = property_value_key_with_receiver(lookup_base, key, receiver, &mut env)?;
        self.apply_env(env);
        Ok(value)
    }

    /// Resolves `super.<key>` as a method call target, pushing `[this, callee]`
    /// for a following `CallResolved`.
    pub(super) fn super_method(&mut self, key: PropertyKey) -> Result<(), RuntimeError> {
        let receiver = self.current_this()?;
        let callee = self.super_get(&key)?;
        self.stack.push(receiver);
        self.stack.push(callee);
        Ok(())
    }

    /// Evaluates `super(...)` in a derived constructor: constructs the parent
    /// with the current `new.target`, binds the result as `this`, and pushes
    /// it. Calling `super(...)` after `this` is already bound is a
    /// ReferenceError.
    pub(super) fn super_call(&mut self, arguments: Vec<Value>) -> Result<(), RuntimeError> {
        let result = self.super_call_inner(arguments);
        if let Some(this_value) = self.handle_runtime_result(result)? {
            self.globals.insert("this".to_owned(), this_value.clone());
            // The instance fields of the derived class initialize immediately
            // after `super(...)` binds `this`, before the rest of the body.
            let field_result = self.initialize_derived_instance_fields(&this_value);
            if self.handle_runtime_result(field_result)?.is_none() {
                return Ok(());
            }
            self.stack.push(this_value);
        }
        Ok(())
    }

    /// Runs the active derived constructor's instance-field initializers once
    /// `super(...)` has bound `this`.
    fn initialize_derived_instance_fields(
        &mut self,
        this_value: &Value,
    ) -> Result<Value, RuntimeError> {
        let Some(Value::Function(constructor)) =
            self.globals.get(crate::ACTIVE_CONSTRUCTOR_BINDING).cloned()
        else {
            return Ok(Value::Undefined);
        };
        let mut env = self.current_env();
        let result =
            crate::function::initialize_instance_fields(&constructor, this_value, &mut env);
        self.apply_env(env);
        result.map(|()| Value::Undefined)
    }

    fn super_call_inner(&mut self, arguments: Vec<Value>) -> Result<Value, RuntimeError> {
        if self.globals.contains_key("this") {
            return Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: super constructor may only be called once".to_owned(),
            });
        }
        let Some(super_constructor) = self.globals.get(crate::SUPER_CONSTRUCTOR_BINDING).cloned()
        else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        let new_target = self
            .globals
            .get(crate::NEW_TARGET_BINDING)
            .cloned()
            .unwrap_or_else(|| super_constructor.clone());

        let mut env = self.current_env();
        let result = construct_function(super_constructor, new_target, arguments, &mut env);
        self.apply_env(env);
        result
    }

    fn current_this(&mut self) -> Result<Value, RuntimeError> {
        match self.globals.get("this").cloned() {
            Some(value) => Ok(value),
            None => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing 'this'"
                    .to_owned(),
            }),
        }
    }

    /// Returns the lookup base for `super` property access: the [[Prototype]]
    /// of the current method's home object.
    fn super_lookup_base(&self) -> Result<Value, RuntimeError> {
        let Some(home) = self.globals.get(crate::HOME_OBJECT_BINDING).cloned() else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        match value_prototype(home, &self.globals) {
            Some(prototype) => Ok(Value::Object(prototype)),
            None => Ok(Value::Undefined),
        }
    }
}

/// The resolved heritage of a class with an `extends` clause.
enum ClassHeritage {
    /// `extends null`: the prototype object has a null [[Prototype]].
    Null,
    /// `extends <constructor>`: the parent constructor and its `prototype`.
    Parent(Box<ClassHeritageParent>),
}

struct ClassHeritageParent {
    constructor: Value,
    prototype: Option<ObjectRef>,
}

impl ClassHeritage {
    fn resolve(value: Value, env: &HashMap<String, Value>) -> Result<Self, RuntimeError> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Function(function) if function.constructable => {
                let prototype =
                    crate::constructor_prototype(&Value::Function(function.clone()), env);
                Ok(Self::Parent(Box::new(ClassHeritageParent {
                    constructor: Value::Function(function),
                    prototype,
                })))
            }
            _ => Err(RuntimeError {
                thrown: None,
                message: "TypeError: class heritage must be a constructor or null".to_owned(),
            }),
        }
    }
}

/// Resolves a class element's key: a literal key is taken directly; a computed
/// key consumes the next value from the source-ordered computed-key iterator.
fn resolve_element_key(
    key: &ClassMemberKeyDef,
    computed_keys: &mut impl Iterator<Item = PropertyKey>,
) -> PropertyKey {
    match key {
        ClassMemberKeyDef::Literal(key) => PropertyKey::String(key.clone()),
        ClassMemberKeyDef::Computed => computed_keys
            .next()
            .expect("computed key count matches elements"),
    }
}

/// Installs a field value on a target via CreateDataPropertyOrThrow semantics:
/// an enumerable, writable, configurable own data property.
pub(crate) fn install_field_value(
    target: &Value,
    key: PropertyKey,
    value: Value,
) -> Result<(), RuntimeError> {
    let descriptor = Property::data(value, true, true, true);
    let success = object::define_property_on_value_key(target.clone(), key, descriptor)?;
    if !success {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot define class field".to_owned(),
        });
    }
    Ok(())
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
