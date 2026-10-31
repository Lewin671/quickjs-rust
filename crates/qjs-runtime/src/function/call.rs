//! JS call/construct entry owns typed callee state and borrows RuntimeServices.
//! Parameters/rest seed positional slots before capture/mapped cells. Native
//! entries reborrow Context with a call site; they construct no activation.
//! Constructor targets are explicit and never overwrite caller lexical state.
//! Synchronous startup consumes borrowed inputs before a suspension is retained.

use std::{collections::HashMap, rc::Rc};

use qjs_ast::FunctionParams;

use crate::{
    Bytecode, Function, NativeFunction, ObjectRef, RuntimeError, Value,
    bytecode::{
        DirectCallSlots, DirectCallUpvalues, eval_call_bytecode, eval_direct_call_bytecode,
        eval_function_bytecode, try_eval_numeric_leaf, try_eval_this_property_leaf,
    },
    function_prototype,
    native::call_native_function,
    object_prototype, symbol,
};

use super::call_frame::{CallCompletionPolicy, PreparedBytecodeCall, missing_bytecode_body};
use super::{CROSS_REALM_TYPE_ERROR_PROTOTYPE, InstanceElementInitializer, function_call_this};

const CROSS_REALM_OBJECT_PROTOTYPE: &str = "__quickjsRustRealmObjectPrototype";
const CROSS_REALM_BOOLEAN_PROTOTYPE: &str = "__quickjsRustRealmBooleanPrototype";
const CROSS_REALM_NUMBER_PROTOTYPE: &str = "__quickjsRustRealmNumberPrototype";
const CROSS_REALM_STRING_PROTOTYPE: &str = "__quickjsRustRealmStringPrototype";
const CROSS_REALM_DATE_PROTOTYPE: &str = "__quickjsRustRealmDatePrototype";
const CROSS_REALM_REGEXP_PROTOTYPE: &str = "__quickjsRustRealmRegExpPrototype";
const CROSS_REALM_ERROR_PROTOTYPE: &str = "__quickjsRustRealmErrorPrototype";
const CROSS_REALM_EVAL_ERROR_PROTOTYPE: &str = "__quickjsRustRealmEvalErrorPrototype";
const CROSS_REALM_RANGE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmRangeErrorPrototype";
const CROSS_REALM_REFERENCE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmReferenceErrorPrototype";
const CROSS_REALM_SYNTAX_ERROR_PROTOTYPE: &str = "__quickjsRustRealmSyntaxErrorPrototype";
const CROSS_REALM_URI_ERROR_PROTOTYPE: &str = "__quickjsRustRealmURIErrorPrototype";
const CROSS_REALM_AGGREGATE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmAggregateErrorPrototype";
const CROSS_REALM_SUPPRESSED_ERROR_PROTOTYPE: &str = "__quickjsRustRealmSuppressedErrorPrototype";
const CROSS_REALM_ITERATOR_PROTOTYPE: &str = "__quickjsRustRealmIteratorPrototype";
const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

pub(crate) fn call_function(
    callee: Value,
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut crate::context::Context<'_>,
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    call_with_arguments(
        &callee,
        this_value,
        &argument_values,
        env,
        is_construct,
        is_construct.then_some(&callee),
    )
}

