//! Class evaluation for the interpreter: `Op::NewClass` (constructor,
//! heritage, methods, fields, static blocks) and the `super` property,
//! method and call operations.

use crate::{
    Function, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_as_prototype_slot,
    function::CompiledUserFunction, object, property_value,
    symbol::symbol_function_name_description,
};
use crate::{
    call_field_initializer, construct_function, property_value_key_with_receiver,
    value_prototype_slot,
};

use super::ir::{
    ClassFieldDef, ClassFieldInitializerDef, ClassMethodDef, ClassMethodKind, ClassStaticBlockDef,
};
use super::vm::Vm;

impl Vm<'_, '_> {
    /// Builds and installs a single method/accessor on the prototype (instance)
    /// or the constructor (static).
    pub(super) fn install_method(
        &mut self,
        method: &ClassMethodDef,
        key: PropertyKey,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        // A method's home object resolves `super.x`: instance methods and
        // accessors use the prototype; static members use the constructor.
        let home_object = if method.is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
        let class_upvalue = self.class_inner_upvalue(constructor_function, name);
        let method_function = Function::new_user_compiled(CompiledUserFunction {
            name: class_method_function_name(method, &key),
            has_name_binding: false,
            params: std::rc::Rc::new(method.params.clone()),
            creation: self.current_creation_context(),
            bytecode: method.bytecode.clone(),
            source_text: method.source_text.clone(),
            local_names: method.local_names.clone().into(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            lexical_call: None,
            is_generator: method.is_generator,
            is_async: method.is_async,
            is_class_constructor: false,
            is_derived_constructor: false,
            owner_kind: crate::context::OwnerKind::Function,
            home_object: Some(home_object.clone()),
            super_constructor: None,
            lexical_scope: self.prepare_scope_cursor(),
            upvalues: self.captured_upvalues_for_function_with_override(
                &method.bytecode,
                &method.lexical_captures,
                name.zip(class_upvalue.as_ref()),
            ),
        });
        if method.is_generator && method.is_async {
            crate::async_generator::wire_async_generator_function_intrinsics(
                &method_function,
                &self.context(),
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
        let mut prop_env = self.context();
        let success = object::define_property_on_value_key(target, key, descriptor, &mut prop_env)?;
        if !success {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot define class member".to_owned(),
            });
        }
        Ok(())
    }

    /// Builds the initializer thunk function for a field. The thunk runs with
    /// `this` bound at call time; its home object resolves `super.x` (instance
    /// fields use the prototype, static fields the constructor).
    pub(super) fn build_field_initializer(
        &mut self,
        field: &ClassFieldDef,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Option<Function> {
        let ClassFieldInitializerDef {
            local_names,
            lexical_captures,
            bytecode,
        } = field.initializer.as_ref()?;
        let home_object = if field.is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
        let class_upvalue = self.class_inner_upvalue(constructor_function, name);
        Some(Function::new_user_compiled(CompiledUserFunction {
            name: None,
            has_name_binding: false,
            params: std::rc::Rc::new(qjs_ast::FunctionParams::positional(Vec::new())),
            creation: self.current_creation_context(),
            bytecode: bytecode.clone(),
            source_text: None,
            local_names: local_names.clone().into(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            lexical_call: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            owner_kind: crate::context::OwnerKind::Field,
            home_object: Some(home_object),
            super_constructor: None,
            lexical_scope: self.prepare_scope_cursor(),
            upvalues: self.captured_upvalues_for_function_with_override(
                bytecode,
                lexical_captures,
                name.zip(class_upvalue.as_ref()),
            ),
        }))
    }

    /// Runs a `static { ... }` block at class definition: builds a parameterless
    /// strict thunk whose home object and `this` are the constructor (so
    /// `super.x` resolves against the constructor's [[Prototype]]) and calls it.
    pub(super) fn run_static_block(
        &mut self,
        block: &ClassStaticBlockDef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let ClassStaticBlockDef {
            local_names,
            lexical_captures,
            bytecode,
        } = block;
        let class_upvalue = self.class_inner_upvalue(constructor_function, name);
        let thunk = Function::new_user_compiled(CompiledUserFunction {
            name: None,
            has_name_binding: false,
            params: std::rc::Rc::new(qjs_ast::FunctionParams::positional(Vec::new())),
            creation: self.current_creation_context(),
            bytecode: bytecode.clone(),
            source_text: None,
            local_names: local_names.clone().into(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            lexical_call: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            owner_kind: crate::context::OwnerKind::StaticBlock,
            home_object: Some(Value::Function(constructor_function.clone())),
            super_constructor: None,
            lexical_scope: self.prepare_scope_cursor(),
            upvalues: self.captured_upvalues_for_function_with_override(
                bytecode,
                lexical_captures,
                name.zip(class_upvalue.as_ref()),
            ),
        });
        self.run_field_initializer(&thunk, Value::Function(constructor_function.clone()))?;
        Ok(())
    }

    /// Runs a field initializer thunk with the given `this` value and returns
    /// its result.
    pub(super) fn run_field_initializer(
        &mut self,
        thunk: &Function,
        this_value: Value,
    ) -> Result<Value, RuntimeError> {
        let mut env = self.context();

        call_field_initializer(thunk, this_value, &mut env)
    }

    /// Resolves `super.<key>` (or `super[key]`): the property is looked up on
    /// the current method's home object [[Prototype]] with the current `this`
    /// as the receiver, so inherited accessors run with the right `this`.
    pub(super) fn super_get(&mut self, key: &PropertyKey) -> Result<Value, RuntimeError> {
        let receiver = self.current_this()?;
        let lookup_base = self.super_lookup_base()?;
        self.super_get_from(lookup_base, receiver, key)
    }

    pub(super) fn super_get_from(
        &mut self,
        lookup_base: Value,
        receiver: Value,
        key: &PropertyKey,
    ) -> Result<Value, RuntimeError> {
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
        let mut env = self.context();
        let value = property_value_key_with_receiver(lookup_base, key, receiver, &mut env)?;
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
        self.super_set_value_from(lookup_base, receiver, key, value, is_strict)
    }

    pub(super) fn super_set_value_from(
        &mut self,
        lookup_base: Value,
        receiver: Value,
        key: PropertyKey,
        value: Value,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        if matches!(lookup_base, Value::Null | Value::Undefined) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property on null or undefined super base"
                    .to_owned(),
            });
        }
        let mut env = self.context();
        let wrote =
            crate::reflect::ordinary_set(lookup_base, &key, value.clone(), receiver, &mut env)?;
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
        let lookup_base = self.super_lookup_base()?;
        self.super_method_from(lookup_base, receiver, key)
    }

    pub(super) fn super_method_from(
        &mut self,
        lookup_base: Value,
        receiver: Value,
        key: PropertyKey,
    ) -> Result<(), RuntimeError> {
        let callee = self.super_get_from(lookup_base, receiver.clone(), &key)?;
        self.stack.push(receiver);
        self.stack.push(callee);
        Ok(())
    }

    pub(super) fn super_reference(&mut self) -> Result<(Value, Value), RuntimeError> {
        let receiver = self.current_this()?;
        let lookup_base = self.super_lookup_base()?;
        Ok((receiver, lookup_base))
    }

    /// Evaluates `super(...)` in a derived constructor: constructs the parent
    /// with the current `new.target`, binds the result as `this`, and pushes
    /// it. Calling `super(...)` after `this` is already bound is a
    /// ReferenceError.
    pub(super) fn super_call(&mut self, arguments: Vec<Value>) -> Result<(), RuntimeError> {
        let result = self.super_call_inner(arguments);
        if let Some(this_value) = self.handle_runtime_result(result)? {
            if let Some(cell) = self.env.derived_this_cell() {
                cell.initialize(this_value.clone())?;
            }
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
        let Some(Value::Function(constructor)) = self.env.active_constructor() else {
            return Ok(Value::Undefined);
        };
        let mut env = self.context();
        let result =
            crate::function::initialize_instance_fields(&constructor, this_value, &mut env);
        result.map(|()| Value::Undefined)
    }

    fn super_call_inner(&mut self, arguments: Vec<Value>) -> Result<Value, RuntimeError> {
        let super_constructor = self.current_super_constructor()?;
        let Some(super_constructor) = super_constructor else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        let new_target = self
            .env
            .lexical_new_target()
            .unwrap_or_else(|| super_constructor.clone());

        let mut env = self.context();
        let result = construct_function(super_constructor, new_target, arguments, &mut env);
        let value = result?;
        // A repeated `super(...)` still performs the parent construction after
        // argument evaluation, then fails while trying to initialize `this`.
        let this_is_bound = self.env.derived_this_cell().map_or_else(
            || self.env.owner_kind().is_some(),
            |cell| cell.is_initialized(),
        );
        if this_is_bound {
            return Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: super constructor may only be called once".to_owned(),
            });
        }
        Ok(value)
    }

