use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    Function, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_as_object_prototype,
    function::{CompiledUserFunction, InstanceFieldInitializer},
    function_prototype, object, object_prototype, property_value,
    symbol::symbol_function_name_description,
    to_property_key_value,
};
use crate::{
    call_function, construct_function, property_value_key_with_receiver, value_prototype_slot,
};

use super::CaptureWriteback;
use super::ir::{
    Bytecode, ClassComputedKeyDef, ClassConstructorDef, ClassElementDef, ClassFieldDef,
    ClassFieldInitializerDef, ClassMemberKeyDef, ClassMethodDef, ClassMethodKind,
    ClassStaticBlockDef,
};
use super::vm::Vm;
use crate::CallEnv;

/// A class element whose evaluation is deferred to pass 2 of class definition,
/// preserving source order so static fields and static blocks interleave.
enum PendingStaticItem<'a> {
    Field(&'a ClassFieldDef, PropertyKey),
    Block(&'a ClassStaticBlockDef),
}

impl Vm<'_> {
    /// Builds a class constructor function object, wires its `prototype` and
    /// `constructor` properties, and installs prototype and static members.
    ///
    /// Computed member keys are evaluated by the surrounding bytecode unless
    /// they need the class private environment, so generator `yield` and abrupt
    /// completions suspend or unwind at the class definition point while
    /// private names remain visible where required.
    pub(super) fn new_class(
        &mut self,
        name: Option<&str>,
        constructor: &ClassConstructorDef,
        elements: &[ClassElementDef],
        private_elements: &[super::ir::ClassPrivateElementDef],
        computed_key_defs: &[ClassComputedKeyDef],
        has_heritage: bool,
    ) -> Result<Value, RuntimeError> {
        let precomputed_keys = self.pop_precomputed_class_keys(computed_key_defs)?;
        // Resolve the parent
        // constructor and the prototype the new class prototype inherits from.
        let heritage = if has_heritage {
            let mut heritage_env = self.current_env();
            let heritage = ClassHeritage::resolve(self.pop()?, &mut heritage_env)?;
            self.apply_env(heritage_env);
            Some(heritage)
        } else {
            None
        };
        let prototype_parent = match &heritage {
            Some(ClassHeritage::Null) => None,
            Some(ClassHeritage::Parent(parent)) => parent.prototype.clone(),
            None => object_prototype(&self.env).map(crate::Prototype::Object),
        };

        let mut constructor_env =
            self.function_capture_env(&constructor.bytecode, &constructor.local_names);
        self.insert_lexical_captures(&mut constructor_env, &constructor.lexical_captures);
        self.refresh_captured_env(&constructor_env);
        let constructor_captured = Rc::new(RefCell::new(constructor_env.clone()));
        let super_constructor = match &heritage {
            Some(ClassHeritage::Parent(parent)) => Some(parent.constructor.clone()),
            Some(ClassHeritage::Null) => function_prototype_value(&self.env),
            _ => None,
        };
        let constructor_function = Function::new_user_compiled(CompiledUserFunction {
            name: constructor.name.clone(),
            has_name_binding: false,
            params: std::rc::Rc::new(constructor.params.clone()),
            env: constructor_env,
            bytecode: constructor.bytecode.clone(),
            local_names: constructor.local_names.clone(),
            constructable: true,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: true,
            is_derived_constructor: has_heritage,
            is_field_initializer: false,
            home_object: None,
            super_constructor: super_constructor.clone(),
            captured_env: constructor_captured,
            with_stack: self.with_stack.clone(),
            capture_writeback: self
                .class_member_capture_writeback(&constructor.bytecode, &constructor.local_names),
        });

        // Static-side inheritance: a subclass constructor inherits the parent
        // constructor's static members through its [[Prototype]], which is the
        // parent constructor itself. Storing the function directly gives
        // `Object.getPrototypeOf(Sub) === Super` reference identity and lets
        // inherited static methods and static `super.x` resolve live (rather
        // than against a definition-time snapshot).
        let constructor_parent_slot = match &heritage {
            Some(ClassHeritage::Parent(heritage_parent)) => match &heritage_parent.constructor {
                Value::Function(parent) => Some(crate::Prototype::Function(parent.clone())),
                Value::Object(parent) => Some(crate::Prototype::Object(parent.clone())),
                _ => None,
            },
            Some(ClassHeritage::Null) => {
                function_prototype_value(&self.env).and_then(|value| match value {
                    Value::Object(parent) => Some(crate::Prototype::Object(parent)),
                    _ => None,
                })
            }
            None => None,
        };
        if let Some(parent_slot) = constructor_parent_slot {
            let _ = constructor_function.set_internal_prototype_slot(Some(parent_slot));
        }

        let prototype = ObjectRef::with_prototype_slot(HashMap::new(), prototype_parent);
        constructor_function.install_class_prototype(prototype.clone());

        // The constructor's home object is its prototype; static `super.x`
        // resolves through it.
        *constructor_function.home_object.borrow_mut() = Some(Value::Object(prototype.clone()));

        self.create_private_environment(private_elements, &prototype, &constructor_function);
        let computed_keys = self.resolve_computed_class_keys(
            computed_key_defs,
            precomputed_keys,
            &constructor_function,
            name,
        )?;
        let mut computed_keys = computed_keys.into_iter();

        // A class binding is visible to its own methods and constructor under
        // the class name, so methods can reference the class recursively. The
        // binding is immutable, so each function gets its own captured env
        // seeded with the class value rather than sharing a mutable parent env.
        //
        // Pass 1: resolve computed keys in source order, install methods
        // immediately, and stash field definitions (with their resolved keys)
        // and static blocks in source order for pass 2.
        let mut pending = Vec::new();
        let mut queued_private_brands = Vec::new();
        for element in elements {
            if let ClassElementDef::Private(private) = element {
                self.queue_instance_private_method_brand(
                    private,
                    &constructor_function,
                    &mut queued_private_brands,
                );
            }
        }
        for element in elements {
            match element {
                ClassElementDef::Method(method) => {
                    let key = resolve_element_key(&method.key, &mut computed_keys);
                    self.install_method(method, key, &prototype, &constructor_function, name)?;
                }
                ClassElementDef::Field(field) => {
                    let key = resolve_element_key(&field.key, &mut computed_keys);
                    if field.is_static {
                        pending.push(PendingStaticItem::Field(field, key));
                    } else {
                        let initializer = self.build_field_initializer(
                            field,
                            &prototype,
                            &constructor_function,
                            name,
                        );
                        constructor_function.push_instance_public_field(InstanceFieldInitializer {
                            key,
                            initializer,
                        });
                    }
                }
                ClassElementDef::Private(private) => {
                    self.queue_instance_private_element(
                        private,
                        &prototype,
                        &constructor_function,
                        name,
                    );
                }
                ClassElementDef::StaticBlock(block) => {
                    pending.push(PendingStaticItem::Block(block));
                }
            }
        }

        // Install private elements: register shared private methods/accessors,
        // brand the constructor with static privates (running static private
        // field initializers now), and queue instance private brands/fields for
        // construction time. Done after public methods so private static field
        // initializers can reference private methods.
        self.install_private_elements(private_elements, &prototype, &constructor_function, name)?;

        // Pass 2: static fields and static blocks are evaluated now, after all
        // method definitions, in source order, with `this` = the constructor.
        for item in pending {
            match item {
                PendingStaticItem::Field(field, key) => {
                    let initializer = self.build_field_initializer(
                        field,
                        &prototype,
                        &constructor_function,
                        name,
                    );
                    let value = match &initializer {
                        Some(thunk) => self.run_field_initializer(
                            thunk,
                            Value::Function(constructor_function.clone()),
                        )?,
                        None => Value::Undefined,
                    };
                    install_field_value(
                        &Value::Function(constructor_function.clone()),
                        key,
                        value,
                        &mut self.realm_env(),
                    )?;
                    self.refresh_instance_field_captures(&constructor_function);
                }
                PendingStaticItem::Block(block) => {
                    self.run_static_block(block, &constructor_function, name)?;
                    self.refresh_instance_field_captures(&constructor_function);
                }
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

    fn refresh_instance_field_captures(&self, constructor_function: &Function) {
        for element in constructor_function.instance_elements().iter() {
            match element {
                crate::function::InstanceElementInitializer::PublicField(field) => {
                    if let Some(initializer) = &field.initializer {
                        self.refresh_function_captures(initializer);
                    }
                }
                crate::function::InstanceElementInitializer::PrivateElement(private) => {
                    if let Some(field) = &private.field_initializer {
                        if let Some(initializer) = &field.initializer {
                            self.refresh_function_captures(initializer);
                        }
                    }
                }
            }
        }
    }

    fn refresh_function_captures(&self, function: &Function) {
        let names = function
            .captured_env
            .borrow()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut captured = function.captured_env.borrow_mut();
        for name in names {
            if let Some(value) = self
                .current_local_binding(&name)
                .cloned()
                .or_else(|| self.env.locals().get(&name).cloned())
            {
                captured.insert(name, value);
            }
        }
    }

    fn pop_precomputed_class_keys(
        &mut self,
        computed_key_defs: &[ClassComputedKeyDef],
    ) -> Result<Vec<PropertyKey>, RuntimeError> {
        let computed_key_count = computed_key_defs
            .iter()
            .filter(|key| matches!(key, ClassComputedKeyDef::Precomputed))
            .count();
        let mut keys = Vec::with_capacity(computed_key_count);
        for _ in 0..computed_key_count {
            let value = self.pop()?;
            let mut env = self.current_env();
            let key = to_property_key_value(value, &mut env)?;
            self.apply_env(env);
            keys.push(key);
        }
        keys.reverse();
        Ok(keys)
    }

    fn resolve_computed_class_keys(
        &mut self,
        computed_key_defs: &[ClassComputedKeyDef],
        precomputed_keys: Vec<PropertyKey>,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<Vec<PropertyKey>, RuntimeError> {
        let mut precomputed_keys = precomputed_keys.into_iter();
        let mut keys = Vec::with_capacity(computed_key_defs.len());
        for key_def in computed_key_defs {
            match key_def {
                ClassComputedKeyDef::Precomputed => {
                    keys.push(
                        precomputed_keys
                            .next()
                            .expect("precomputed key count matches descriptors"),
                    );
                }
                ClassComputedKeyDef::Deferred {
                    local_names,
                    bytecode,
                } => {
                    let mut key_env = self.function_capture_env(bytecode, local_names);
                    bind_class_inner_name(&mut key_env, name, constructor_function);
                    let thunk = Function::new_user_compiled(CompiledUserFunction {
                        name: None,
                        has_name_binding: false,
                        params: std::rc::Rc::new(qjs_ast::FunctionParams::positional(Vec::new())),
                        env: key_env.clone(),
                        bytecode: bytecode.clone(),
                        local_names: local_names.clone(),
                        constructable: false,
                        is_strict: true,
                        lexical_this: false,
                        lexical_arguments: false,
                        is_generator: false,
                        is_async: false,
                        is_class_constructor: false,
                        is_derived_constructor: false,
                        is_field_initializer: false,
                        home_object: Some(Value::Function(constructor_function.clone())),
                        super_constructor: None,
                        captured_env: Rc::new(RefCell::new(key_env)),
                        with_stack: self.with_stack.clone(),
                        capture_writeback: self
                            .class_member_capture_writeback(bytecode, local_names),
                    });
                    let this_value = self.env.get("this").unwrap_or(Value::Undefined);
                    let value = self.run_field_initializer(&thunk, this_value)?;
                    let mut env = self.current_env();
                    let key = to_property_key_value(value, &mut env)?;
                    self.apply_env(env);
                    keys.push(key);
                }
            }
        }
        Ok(keys)
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
            name: class_method_function_name(method, &key),
            has_name_binding: false,
            params: std::rc::Rc::new(method.params.clone()),
            env: method_env.clone(),
            bytecode: method.bytecode.clone(),
            local_names: method.local_names.clone(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: method.is_generator,
            is_async: method.is_async,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            home_object: Some(home_object.clone()),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(method_env)),
            with_stack: self.with_stack.clone(),
            capture_writeback: self
                .class_member_capture_writeback(&method.bytecode, &method.local_names),
        });
        if method.is_generator && method.is_async {
            crate::async_generator::wire_async_generator_function_intrinsics(
                &method_function,
                &self.env,
            );
        } else if method.is_generator {
            self.wire_generator_function_intrinsics(&method_function);
        } else if method.is_async {
            self.wire_async_function_intrinsics(&method_function);
        }
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

        // Member definition only needs realm access (array length coercion on
        // concrete values); no frame bindings can change here.
        let mut prop_env = self.realm_env();
        let success = object::define_property_on_value_key(target, key, descriptor, &mut prop_env)?;
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
            has_name_binding: false,
            params: std::rc::Rc::new(qjs_ast::FunctionParams::positional(Vec::new())),
            env: field_env.clone(),
            bytecode: bytecode.clone(),
            local_names: local_names.clone(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: true,
            home_object: Some(home_object),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(field_env)),
            with_stack: self.with_stack.clone(),
            capture_writeback: self.class_member_capture_writeback(bytecode, local_names),
        }))
    }

    pub(super) fn class_member_capture_writeback(
        &self,
        bytecode: &Bytecode,
        local_names: &[String],
    ) -> Option<CaptureWriteback> {
        let mut names = Vec::new();
        for name in bytecode.global_names() {
            self.push_member_capture_name(&mut names, name);
        }
        for name in bytecode.local_names() {
            if local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_err()
            {
                self.push_member_capture_name(&mut names, name);
            }
        }
        (!names.is_empty()).then(|| CaptureWriteback {
            target: self.captured_env.clone(),
            names,
            aliases: Vec::new(),
            parent: None,
        })
    }

    fn push_member_capture_name(&self, names: &mut Vec<String>, name: &str) {
        if crate::function::is_internal_binding_name(name) {
            return;
        }
        if self.current_local_binding(name).is_none() && !self.env.locals().contains_key(name) {
            return;
        }
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }

    /// Runs a `static { ... }` block at class definition: builds a parameterless
    /// strict thunk whose home object and `this` are the constructor (so
    /// `super.x` resolves against the constructor's [[Prototype]]) and calls it.
    fn run_static_block(
        &mut self,
        block: &ClassStaticBlockDef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let ClassStaticBlockDef {
            local_names,
            bytecode,
        } = block;
        let mut block_env = self.function_capture_env(bytecode, local_names);
        bind_class_inner_name(&mut block_env, name, constructor_function);
        let thunk = Function::new_user_compiled(CompiledUserFunction {
            name: None,
            has_name_binding: false,
            params: std::rc::Rc::new(qjs_ast::FunctionParams::positional(Vec::new())),
            env: block_env.clone(),
            bytecode: bytecode.clone(),
            local_names: local_names.clone(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            home_object: Some(Value::Function(constructor_function.clone())),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(block_env)),
            with_stack: self.with_stack.clone(),
            capture_writeback: self.class_member_capture_writeback(bytecode, local_names),
        });
        self.run_field_initializer(&thunk, Value::Function(constructor_function.clone()))?;
        Ok(())
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
        self.refresh_call_env_from_captured_env(&mut env);
        self.apply_env(env);
        self.refresh_locals_from_captured_env();
        result
    }

    /// Resolves `super.<key>` (or `super[key]`): the property is looked up on
    /// the current method's home object [[Prototype]] with the current `this`
    /// as the receiver, so inherited accessors run with the right `this`.
    pub(super) fn super_get(&mut self, key: &PropertyKey) -> Result<Value, RuntimeError> {
        let receiver = self.current_this()?;
        let lookup_base = self.super_lookup_base()?;
        // GetSuperBase yields the home object's [[Prototype]]; reading a property
        // off it requires RequireObjectCoercible, so a `null` super base (e.g.
        // `extends null` or a null-proto home object) throws a TypeError.
        if matches!(lookup_base, Value::Null | Value::Undefined) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot read property of null or undefined super base"
                    .to_owned(),
            });
        }
        let mut env = self.current_env();
        let value = property_value_key_with_receiver(lookup_base, key, receiver, &mut env)?;
        self.apply_env(env);
        Ok(value)
    }

    /// Resolves `super.<key> = value`: the write targets the current method's
    /// home object [[Prototype]] and uses current `this` as the receiver.
    pub(super) fn super_set(
        &mut self,
        key: &PropertyKey,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        self.super_set_value(key.clone(), value, is_strict)
    }

    pub(super) fn super_set_value(
        &mut self,
        key: PropertyKey,
        value: Value,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        let receiver = self.current_this()?;
        let lookup_base = self.super_lookup_base()?;
        if matches!(lookup_base, Value::Null | Value::Undefined) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property on null or undefined super base"
                    .to_owned(),
            });
        }
        let mut env = self.current_env();
        let wrote =
            crate::reflect::ordinary_set(lookup_base, &key, value.clone(), receiver, &mut env)?;
        self.apply_env(env);
        if !wrote && is_strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property".to_owned(),
            });
        }
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
            self.env.insert("this".to_owned(), this_value.clone());
            self.write_through_captured("this", this_value.clone());
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
        let Some(Value::Function(constructor)) = self.env.get(crate::ACTIVE_CONSTRUCTOR_BINDING)
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
        let Some(super_constructor) = self.env.get(crate::SUPER_CONSTRUCTOR_BINDING) else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        let new_target = self
            .env
            .get(crate::NEW_TARGET_BINDING)
            .unwrap_or_else(|| super_constructor.clone());

        let mut env = self.current_env();
        let result = construct_function(super_constructor, new_target, arguments, &mut env);
        self.apply_env(env);
        let value = result?;
        // A repeated `super(...)` still performs the parent construction after
        // argument evaluation, then fails while trying to initialize `this`.
        if self.env.locals().contains_key("this") {
            return Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: super constructor may only be called once".to_owned(),
            });
        }
        Ok(value)
    }

    fn current_this(&mut self) -> Result<Value, RuntimeError> {
        match self.env.get_local("this") {
            Some(value) => Ok(value),
            None => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing 'this'"
                    .to_owned(),
            }),
        }
    }

    pub(super) fn require_super_this(&mut self) -> Result<(), RuntimeError> {
        self.current_this().map(|_| ())
    }

    /// Returns the lookup base for `super` property access: the [[Prototype]]
    /// of the current method's home object.
    fn super_lookup_base(&self) -> Result<Value, RuntimeError> {
        let Some(home) = self.env.get(crate::HOME_OBJECT_BINDING) else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        match value_prototype_slot(home, &self.env) {
            Some(prototype) => Ok(prototype.to_value()),
            None => Ok(Value::Undefined),
        }
    }

    /// Wires a freshly created generator function into the generator intrinsic
    /// chain: its [[Prototype]] becomes `%GeneratorFunction.prototype%`, and its
    /// own `prototype` property's [[Prototype]] becomes `%GeneratorPrototype%`
    /// so generator instances inherit `next`/`return`/`throw`.
    pub(super) fn wire_generator_function_intrinsics(&self, function: &Function) {
        crate::generator::wire_generator_function_intrinsics(function, &self.env);
    }

    /// Wires a freshly created async function into the async intrinsic chain:
    /// its `[[Prototype]]` becomes `%AsyncFunction.prototype%`. Async functions
    /// are non-constructable and carry no own `prototype` property.
    pub(super) fn wire_async_function_intrinsics(&self, function: &Function) {
        crate::async_function::wire_async_function_intrinsics(function, &self.env);
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
    prototype: Option<crate::Prototype>,
}