/// Common host/callback/native entry. Only semantic forwarding lists (bound
/// arguments and Proxy trap arrays) need ownership; ordinary calls borrow.
fn call_with_arguments(
    callee: &Value,
    this_value: Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
    is_construct: bool,
    new_target: Option<&Value>,
) -> Result<Value, RuntimeError> {
    if let Value::Proxy(proxy) = callee {
        if is_construct {
            return crate::proxy::proxy_construct(
                proxy.clone(),
                callee.clone(),
                argument_values.to_vec(),
                env,
            );
        }
        return crate::proxy::proxy_apply(proxy.clone(), this_value, argument_values.to_vec(), env);
    }
    let Value::Function(function) = callee else {
        return Err(RuntimeError {
            thrown: None,
            message: "value is not callable".to_owned(),
        });
    };
    if let Some(bound) = &function.bound {
        if !is_construct
            && matches!(
                &bound.target,
                Value::Function(target)
                    if target.native == Some(NativeFunction::FunctionPrototypeCall)
            )
        {
            let mut combined_arguments = bound.arguments.clone();
            combined_arguments.extend_from_slice(argument_values);
            let call_this = combined_arguments
                .first()
                .cloned()
                .unwrap_or(Value::Undefined);
            let call_arguments = &combined_arguments[usize::from(!combined_arguments.is_empty())..];
            return call_with_arguments(
                &bound.this_value,
                call_this,
                call_arguments,
                env,
                false,
                None,
            );
        }
        let mut bound_arguments = bound.arguments.clone();
        bound_arguments.extend_from_slice(argument_values);
        let bound_this = if is_construct {
            this_value
        } else {
            bound.this_value.clone()
        };
        return call_with_arguments(
            &bound.target,
            bound_this,
            &bound_arguments,
            env,
            is_construct,
            new_target,
        );
    }
    if function.is_class_constructor && !is_construct {
        return Err(class_constructor_call_error(function));
    }
    if let Some(entry) = function.native_entry {
        crate::diagnostics::count!(native_calls);
        return call_native_function(
            entry,
            crate::native::CallInfo {
                callee: function,
                this_value: &this_value,
                is_construct,
            },
            argument_values,
            new_target,
            None,
            env,
        );
    }
    if let Some(bytecode) = &function.bytecode {
        crate::diagnostics::count!(ordinary_call_attempts);
        if function.is_generator && function.is_async {
            let enter_function_activation = enter_function_activation(
                function,
                bytecode,
                callee,
                this_value,
                argument_values,
                env,
                new_target,
            );
            return crate::async_generator::call_async_generator_function(
                function,
                enter_function_activation.env,
                enter_function_activation.call_inputs,
                env,
            );
        }
        // Calling a generator function does not run its body, but it does run
        // the parameter prologue synchronously (per FunctionDeclarationInstantiation)
        // and then returns a generator object suspended at the start of the body.
        // A parameter-binding error therefore throws at the call, before the
        // generator object exists.
        if function.is_generator {
            let enter_function_activation = enter_function_activation(
                function,
                bytecode,
                callee,
                this_value,
                argument_values,
                env,
                new_target,
            );
            return crate::generator::make_generator_object(
                function,
                crate::bytecode::GeneratorStart {
                    bytecode: bytecode.clone(),
                    env: enter_function_activation.env,
                    upvalues: function.upvalues.clone(),
                    with_stack: Vec::new(),
                },
                enter_function_activation.call_inputs,
                env,
            );
        }
        // Calling an async function does not run its body to completion: it
        // captures the call frame, builds the promise it returns, and drives the
        // body until the first `await` or completion. The returned promise is
        // resolved/rejected with the body's eventual outcome (including
        // parameter-binding errors, which reject rather than throw).
        if function.is_async {
            let enter_function_activation = enter_function_activation(
                function,
                bytecode,
                callee,
                this_value,
                argument_values,
                env,
                new_target,
            );
            return Ok(crate::async_function::call_async_function(
                function,
                enter_function_activation.env,
                enter_function_activation.call_inputs,
                env,
            ));
        }
        // A base-class constructor initializes its instance fields right after
        // the receiver is created, before the constructor body runs. A derived
        // constructor defers this until `super(...)` binds `this`.
        if function.is_class_constructor && !function.is_derived_constructor && is_construct {
            initialize_instance_fields(function, &this_value, env)?;
        }
        // A native's callback -- `forEach`, `map`, `replace` -- calling an
        // ordinary function takes the path the interpreter's own calls take:
        // the compact tiers run it in the caller's environment, and only a
        // body they decline builds its slot-seeded frame. The general path
        // below built a compatibility frame for every such call, which was
        // most of a small callback's cost.
        if !is_construct && is_direct_leaf_function(callee) {
            return call_direct_leaf_function(callee.clone(), this_value, argument_values, env);
        }
        crate::diagnostics::count!(generic_call_frames);
        let prepared = prepare_bytecode_call(
            function,
            bytecode,
            callee,
            this_value,
            argument_values,
            env,
            new_target,
        );
        return run_prepared_bytecode_call(prepared, env.runtime);
    }

    Err(missing_bytecode_body())
}

/// Invokes a callback or host callee without copying its argument slice.
pub(crate) fn call_function_slice(
    callee: &Value,
    this_value: Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    // Preserve the base callback route: admitted JS bodies never enter the
    // general dispatcher's large generator/async/prepared-frame stack frame.
    if is_direct_leaf_function(callee) {
        return call_direct_leaf_function(callee.clone(), this_value, argument_values, env);
    }
    call_with_arguments(callee, this_value, argument_values, env, false, None)
}

/// Builds everything an ordinary synchronous call needs, without running it.
///
/// Every callable-kind decision -- proxy, bound, native, generator, async,
/// construction -- has been made by the time this runs. One question is left:
/// which environment and slots this body gets.
fn prepare_bytecode_call<'a>(
    function: &'a Function,
    bytecode: &'a Rc<Bytecode>,
    callee: &Value,
    this_value: Value,
    argument_values: &'a [Value],
    env: &mut crate::context::Context<'_>,
    new_target: Option<&Value>,
) -> PreparedBytecodeCall<'a> {
    let is_construct = new_target.is_some();
    let PreparedFunctionActivation {
        env: call_env,
        direct_call_slots,
        call_inputs,
    } = enter_function_activation(
        function,
        bytecode,
        callee,
        this_value,
        argument_values,
        env,
        new_target,
    );
    PreparedBytecodeCall {
        bytecode: Rc::clone(bytecode),
        env: call_env,
        slots: direct_call_slots,
        call_inputs,
        upvalues: function.upvalues.clone(),
        with_stack: Vec::new(),
        completion: CallCompletionPolicy::new(function.is_derived_constructor && is_construct),
    }
}

/// Runs a prepared call and settles what its caller is owed.
fn run_prepared_bytecode_call(
    prepared: PreparedBytecodeCall<'_>,
    runtime: &mut crate::context::RuntimeServices,
) -> Result<Value, RuntimeError> {
    let slot_seeded = prepared.is_slot_seeded();
    let PreparedBytecodeCall {
        bytecode,
        env,
        slots,
        call_inputs,
        upvalues,
        with_stack,
        completion,
    } = prepared;
    if slot_seeded {
        let slots = slots.expect("a slot-seeded call carries its slots");
        let result = eval_direct_call_bytecode(&bytecode, env, runtime, slots);
        return result;
    }
    if !completion.finishes_derived_construct() {
        let value = eval_call_bytecode(&bytecode, env, runtime, upvalues, with_stack, call_inputs);
        return value;
    }
    let result = eval_function_bytecode(
        &bytecode,
        env,
        runtime,
        upvalues,
        with_stack,
        true,
        call_inputs,
    );
    if completion.finishes_derived_construct() {
        return finish_derived_construct(result);
    }
    result.value
}