    fn current_super_constructor(&mut self) -> Result<Option<Value>, RuntimeError> {
        if let Some(active_constructor) = self.env.active_constructor()
            && let Some(prototype) = value_prototype_slot(active_constructor, &self.context())
        {
            return Ok(Some(prototype.to_value()));
        }
        Ok(self.env.super_constructor())
    }

    fn current_this(&mut self) -> Result<Value, RuntimeError> {
        self.env.this_value()
    }

    /// Returns the lookup base for `super` property access: the [[Prototype]]
    /// of the current method's home object.
    fn super_lookup_base(&mut self) -> Result<Value, RuntimeError> {
        let Some(home) = self.env.home_object() else {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: 'super' keyword unexpected here".to_owned(),
            });
        };
        match value_prototype_slot(home, &self.context()) {
            Some(prototype) => Ok(prototype.to_value()),
            None => Ok(Value::Undefined),
        }
    }

    /// Wires a freshly created generator function into the generator intrinsic
    /// chain: its [[Prototype]] becomes `%GeneratorFunction.prototype%`, and its
    /// own `prototype` property's [[Prototype]] becomes `%GeneratorPrototype%`
    /// so generator instances inherit `next`/`return`/`throw`.
    pub(super) fn wire_generator_function_intrinsics(&mut self, function: &Function) {
        crate::generator::wire_generator_function_intrinsics(function, &self.context());
    }

    /// Wires a freshly created async function into the async intrinsic chain:
    /// its `[[Prototype]]` becomes `%AsyncFunction.prototype%`. Async functions
    /// are non-constructable and carry no own `prototype` property.
    pub(super) fn wire_async_function_intrinsics(&mut self, function: &Function) {
        crate::async_function::wire_async_function_intrinsics(function, &self.context());
    }
}