fn function_prototype_value(env: &CallEnv) -> Option<Value> {
    let Some(Value::Function(function_constructor)) = env.get("Function") else {
        return None;
    };
    function_prototype(&function_constructor).map(Value::Object)
}

impl ClassHeritage {
    fn resolve(value: Value, env: &mut CallEnv) -> Result<Self, RuntimeError> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::Function(function) if function.constructable => {
                let constructor = Value::Function(function);
                let prototype = class_heritage_prototype_slot(
                    property_value(constructor.clone(), "prototype", env)?,
                    env,
                )?;
                Ok(Self::Parent(Box::new(ClassHeritageParent {
                    constructor,
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

fn class_heritage_prototype_slot(
    value: Value,
    env: &CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    match value {
        Value::Null => Ok(None),
        Value::Object(prototype) if crate::symbol::is_symbol_primitive(&prototype) => {
            Err(class_heritage_prototype_error())
        }
        Value::Object(prototype) => Ok(Some(crate::Prototype::Object(prototype))),
        Value::Function(prototype) => Ok(Some(crate::Prototype::Function(prototype))),
        Value::Array(array) => Ok(Some(crate::Prototype::Object(array_as_object_prototype(
            &array, env,
        )))),
        _ => Err(class_heritage_prototype_error()),
    }
}

fn class_heritage_prototype_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: superclass prototype must be an object or null".to_owned(),
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

pub(super) fn class_method_function_name(
    method: &ClassMethodDef,
    key: &PropertyKey,
) -> Option<String> {
    let base_name = method
        .name
        .clone()
        .or_else(|| function_name_from_property_key(key));
    class_method_function_name_with_base(method.method_kind, base_name)
}

pub(super) fn class_method_function_name_with_base(
    kind: ClassMethodKind,
    base_name: Option<String>,
) -> Option<String> {
    match kind {
        ClassMethodKind::Method => base_name,
        ClassMethodKind::Getter => Some(format!("get {}", base_name.unwrap_or_default())),
        ClassMethodKind::Setter => Some(format!("set {}", base_name.unwrap_or_default())),
    }
}

pub(super) fn function_name_from_property_key(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::String(name) => Some(name.clone()),
        PropertyKey::Symbol(symbol) => {
            let name = symbol_function_name_description(symbol)
                .map(|description| format!("[{description}]"))
                .unwrap_or_default();
            Some(name)
        }
    }
}

/// Installs a field value on a target via CreateDataPropertyOrThrow semantics:
/// an enumerable, writable, configurable own data property.
pub(crate) fn install_field_value(
    target: &Value,
    key: PropertyKey,
    value: Value,
    env: &mut crate::CallEnv,
) -> Result<(), RuntimeError> {
    let descriptor = Property::data(value, true, true, true);
    let success = object::define_property_on_value_key(target.clone(), key, descriptor, env)?;
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