/// Runs an admitted ordinary callee with its own typed activation and borrowed
/// positional inputs. The tier ladder may consume that activation or return it
/// unchanged to the general VM; the caller's context is never substituted.
#[inline(never)]
pub(crate) fn call_direct_leaf_function(
    callee: Value,
    this_value: Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(ordinary_call_attempts);
    let Value::Function(function) = &callee else {
        unreachable!("direct leaf predicate only accepts functions");
    };
    let bytecode = function
        .bytecode
        .as_ref()
        .expect("direct leaf predicate requires bytecode");
    if let Some(value) = try_eval_numeric_leaf(
        bytecode,
        &function.params,
        argument_values,
        &function.upvalues,
    ) {
        let _identity = function.creation_realm().atoms().next_variable_identity();
        crate::diagnostics::count!(closed_form_leaf_evaluations);
        return Ok(value);
    }
    if let Some(value) = try_eval_this_property_leaf(bytecode, &this_value, argument_values) {
        let _identity = function.creation_realm().atoms().next_variable_identity();
        crate::diagnostics::count!(closed_form_leaf_evaluations);
        return Ok(value);
    }
    crate::diagnostics::count!(direct_leaf_frames);
    let PreparedFunctionActivation {
        env: call_env,
        direct_call_slots,
        ..
    } = enter_indexed_function_activation(
        function,
        bytecode,
        this_value,
        argument_values,
        env,
        None,
    );
    let direct_call_slots = direct_call_slots.expect("guarded direct leaf calls always seed slots");
    eval_direct_call_bytecode(bytecode, call_env, env.runtime, direct_call_slots)
}