/// The resolved heritage of a class with an `extends` clause.
pub(super) enum ClassHeritage {
    /// `extends null`: the prototype object has a null [[Prototype]].
    Null,
    /// `extends <constructor>`: the parent constructor and its `prototype`.
    Parent(Box<ClassHeritageParent>),
}

pub(super) struct ClassHeritageParent {
    pub(super) constructor: Value,
    pub(super) prototype: Option<crate::Prototype>,
}

pub(super) fn function_prototype_value(env: &crate::context::Context<'_>) -> Option<Value> {
    env.realm().function_prototype().map(|p| p.to_value())
}

impl ClassHeritage {
    pub(super) fn resolve(
        value: Value,
        env: &mut crate::context::Context<'_>,
    ) -> Result<Self, RuntimeError> {
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
    env: &crate::context::Context<'_>,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    match value {
        Value::Null => Ok(None),
        Value::Object(prototype) if crate::symbol::is_symbol_primitive(&prototype) => {
            Err(class_heritage_prototype_error())
        }
        Value::Object(prototype) => Ok(Some(crate::Prototype::Object(prototype))),
        Value::Function(prototype) => Ok(Some(crate::Prototype::Function(prototype))),
        Value::Array(array) => Ok(Some(array_as_prototype_slot(&array, env))),
        _ => Err(class_heritage_prototype_error()),
    }
}

fn class_heritage_prototype_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: superclass prototype must be an object or null".to_owned(),
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
    env: &mut crate::context::Context<'_>,
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

/// Merges a new accessor descriptor with any existing accessor for the same
/// key on `target`, so a `get`/`set` pair declared separately combines into a
/// single accessor property.
fn merge_accessor(target: &Value, key: &PropertyKey, mut descriptor: Property) -> Property {
    if let Some(existing) = own_property_for_key(target, key)
        && existing.is_accessor()
    {
        descriptor.merge_missing_accessor_halves(existing);
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
