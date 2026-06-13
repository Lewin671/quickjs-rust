//! VM support for private class names: installing private elements when a class
//! is built, and the `GetPrivate`/`SetPrivate`/`PrivateIn` operations.

use std::{cell::RefCell, rc::Rc};

use crate::CallEnv;

use crate::{
    Function, ObjectRef, RuntimeError, Value, call_function,
    function::{CompiledUserFunction, InstancePrivateElement, PrivateFieldInit},
    private::{PrivateBinding, PrivateEnvironment, PrivateKind, PrivateName, PrivateStorage},
};

use super::ir::{ClassMethodDef, ClassMethodKind, ClassPrivateElementDef};
use super::vm::Vm;
use super::vm_class::class_method_function_name_with_base;

impl Vm<'_> {
    /// Installs the private elements of a class. Methods and accessors become
    /// shared function values registered in the private environment; instances
    /// are branded with them at construction. Private fields register an
    /// instance initializer (or, when static, install immediately on the
    /// constructor). The private environment is attached to the prototype
    /// (instance home object) and the constructor (static home object) so
    /// member bodies resolve `#x` references through their home object.
    pub(super) fn install_private_elements(
        &mut self,
        private_elements: &[ClassPrivateElementDef],
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let Some(environment) = prototype.private_environment() else {
            return Ok(());
        };

        for element in private_elements {
            match element {
                ClassPrivateElementDef::Field {
                    name: field_name,
                    is_static,
                    initializer,
                } => {
                    let id = environment.declare_field(field_name);
                    let thunk = initializer.as_ref().map(|definition| {
                        self.build_private_field_thunk(
                            definition,
                            *is_static,
                            prototype,
                            constructor_function,
                            name,
                        )
                    });
                    if *is_static {
                        let value = match &thunk {
                            Some(thunk) => self.run_private_field_initializer(
                                thunk,
                                Value::Function(constructor_function.clone()),
                            )?,
                            None => Value::Undefined,
                        };
                        constructor_function.private_storage().add_field(id, value);
                    } else {
                        constructor_function.push_instance_private_element(
                            InstancePrivateElement {
                                id,
                                field_initializer: Some(PrivateFieldInit { initializer: thunk }),
                            },
                        );
                    }
                }
                ClassPrivateElementDef::Method {
                    name: method_name,
                    is_static,
                    def,
                } => {
                    let function = self.build_private_method(
                        def,
                        *is_static,
                        prototype,
                        constructor_function,
                        name,
                    );
                    let id = environment.declare_method(method_name, function);
                    self.brand_private_member(id, *is_static, constructor_function);
                }
                ClassPrivateElementDef::Getter {
                    name: accessor_name,
                    is_static,
                    def,
                }
                | ClassPrivateElementDef::Setter {
                    name: accessor_name,
                    is_static,
                    def,
                } => {
                    let function = self.build_private_method(
                        def,
                        *is_static,
                        prototype,
                        constructor_function,
                        name,
                    );
                    let (get, set) = match def.method_kind {
                        ClassMethodKind::Getter => (Some(function), None),
                        _ => (None, Some(function)),
                    };
                    let id = environment.declare_accessor(accessor_name, get, set);
                    self.brand_private_member(id, *is_static, constructor_function);
                }
            }
        }
        Ok(())
    }

    /// Creates and attaches a class private environment before computed keys are
    /// evaluated, then predeclares every private name in the class body.
    pub(super) fn create_private_environment(
        &mut self,
        private_elements: &[ClassPrivateElementDef],
        prototype: &ObjectRef,
        constructor_function: &Function,
    ) {
        let enclosing = self.current_private_environment();
        if private_elements.is_empty() && enclosing.is_none() {
            return;
        }
        let environment = PrivateEnvironment::with_outer(enclosing);
        prototype.set_private_environment(environment.clone());
        constructor_function.set_private_environment(environment.clone());
        for element in private_elements {
            match element {
                ClassPrivateElementDef::Field { name, .. }
                | ClassPrivateElementDef::Method { name, .. }
                | ClassPrivateElementDef::Getter { name, .. }
                | ClassPrivateElementDef::Setter { name, .. } => {
                    environment.declare_placeholder(name);
                }
            }
        }
    }

    /// Brands a private method/accessor onto the constructor (static) or
    /// registers it as an instance brand applied at construction.
    fn brand_private_member(
        &self,
        id: PrivateName,
        is_static: bool,
        constructor_function: &Function,
    ) {
        if is_static {
            constructor_function.private_storage().add_brand(id);
        } else {
            constructor_function.push_instance_private_element(InstancePrivateElement {
                id,
                field_initializer: None,
            });
        }
    }

    /// Builds the shared function object for a private method or accessor. Its
    /// home object resolves `super.x` (instance: prototype; static:
    /// constructor) and carries the private environment.
    fn build_private_method(
        &self,
        def: &ClassMethodDef,
        is_static: bool,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Value {
        let mut method_env = self.function_capture_env(&def.bytecode, &def.local_names);
        bind_inner_name(&mut method_env, name, constructor_function);
        let home_object = if is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
        let function = Function::new_user_compiled(CompiledUserFunction {
            name: class_method_function_name_with_base(def.method_kind, def.name.clone()),
            params: def.params.clone(),
            env: method_env.clone(),
            bytecode: def.bytecode.clone(),
            local_names: def.local_names.clone(),
            constructable: false,
            is_strict: true,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: def.is_generator,
            is_async: def.is_async,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            home_object: Some(home_object),
            super_constructor: None,
            captured_env: Rc::new(RefCell::new(method_env)),
            capture_writeback: self.capture_writeback.clone(),
        });
        if def.is_generator && def.is_async {
            crate::async_generator::wire_async_generator_function_intrinsics(&function, &self.env);
        } else if def.is_generator {
            self.wire_generator_function_intrinsics(&function);
        } else if def.is_async {
            self.wire_async_function_intrinsics(&function);
        }
        Value::Function(function)
    }

    /// Builds the initializer thunk for a private field, mirroring public-field
    /// thunks: parameterless, strict, with the right home object.
    fn build_private_field_thunk(
        &self,
        definition: &super::ir::ClassFieldInitializerDef,
        is_static: bool,
        prototype: &ObjectRef,
        constructor_function: &Function,
        name: Option<&str>,
    ) -> Function {
        let mut field_env =
            self.function_capture_env(&definition.bytecode, &definition.local_names);
        bind_inner_name(&mut field_env, name, constructor_function);
        let home_object = if is_static {
            Value::Function(constructor_function.clone())
        } else {
            Value::Object(prototype.clone())
        };
        Function::new_user_compiled(CompiledUserFunction {
            name: None,
            params: qjs_ast::FunctionParams::positional(Vec::new()),
            env: field_env.clone(),
            bytecode: definition.bytecode.clone(),
            local_names: definition.local_names.clone(),
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
            capture_writeback: self.capture_writeback.clone(),
        })
    }

    fn run_private_field_initializer(
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

    /// Reads `obj.#name`: resolves the binding through the current home object's
    /// private environment, checks the brand, and returns the field value, the
    /// shared method, or the getter result.
    pub(super) fn get_private(&mut self, name: &str) -> Result<Value, RuntimeError> {
        let object = self.pop()?;
        let binding = self.resolve_private_binding(name)?;
        let storage = private_storage_of(&object).filter(|storage| storage.has(&binding.id));
        let Some(storage) = storage else {
            return Err(foreign_private_error(name));
        };
        match &binding.kind {
            PrivateKind::Field => Ok(storage.get_field(&binding.id).unwrap_or(Value::Undefined)),
            PrivateKind::Method(function) => Ok((**function).clone()),
            PrivateKind::Accessor(accessor) => match &accessor.get {
                Some(getter) => self.call_private_accessor(getter.clone(), object, None),
                None => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "TypeError: private member #{name} was defined without a getter"
                    ),
                }),
            },
        }
    }

    /// Writes `obj.#name = value`: resolves the binding, checks the brand, and
    /// either stores the field or runs the setter.
    pub(super) fn set_private(&mut self, name: &str) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        let object = self.pop()?;
        let binding = self.resolve_private_binding(name)?;
        let storage = private_storage_of(&object).filter(|storage| storage.has(&binding.id));
        let Some(storage) = storage else {
            return Err(foreign_private_error(name));
        };
        match &binding.kind {
            PrivateKind::Field => {
                storage.set_field(&binding.id, value.clone());
                Ok(value)
            }
            PrivateKind::Method(_) => Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: private method #{name} is not writable"),
            }),
            PrivateKind::Accessor(accessor) => match &accessor.set {
                Some(setter) => {
                    self.call_private_accessor(setter.clone(), object, Some(value.clone()))?;
                    Ok(value)
                }
                None => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "TypeError: private member #{name} was defined without a setter"
                    ),
                }),
            },
        }
    }

    /// Evaluates `#name in obj`: a brand/slot presence check that never throws.
    pub(super) fn private_in(&mut self, name: &str) -> Result<Value, RuntimeError> {
        let object = self.pop()?;
        let binding = self.resolve_private_binding(name)?;
        let present = private_storage_of(&object).is_some_and(|storage| storage.has(&binding.id));
        Ok(Value::Boolean(present))
    }

    fn call_private_accessor(
        &mut self,
        accessor: Value,
        this_value: Value,
        argument: Option<Value>,
    ) -> Result<Value, RuntimeError> {
        let arguments = argument.into_iter().collect();
        let mut env = self.current_env();
        let result = call_function(accessor, this_value, arguments, &mut env, false);
        self.apply_env(env);
        result
    }

    /// Resolves a private name against the private environment of the current
    /// home object. A private name reference is only valid lexically inside the
    /// class that declares it, so the home object always carries it.
    fn resolve_private_binding(&self, name: &str) -> Result<PrivateBinding, RuntimeError> {
        let environment = self
            .current_private_environment()
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: format!("SyntaxError: private name #{name} used outside a class body"),
            })?;
        environment.resolve(name).ok_or_else(|| RuntimeError {
            thrown: None,
            message: format!("SyntaxError: private name #{name} is not declared in scope"),
        })
    }

    /// Returns the private environment carried by the current home object.
    pub(super) fn current_private_environment(&self) -> Option<PrivateEnvironment> {
        if let Some(environment) = self.env.private_environment() {
            return Some(environment);
        }
        match self.env.get(crate::HOME_OBJECT_BINDING) {
            Some(Value::Object(object)) => object.private_environment(),
            Some(Value::Function(function)) => function.private_environment(),
            _ => None,
        }
    }

    pub(super) fn capture_private_environment(&self, function: &Function) {
        if let Some(environment) = self.current_private_environment() {
            function.set_private_environment(environment);
        }
    }
}