/// Constructs an ordinary bytecode function through the same slot-seeded
/// direct-leaf frame an ordinary call uses, instead of the general
/// `construct_function` path that builds a name-keyed compatibility frame.
///
/// Declines, before any observable step, unless the callee is a plain
/// constructable non-class bytecode function admitted by the direct-leaf
/// predicate whose `prototype` is an own data property holding an object or
/// function. Accessor or non-object prototypes (which fall back to the realm's
/// `Object.prototype` or a cross-realm marker), class constructors (instance
/// field initializers and the derived-constructor TDZ), bound and native
/// constructors, and Proxies all keep the general path, which performs the
/// identical `prototype` read first, so a decline repeats no side effect.
pub(crate) fn try_construct_direct_leaf_function(
    callee: &Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Option<Result<Value, RuntimeError>> {
    let Value::Function(function) = callee else {
        return None;
    };
    if function.native.is_some() || function.bound.is_some() || !function.constructable {
        return None;
    }
    let class_fields = if function.is_class_constructor {
        if !is_direct_construct_class(function) {
            return None;
        }
        Some(function.instance_elements())
    } else {
        if !is_direct_leaf_function(callee) {
            return None;
        }
        None
    };
    let receiver = function.direct_instance_receiver(env.realm().atoms())?;
    let bytecode = function
        .bytecode
        .as_ref()
        .expect("direct leaf predicate requires bytecode");
    crate::diagnostics::count!(ordinary_call_attempts);
    crate::diagnostics::count!(direct_leaf_frames);
    // A base class initializes its instance fields before the body runs.
    if let Some(elements) = class_fields
        && let Err(error) = initialize_direct_instance_fields(&elements, &receiver, env)
    {
        return Some(Err(error));
    }
    let PreparedFunctionActivation {
        env: call_env,
        direct_call_slots,
        ..
    } = enter_indexed_function_activation(
        function,
        bytecode,
        Value::Object(receiver.clone()),
        argument_values,
        env,
        Some(callee.clone()),
    );
    let direct_call_slots = direct_call_slots.expect("guarded direct leaf calls always seed slots");
    let result = eval_direct_call_bytecode(bytecode, call_env, env.runtime, direct_call_slots);
    Some(match result {
        Ok(
            value @ (Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Object(_)
            | Value::Proxy(_)),
        ) => Ok(value),
        Ok(_) => Ok(Value::Object(receiver)),
        Err(error) => Err(error),
    })
}

/// Whether `new` on a class constructor may take the direct-leaf construct
/// path: a base class whose body the slot-seeding predicate admits and whose
/// instance elements are all public string-keyed fields with initializers
/// the direct-leaf call can run. Memoized on first construction, which is
/// after class definition recorded every element.
pub(crate) fn is_direct_construct_class(function: &Function) -> bool {
    if let Some(eligible) = function.direct_construct_eligible.get() {
        return eligible;
    }
    let eligible = !function.is_derived_constructor
        && function
            .bytecode
            .as_ref()
            .is_some_and(|bytecode| can_seed_base_class_constructor_slots(function, bytecode))
        && function
            .instance_elements()
            .iter()
            .all(|element| match element {
                InstanceElementInitializer::PublicField(field) => {
                    field.shared_string_key().is_some()
                        && field.initializer.as_ref().is_none_or(|thunk| {
                            thunk
                                .bytecode
                                .as_ref()
                                .is_some_and(|bytecode| bytecode.constant_return().is_some())
                                || is_direct_leaf_function(&Value::Function(thunk.clone()))
                        })
                }
                InstanceElementInitializer::PrivateElement(_) => false,
            });
    function.direct_construct_eligible.set(Some(eligible));
    eligible
}

/// Installs a base class's public string-keyed instance fields on a freshly
/// built receiver, in definition order, exactly as `initialize_instance_fields`
/// does: a literal initializer is read directly, any other runs as a
/// direct-leaf call with `this` = the receiver, and the value is defined as
/// an ordinary enumerable, writable, configurable data property. The receiver
/// is fresh, so it is never the realm's global object; a previous
/// initializer may still have made it non-extensible or defined the key, in
/// which case the general define path (and its TypeError) is used.
pub(crate) fn initialize_direct_instance_fields(
    elements: &[InstanceElementInitializer],
    receiver: &ObjectRef,
    env: &mut crate::context::Context<'_>,
) -> Result<(), RuntimeError> {
    let mut next = 0;
    let mut segments = ConstantFieldSegments::default();
    while next < elements.len() {
        let installed = segments.install(elements, next, receiver);
        if installed != 0 {
            next += installed;
            continue;
        }
        let element = &elements[next];
        next += 1;
        let InstanceElementInitializer::PublicField(field) = element else {
            unreachable!("direct construct admission accepts public fields only");
        };
        let value = match &field.initializer {
            None => Value::Undefined,
            Some(thunk) => {
                let bytecode = thunk
                    .bytecode
                    .as_ref()
                    .expect("direct construct admission requires thunk bytecode");
                match bytecode.constant_return() {
                    Some(value) => value.clone(),
                    None if let Some(result) =
                        crate::bytecode::field_initializer_member_read(thunk, env) =>
                    {
                        result?
                    }
                    None => call_direct_leaf_function(
                        Value::Function(thunk.clone()),
                        Value::Object(receiver.clone()),
                        &[],
                        env,
                    )?,
                }
            }
        };
        let key = field
            .shared_string_key()
            .expect("direct construct admission requires string keys");
        if !receiver.install_shared_class_field(key, &value, &field.definition_plan) {
            let define_env = &mut *env;
            crate::bytecode::install_field_value(
                &Value::Object(receiver.clone()),
                field.property_key(),
                value,
                define_env,
            )?;
        }
    }
    Ok(())
}

/// Executes a function literal whose allocation and identity were proven
/// unobservable by the bytecode flow analysis. The eligibility proof excludes
/// captures and every function-owned call-context feature, so this is the same
/// direct-slot entry used by an allocated ordinary leaf, minus the temporary
/// `Function` object and its metadata copies.
pub(crate) fn call_direct_function_literal(
    params: &FunctionParams,
    bytecode: &Bytecode,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(ordinary_call_attempts);
    if let Some(value) = try_eval_numeric_leaf(bytecode, params, argument_values, &[]) {
        let _identity = env.realm().atoms().next_variable_identity();
        crate::diagnostics::count!(closed_form_leaf_evaluations);
        return Ok(value);
    }
    crate::diagnostics::count!(direct_leaf_frames);
    let mut call_env = crate::context::ActivationContext::new(
        env.creation_context(),
        crate::context::ExecutionMode::Function,
    );
    call_env.strict = bytecode.is_strict();
    call_env.set_call_owner(crate::context::CallOwner::root(
        Value::Undefined,
        crate::context::OwnerKind::Function,
    ));
    let direct_call_slots = DirectCallSlots {
        this_value: None,
        parameter_slots: bytecode.parameter_slots(),
        arguments: argument_values,
        upvalues: DirectCallUpvalues::Slice(&[]),
        realm_upvalue_slots: 0,
    };
    eval_direct_call_bytecode(bytecode, call_env, env.runtime, direct_call_slots)
}

pub(crate) fn is_direct_leaf_function(callee: &Value) -> bool {
    let Value::Function(function) = callee else {
        return false;
    };
    if let Some(eligible) = function.direct_leaf_call_eligible.get() {
        return eligible;
    }
    let eligible = function
        .bytecode
        .as_ref()
        .is_some_and(|bytecode| can_seed_direct_leaf_call(function, bytecode));
    function.direct_leaf_call_eligible.set(Some(eligible));
    eligible
}

fn class_constructor_call_error(function: &Function) -> RuntimeError {
    let message = "TypeError: class constructor cannot be invoked without 'new'".to_owned();
    if let Some(crate::Property {
        value: Value::Object(prototype),
        ..
    }) = function.own_property(CROSS_REALM_TYPE_ERROR_PROTOTYPE)
    {
        let error =
            ObjectRef::with_prototype(HashMap::new(), Some(prototype.clone()), &prototype.domain());
        return RuntimeError {
            thrown: Some(Box::new(Value::Object(error))),
            message,
        };
    }
    RuntimeError {
        thrown: None,
        message,
    }
}

/// Runs a class constructor's instance-field initializers, in definition
/// order, installing each field on the receiver via CreateDataPropertyOrThrow.
/// Each initializer thunk evaluates with `this` = the receiver; a field with no
/// initializer installs `undefined`.
pub(crate) fn initialize_instance_fields(
    function: &Function,
    this_value: &Value,
    env: &mut crate::context::Context<'_>,
) -> Result<(), RuntimeError> {
    let elements = function.instance_elements();
    let mut next = 0;
    let mut segments = ConstantFieldSegments::default();
    while next < elements.len() {
        if let Value::Object(object) = this_value {
            let installed = segments.install(&elements, next, object);
            if installed != 0 {
                for element in &elements[next..next + installed] {
                    let (key, _, _) =
                        constant_public_field(element).expect("installed constant field");
                    env.sync_realm_global_object_property(object, key);
                }
                next += installed;
                continue;
            }
        }
        let element = &elements[next];
        next += 1;
        match element {
            InstanceElementInitializer::PublicField(field) => {
                let value = match &field.initializer {
                    Some(thunk) => call_field_initializer(thunk, this_value.clone(), env)?,
                    None => Value::Undefined,
                };
                if let Some(key) = field.shared_string_key()
                    && let Value::Object(object) = this_value
                    && is_ordinary_public_field_receiver(object)
                    && object.install_shared_class_field(key, &value, &field.definition_plan)
                {
                    env.sync_realm_global_object_property(object, key);
                } else {
                    crate::bytecode::install_field_value(
                        this_value,
                        field.property_key(),
                        value,
                        env,
                    )?;
                }
            }
            InstanceElementInitializer::PrivateElement(private) => {
                crate::bytecode::apply_instance_private_element(
                    function, this_value, private, env,
                )?;
            }
        }
    }
    Ok(())
}

/// Returns only a resolved public string key and a proven primitive constant.
/// The existing bytecode proof excludes code execution and per-instance identity.
type ConstantPublicField<'a> = (
    &'a Rc<str>,
    &'a Value,
    &'a std::cell::RefCell<crate::value::layout::FieldPlans>,
);
fn constant_public_field(element: &InstanceElementInitializer) -> Option<ConstantPublicField<'_>> {
    let InstanceElementInitializer::PublicField(field) = element else {
        return None;
    };
    let value = match &field.initializer {
        None => &Value::Undefined,
        Some(thunk) => thunk.bytecode.as_ref()?.constant_return()?,
    };
    if !matches!(
        value,
        Value::Undefined
            | Value::Null
            | Value::Boolean(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::BigInt(_)
    ) {
        return None;
    }
    Some((field.shared_string_key()?, value, &field.definition_plan))
}

/// Scans each observer-free segment once, including segments that decline.
/// Sequential definitions cannot make a declined segment worth re-probing:
/// they execute no code. Retry only at its boundary, after which an initializer
/// or private/symbol element may change the receiver's state.
#[derive(Default)]
struct ConstantFieldSegments {
    retry_at: usize,
    #[cfg(test)]
    inspected: usize,
}

impl ConstantFieldSegments {
    fn install(
        &mut self,
        elements: &[InstanceElementInitializer],
        next: usize,
        receiver: &ObjectRef,
    ) -> usize {
        if next < self.retry_at {
            return 0;
        }
        let count = elements[next..]
            .iter()
            .take_while(|element| {
                #[cfg(test)]
                {
                    self.inspected += 1;
                }
                constant_public_field(element).is_some()
            })
            .count();
        self.retry_at = next + count.max(1);
        if count < 2 {
            return 0;
        }
        let fields = elements[next..next + count]
            .iter()
            .map(|element| constant_public_field(element).expect("constant segment"));
        if receiver.install_constant_class_fields(fields) {
            count
        } else {
            0
        }
    }
}

/// Whether a string-keyed class field can reuse the existing ordinary-object
/// shared-key write. The generic DefineProperty route remains authoritative for
/// every receiver with custom string-key behavior.
fn is_ordinary_public_field_receiver(object: &ObjectRef) -> bool {
    !crate::typed_array::is_typed_array_object(object)
        && !object.is_module_namespace_exotic()
        && !object.is_array_prototype_exotic()
        && !object.is_symbol_object()
}

/// Evaluates a class field initializer with the receiver supplied by class
/// construction. Simple initializers have no observable compatibility
/// environment: their receiver and captures are already represented by VM
/// slots, and direct eval/closure/super cases are excluded by the leaf guard.
/// Run those thunks through the same allocation-light path as ordinary leaf
/// calls; semantic-heavy initializers retain the complete call machinery.
pub(crate) fn call_field_initializer(
    thunk: &Function,
    this_value: Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    // `x = 0` compiles to `return 0`; the constant is the whole effect.
    if let Some(value) = thunk
        .bytecode
        .as_ref()
        .and_then(|bytecode| bytecode.constant_return())
    {
        return Ok(value.clone());
    }
    // `color = Material.defaultColor` reads a captured binding's property.
    if let Some(result) = crate::bytecode::field_initializer_member_read(thunk, env) {
        return result;
    }
    let callee = Value::Function(thunk.clone());
    if is_direct_leaf_function(&callee) {
        return call_direct_leaf_function(callee, this_value, &[], env);
    }
    call_function(callee, this_value, Vec::new(), env, false)
}

fn finish_derived_construct(
    result: crate::bytecode::FunctionBytecodeResult,
) -> Result<Value, RuntimeError> {
    let bound_this = result.bound_this;
    let value = result.value?;
    match value {
        // A Symbol (or BigInt) primitive is represented as an object reference
        // but is not an Object, so it does not override `this`; it is a TypeError
        // like any other primitive return.
        Value::Object(ref object) if crate::symbol::is_symbol_primitive(object) => {
            Err(primitive_derived_return_error())
        }
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => Ok(value),
        Value::Undefined => match bound_this {
            Some(this_value) => Ok(this_value),
            None => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before returning \
                          from derived constructor"
                    .to_owned(),
            }),
        },
        // A primitive explicit return from a derived constructor is a
        // TypeError per the spec.
        _ => Err(primitive_derived_return_error()),
    }
}

fn primitive_derived_return_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: derived constructor may only return an object or undefined".to_owned(),
    }
}

pub(crate) fn construct_function(
    target: Value,
    new_target: Value,
    argument_values: Vec<Value>,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    construct_function_slice(target, new_target, &argument_values, env)
}