/// Applies a constructor's instance private elements to a freshly created
/// instance: branding it with each private name and installing field values.
pub(crate) fn apply_instance_private_elements(
    constructor: &Function,
    this_value: &Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let elements = constructor.instance_private_elements();
    if elements.is_empty() {
        return Ok(());
    }
    let storage = match instance_private_storage(this_value) {
        Some(storage) => storage,
        None => return Ok(()),
    };
    for element in elements {
        match &element.field_initializer {
            Some(field) => {
                let value = match &field.initializer {
                    Some(thunk) => call_function(
                        Value::Function(thunk.clone()),
                        this_value.clone(),
                        Vec::new(),
                        env,
                        false,
                    )?,
                    None => Value::Undefined,
                };
                if !storage.add_field(element.id.clone(), value) {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "TypeError: private field #{} is already present on the object",
                            element.id.description()
                        ),
                    });
                }
            }
            None => storage.add_brand(element.id.clone()),
        }
    }
    Ok(())
}

fn instance_private_storage(value: &Value) -> Option<PrivateStorage> {
    match value {
        Value::Object(object) => Some(object.private_storage()),
        Value::Function(function) => Some(function.private_storage()),
        Value::Proxy(proxy) => private_storage_of(&proxy.target()),
        _ => None,
    }
}

fn private_storage_of(value: &Value) -> Option<PrivateStorage> {
    instance_private_storage(value)
}

fn foreign_private_error(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!(
            "TypeError: Cannot read private member #{name} from an object whose class did not \
             declare it"
        ),
    }
}

fn bind_inner_name(
    env: &mut std::collections::HashMap<String, Value>,
    name: Option<&str>,
    constructor: &Function,
) {
    if let Some(name) = name {
        env.insert(name.to_owned(), Value::Function(constructor.clone()));
    }
}