pub(crate) fn construct_function_slice(
    target: Value,
    new_target: Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    // A constructor Proxy dispatches through its `construct` trap. The trap (or
    // a forwarded target construction) produces the instance directly, so the
    // ordinary receiver-allocation path below is skipped.
    if let Value::Proxy(proxy) = &target {
        ensure_constructor(&new_target)?;
        return crate::proxy::proxy_construct(
            proxy.clone(),
            new_target,
            argument_values.to_vec(),
            env,
        );
    }
    // A bound function's [[Construct]] prepends the bound arguments and forwards
    // to its target. Per step 4, when the bound function is itself the
    // `new.target` (`new B()` where `B = A.bind()`), `new.target` becomes the
    // target so the constructed instance and `new.target` observe the target,
    // not the bound wrapper. Reflect.construct with an explicit new.target keeps
    // it. Unwrapping here (before the receiver is allocated from
    // `new.target.prototype`) also handles chained binds.
    if let Value::Function(function) = &target
        && let Some(bound) = &function.bound
    {
        let real_target = bound.target.clone();
        let mut combined_arguments = bound.arguments.clone();
        combined_arguments.extend_from_slice(argument_values);
        let forwarded_new_target = if target.same_value(&new_target) {
            real_target.clone()
        } else {
            new_target
        };
        return construct_function_slice(
            real_target,
            forwarded_new_target,
            &combined_arguments,
            env,
        );
    }
    if let Value::Function(function) = &target
        && function.native.is_some()
        && target.same_value(&new_target)
        && let Some(result) = super::construct_intrinsic_directly(function, argument_values, env)
    {
        return result;
    }
    ensure_constructor(&target)?;
    ensure_constructor(&new_target)?;

    // Make `new.target` visible to the constructor frame (and, via `super(...)`,
    // to ancestor constructors) so subclass instances get the right prototype.

    // A derived constructor must create its `this` through `super(...)`, so it
    // receives no pre-built receiver. Some native constructors also need to run
    // argument validation before the user-observable `new.target.prototype`
    // access that allocates the receiver.
    let is_derived =
        matches!(&target, Value::Function(function) if function.is_derived_constructor);
    let constructs_receiver_lazily = native_constructs_receiver_lazily(&target);

    (|| {
        let this_value = if is_derived || constructs_receiver_lazily {
            Value::Undefined
        } else {
            let prototype = construct_prototype_slot(&target, &new_target, env)?;
            Value::Object(match &new_target {
                Value::Function(function) => {
                    function.new_instance_receiver(prototype, env.realm().atoms())
                }
                _ => ObjectRef::with_prototype_slot(HashMap::new(), prototype, env.realm().atoms()),
            })
        };

        let result = call_with_arguments(
            &target,
            this_value.clone(),
            argument_values,
            env,
            true,
            Some(&new_target),
        );

        let result = result?;

        match result {
            Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Object(_)
            | Value::Proxy(_) => Ok(result),
            // A derived constructor that returns no object must have called
            // `super(...)`, which bound `this` and is returned as the result.
            _ if is_derived || constructs_receiver_lazily => Ok(result),
            _ => Ok(this_value),
        }
    })()
}

fn native_constructs_receiver_lazily(target: &Value) -> bool {
    matches!(
        target,
        Value::Function(function)
            if matches!(
                function.native,
                Some(
                    NativeFunction::Promise
                        | NativeFunction::ArrayBuffer
                        | NativeFunction::DataView
                        | NativeFunction::SharedArrayBuffer
                        | NativeFunction::Uint8Array
                        | NativeFunction::Int8Array
                        | NativeFunction::Uint8ClampedArray
                        | NativeFunction::Uint16Array
                        | NativeFunction::Int16Array
                        | NativeFunction::Uint32Array
                        | NativeFunction::Int32Array
                        | NativeFunction::Float32Array
                        | NativeFunction::Float64Array
                        | NativeFunction::BigInt64Array
                        | NativeFunction::BigUint64Array
                )
            )
    )
}

fn construct_prototype_slot(
    target: &Value,
    new_target: &Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    let cached = match new_target {
        Value::Function(function) => function.constructor_prototype_data(),
        _ => None,
    };
    let value = match cached {
        Some(value) => value,
        None => crate::property_value(new_target.clone(), "prototype", env)?,
    };
    let prototype = prototype_value_to_slot(value, env);
    if prototype.is_none()
        && let Some(prototype) = cross_realm_construct_prototype_slot(target, new_target, env)?
    {
        return Ok(Some(prototype));
    }
    Ok(prototype.or_else(|| default_construct_prototype_slot(target, env)))
}

fn cross_realm_construct_prototype_slot(
    target: &Value,
    new_target: &Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    let marker = match target {
        Value::Function(function) if function.native == Some(NativeFunction::Boolean) => {
            CROSS_REALM_BOOLEAN_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Number) => {
            CROSS_REALM_NUMBER_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::String) => {
            CROSS_REALM_STRING_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Date) => {
            CROSS_REALM_DATE_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Error) => {
            CROSS_REALM_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::EvalError) => {
            CROSS_REALM_EVAL_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::RangeError) => {
            CROSS_REALM_RANGE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::ReferenceError) => {
            CROSS_REALM_REFERENCE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::SyntaxError) => {
            CROSS_REALM_SYNTAX_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::TypeError) => {
            CROSS_REALM_TYPE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::UriError) => {
            CROSS_REALM_URI_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::AggregateError) => {
            CROSS_REALM_AGGREGATE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::SuppressedError) => {
            CROSS_REALM_SUPPRESSED_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::RegExp) => {
            CROSS_REALM_REGEXP_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Iterator) => {
            CROSS_REALM_ITERATOR_PROTOTYPE
        }
        Value::Function(_) => CROSS_REALM_OBJECT_PROTOTYPE,
        Value::Proxy(proxy) => {
            return cross_realm_construct_prototype_slot(&proxy.target_result()?, new_target, env);
        }
        _ => CROSS_REALM_OBJECT_PROTOTYPE,
    };
    marked_realm_prototype_slot(new_target, marker, env)
}

fn marked_realm_prototype_slot(
    new_target: &Value,
    marker: &str,
    env: &mut crate::context::Context<'_>,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    match new_target {
        Value::Function(function) => Ok(function
            .own_property(marker)
            .and_then(|property| prototype_value_to_slot(property.value, env))),
        Value::Proxy(proxy) => marked_realm_prototype_slot(&proxy.target_result()?, marker, env),
        _ => Ok(None),
    }
}

fn prototype_value_to_slot(
    value: Value,
    env: &mut crate::context::Context<'_>,
) -> Option<crate::Prototype> {
    match value {
        Value::Object(prototype) if !symbol::is_symbol_primitive(&prototype) => {
            Some(crate::Prototype::Object(prototype))
        }
        Value::Function(prototype) => Some(crate::Prototype::Function(prototype)),
        Value::Array(array) => Some(crate::array_as_prototype_slot(&array, env)),
        Value::Proxy(prototype) => Some(crate::Prototype::Proxy(prototype)),
        _ => None,
    }
}

fn default_construct_prototype_slot(
    target: &Value,
    env: &mut crate::context::Context<'_>,
) -> Option<crate::Prototype> {
    match target {
        Value::Function(function) if function.native.is_some() => {
            function_prototype(function).map(crate::Prototype::Object)
        }
        Value::Function(_) => object_prototype(env).map(crate::Prototype::Object),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| default_construct_prototype_slot(&target, env)),
        _ => object_prototype(env).map(crate::Prototype::Object),
    }
}

pub(crate) fn ensure_constructor(value: &Value) -> Result<(), RuntimeError> {
    match value {
        Value::Function(function) if function.constructable => Ok(()),
        Value::Proxy(proxy) if crate::proxy::proxy_is_constructor(proxy) => Ok(()),
        _ => Err(not_constructor_error()),
    }
}

fn not_constructor_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: value is not a constructor".to_owned(),
    }
}

struct PreparedFunctionActivation<'a> {
    env: crate::context::ActivationContext,
    direct_call_slots: Option<DirectCallSlots<'a>>,
    call_inputs: Option<crate::function::CallInputs<'a>>,
}

fn enter_indexed_function_activation<'a>(
    function: &'a Function,
    bytecode: &'a Bytecode,
    this_value: Value,
    argument_values: &'a [Value],
    env: &mut crate::context::Context<'_>,
    new_target: Option<Value>,
) -> PreparedFunctionActivation<'a> {
    // Ordinary leaf calls and direct base class constructions both reach
    // this frame; the class-constructor TypeError is the caller's concern.
    debug_assert!(can_seed_slot_backed_call(function, bytecode));
    let mut frame_env = crate::context::ActivationContext::new(
        selected_function_creation(function),
        crate::context::ExecutionMode::Function,
    );
    frame_env.strict = function.is_strict;
    let direct_this_value = bytecode.uses_lexical_this().then(|| {
        function_call_this(
            Some(this_value),
            &crate::context::Context {
                runtime: env.runtime,
                activation: &mut frame_env,
                native: None,
            },
            function.is_strict,
        )
    });
    if let Some(owner) = function.lexical_call() {
        frame_env.inherit_call_owner(owner);
    } else {
        let mut owner = crate::context::CallOwner::root(
            direct_this_value.clone().unwrap_or(Value::Undefined),
            function.owner_kind,
        );
        owner.new_target = new_target;
        frame_env.set_call_owner(owner);
    }
    PreparedFunctionActivation {
        env: frame_env,
        call_inputs: None,
        direct_call_slots: Some(DirectCallSlots {
            this_value: direct_this_value,
            parameter_slots: bytecode.parameter_slots(),
            arguments: argument_values,
            upvalues: DirectCallUpvalues::Function(function),
            realm_upvalue_slots: function.realm_upvalue_slots,
        }),
    }
}

fn enter_function_activation<'a>(
    function: &'a Function,
    bytecode: &'a Bytecode,
    callee: &Value,
    this_value: Value,
    argument_values: &'a [Value],
    env: &mut crate::context::Context<'_>,
    new_target: Option<&Value>,
) -> PreparedFunctionActivation<'a> {
    let is_construct = new_target.is_some();
    let use_direct_call_slots = (is_direct_leaf_function(callee)
        || (is_construct && can_seed_base_class_constructor_slots(function, bytecode)))
        && function.lexical_scope().is_none();
    let mut frame_env = crate::context::ActivationContext::new(
        selected_function_creation(function),
        crate::context::ExecutionMode::Function,
    );
    frame_env.strict = function.is_strict;
    frame_env.set_scope_cursor(function.lexical_scope());
    let mut direct_this_value = None;
    if let Some(owner) = function.lexical_call() {
        frame_env.inherit_call_owner(owner);
    } else {
        let needs_receiver = bytecode.needs_call_receiver() || is_construct;
        let receiver = if needs_receiver {
            function_call_this(
                Some(this_value),
                &crate::context::Context {
                    runtime: env.runtime,
                    activation: &mut frame_env,
                    native: None,
                },
                function.is_strict,
            )
        } else {
            Value::Undefined
        };
        if use_direct_call_slots {
            direct_this_value = Some(receiver.clone());
        }
        let derived = function.is_derived_constructor && is_construct;
        let this = if derived {
            crate::context::ThisBinding::Derived(super::BindingCell::new(Value::Function(
                Function::uninitialized_lexical_marker(),
            )))
        } else {
            crate::context::ThisBinding::Initialized(receiver)
        };
        frame_env.set_call_owner(crate::context::CallOwner {
            this,
            new_target: if is_construct {
                new_target.cloned()
            } else {
                None
            },
            home: function.home_object(),
            super_constructor: function.super_constructor(),
            active_constructor: derived.then(|| callee.clone()),
            kind: function.owner_kind,
        });
    }
    let direct_call_slots = use_direct_call_slots.then(|| DirectCallSlots {
        this_value: direct_this_value,
        parameter_slots: bytecode.parameter_slots(),
        arguments: argument_values,
        upvalues: DirectCallUpvalues::Function(function),
        realm_upvalue_slots: function.realm_upvalue_slots,
    });
    PreparedFunctionActivation {
        env: frame_env,
        direct_call_slots,
        call_inputs: (!use_direct_call_slots).then_some(super::CallInputs {
            function,
            arguments: argument_values,
        }),
    }
}

fn can_seed_direct_leaf_call(function: &Function, bytecode: &Bytecode) -> bool {
    if function.lexical_scope().is_some() {
        return false;
    }
    // This predicate feeds the ordinary direct-call dispatcher, which must
    // retain the TypeError for `ClassConstructor()`. Base class constructors
    // can separately reuse these slots only after `enter_function_activation` has proved
    // this is a construction call.
    !function.is_class_constructor && can_seed_slot_backed_call(function, bytecode)
}

fn can_seed_base_class_constructor_slots(function: &Function, bytecode: &Bytecode) -> bool {
    function.is_class_constructor
        && !function.is_derived_constructor
        && can_seed_slot_backed_call(function, bytecode)
}

fn can_seed_slot_backed_call(function: &Function, bytecode: &Bytecode) -> bool {
    // Ordinary constructors use the same slot-backed parameter and receiver
    // model as ordinary calls. Constructor-only state such as `new.target`
    // remains in the small compatibility frame installed by enter_function_activation.
    // An arrow inherits `this`, `arguments`, and `new.target` from the function
    // that created it, none of which a fresh frame provides. One that reads none
    // of them is indistinguishable from an ordinary function here, and arrow
    // callbacks that only use their parameters are the common case.
    (!function.lexical_this || (!bytecode.uses_lexical_this() && !bytecode.reads_new_target()))
        && (!function.lexical_arguments || !bytecode.reads_arguments())
        && !function.is_generator
        && !function.is_async
        && !function.has_name_binding
        && bytecode.has_direct_seedable_parameters()
        && !bytecode.needs_arguments_object()
        && !bytecode.has_dynamic_scope_tree()
        && !bytecode.contains_direct_eval()
        && !bytecode.contains_with()
        && !bytecode.contains_super_operation()
        && !bytecode.creates_capturing_closures()
}

/// A function owns its immutable creation metadata. An explicit host override
/// selects a live object, never a copied namespace. The default entry only
/// retains the existing creation handle.
fn selected_function_creation(function: &Function) -> Rc<crate::context::CreationContext> {
    let mut creation = function.creation_context().clone();
    if function.has_dynamic_function_realm_override.get() {
        let value = function
            .own_property(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .map_or(Value::Undefined, |property| property.value);
        let selected = Rc::make_mut(&mut creation);
        selected.intrinsics = match &value {
            Value::Object(global) => crate::context::IntrinsicOverrides::for_marked_global(global),
            _ => Default::default(),
        };
        selected.global = crate::context::GlobalSelection::from_control(value);
    } else if function.has_dynamic_function_realm
        && matches!(creation.global, crate::context::GlobalSelection::Default)
        && let Some(global) = creation.realm.dynamic_function_realm_global()
    {
        let selected = Rc::make_mut(&mut creation);
        selected.intrinsics = crate::context::IntrinsicOverrides::for_marked_global(&global);
        selected.global = crate::context::GlobalSelection::Override(global);
    }
    creation
}

#[cfg(test)]
mod tests {
    use super::{
        ConstantFieldSegments, can_seed_base_class_constructor_slots, can_seed_direct_leaf_call,
        constant_public_field,
    };
    use crate::{ObjectRef, Property, Value, eval};

    #[test]
    fn declined_long_constant_segment_is_scanned_once() {
        let domain = crate::property::atom::AtomDomain::new();
        let count = 512;
        let fields = (0..count).map(|i| format!("p{i}={i};")).collect::<String>();
        let Value::Function(class) = eval(&format!("(class {{ {fields} }})")).unwrap() else {
            panic!("expected class constructor");
        };
        let elements = class.instance_elements();
        let receiver = ObjectRef::new(std::collections::HashMap::new(), &domain);
        // An unrelated non-configurable descriptor declines before the batch
        // consumes any field. All sequential field definitions still succeed.
        receiver.define_property(
            "fixed".into(),
            Property::data(Value::Null, false, false, false),
        );
        let mut segments = ConstantFieldSegments::default();
        for next in 0..elements.len() {
            assert_eq!(segments.install(&elements, next, &receiver), 0);
            let (key, value, _) = constant_public_field(&elements[next]).unwrap();
            receiver.set_shared_key(key.clone(), value.clone());
        }
        assert_eq!(segments.inspected, count);
        assert_eq!(receiver.get("p511"), Some(Value::Number(511.0)));
    }

    #[test]
    fn base_class_constructor_slot_eligibility_is_construct_only() {
        let Value::Function(function) = eval(
            "(class { #value; constructor(value = 1) { this.#value = value; this.target = new.target; } })",
        )
        .expect("class expression should evaluate")
        else {
            panic!("expected class constructor function");
        };
        let bytecode = function
            .bytecode
            .as_ref()
            .expect("class constructor should have bytecode");

        assert!(can_seed_base_class_constructor_slots(&function, bytecode));
        assert!(!can_seed_direct_leaf_call(&function, bytecode));
    }

    #[test]
    fn derived_class_constructor_stays_out_of_slot_seeded_construction() {
        let Value::Function(function) = eval(
            "(class extends class {} { constructor(value) { super(); this.value = value; } })",
        )
        .expect("derived class expression should evaluate") else {
            panic!("expected derived class constructor function");
        };
        let bytecode = function
            .bytecode
            .as_ref()
            .expect("derived class constructor should have bytecode");

        assert!(!can_seed_base_class_constructor_slots(&function, bytecode));
        assert!(!can_seed_direct_leaf_call(&function, bytecode));
    }
}

#[cfg(test)]
#[path = "call_activation_tests.rs"]
mod activation_tests;

#[cfg(test)]
#[path = "call_context_tests.rs"]
mod context_tests;

pub(crate) fn enter_indexed_activation(function: &Function) -> crate::context::ActivationContext {
    let mut activation = crate::context::ActivationContext::new(
        selected_function_creation(function),
        crate::context::ExecutionMode::Function,
    );
    activation.strict = function.is_strict;
    activation.set_call_owner(crate::context::CallOwner::root(
        Value::Undefined,
        function.owner_kind,
    ));
    activation
}
