//! Generator object intrinsics: `%GeneratorPrototype%` and the
//! `next`/`return`/`throw` protocol methods. The suspend/resume state machine
//! lives in `bytecode::vm_generator`; this module wires the prototype, builds
//! generator objects when a `function*` is called, and adapts the resume
//! outcomes into iterator-result objects.

use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    bytecode::{GeneratorOutcome, GeneratorStart, Resume, resume_generator},
    function_intrinsic_prototype, object_prototype, symbol,
};

/// Intrinsic binding for `%GeneratorPrototype%`, propagated into call frames so
/// a `function*` can reach it when building its generator objects.
pub(crate) const GENERATOR_PROTOTYPE_BINDING: &str = "\0GeneratorPrototype";

/// Intrinsic binding for `%GeneratorFunction.prototype%`, the object that sits
/// between a generator function and `%Function.prototype%` in the prototype
/// chain.
pub(crate) const GENERATOR_FUNCTION_PROTOTYPE_BINDING: &str = "\0GeneratorFunctionPrototype";

/// Installs `%GeneratorPrototype%` (with `next`/`return`/`throw`,
/// `Symbol.iterator`, and the `Generator` toStringTag) and the
/// `%GeneratorFunction.prototype%` object, recording both under intrinsic
/// bindings so generator function calls can wire generator objects.
pub(crate) fn install_generator(
    env: &mut CallEnv,
    _global_this: &Value,
    object_prototype: ObjectRef,
) {
    // %GeneratorPrototype% inherits %Iterator.prototype% (27.5.1), which already
    // carries `Symbol.iterator` returning `this` plus the iterator helpers, so
    // generators get those methods. Fall back to the ordinary object prototype
    // only if the iterator intrinsic is somehow unavailable.
    let generator_parent =
        crate::iterator::iterator_prototype(env).unwrap_or(object_prototype.clone());
    let generator_prototype = ObjectRef::with_prototype(HashMap::new(), Some(generator_parent));

    for (name, native) in [
        ("next", NativeFunction::GeneratorPrototypeNext),
        ("return", NativeFunction::GeneratorPrototypeReturn),
        ("throw", NativeFunction::GeneratorPrototypeThrow),
    ] {
        generator_prototype.define_non_enumerable(
            name.to_owned(),
            Value::Function(Function::new_native(Some(name), 1, native, false)),
        );
    }

    // `Symbol.iterator` on %GeneratorPrototype% returns the generator itself.
    if let Some(iterator) = symbol::iterator_symbol(env) {
        generator_prototype.define_symbol_property(
            iterator,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.iterator]"),
                0,
                NativeFunction::IteratorPrototypeIterator,
                false,
            ))),
        );
    }
    generator_prototype.set_to_string_tag("Generator");
    symbol::define_well_known_to_string_tag(env, &generator_prototype, "Generator");

    // `%GeneratorFunction.prototype%` is the [[Prototype]] every generator
    // function points at. Its own [[Prototype]] is `%Function.prototype%`, it
    // exposes `%GeneratorPrototype%` as its `prototype`, and carries the
    // "GeneratorFunction" toStringTag.
    let generator_function_prototype = ObjectRef::with_prototype(
        HashMap::new(),
        function_intrinsic_prototype(env).or(Some(object_prototype)),
    );
    let generator_function = Function::new_native(
        Some("GeneratorFunction"),
        1,
        NativeFunction::GeneratorFunction,
        true,
    );
    let _ = generator_function.set_internal_prototype_slot(
        function_intrinsic_prototype(env).map(crate::Prototype::Object),
    );
    generator_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(
            Value::Object(generator_function_prototype.clone()),
            false,
            false,
            false,
        ),
    );
    generator_function_prototype.define_property(
        "prototype".to_owned(),
        Property::data(
            Value::Object(generator_prototype.clone()),
            false,
            false,
            true,
        ),
    );
    generator_function_prototype.set_to_string_tag("GeneratorFunction");
    symbol::define_well_known_to_string_tag(
        env,
        &generator_function_prototype,
        "GeneratorFunction",
    );
    generator_function_prototype.define_property(
        "constructor".to_owned(),
        Property::data(
            Value::Function(generator_function.clone()),
            false,
            false,
            true,
        ),
    );
    // %GeneratorPrototype%.constructor is %GeneratorFunction.prototype%.
    generator_prototype.define_property(
        "constructor".to_owned(),
        Property::data(
            Value::Object(generator_function_prototype.clone()),
            false,
            false,
            true,
        ),
    );

    env.insert_realm(
        GENERATOR_PROTOTYPE_BINDING.to_owned(),
        Value::Object(generator_prototype),
    );
    env.insert_realm(
        GENERATOR_FUNCTION_PROTOTYPE_BINDING.to_owned(),
        Value::Object(generator_function_prototype),
    );
}

/// Returns `%GeneratorFunction.prototype%` from the current environment.
pub(crate) fn generator_function_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(GENERATOR_FUNCTION_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Returns `%GeneratorPrototype%` from the current environment.
fn generator_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(GENERATOR_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Returns `%GeneratorPrototype%` from the current environment (public alias for
/// intrinsic wiring at function-creation time).
pub(crate) fn generator_prototype_intrinsic(env: &CallEnv) -> Option<ObjectRef> {
    generator_prototype(env)
}

/// Wires a freshly created generator function into the generator intrinsic
/// chain: its [[Prototype]] becomes `%GeneratorFunction.prototype%`, and its
/// own `prototype` property's [[Prototype]] becomes `%GeneratorPrototype%`.
pub(crate) fn wire_generator_function_intrinsics(function: &Function, env: &CallEnv) {
    if let Some(generator_function_prototype) = generator_function_prototype(env) {
        let _ = function.set_internal_prototype_slot(Some(crate::Prototype::Object(
            generator_function_prototype,
        )));
    }
    if let Some(generator_prototype) = generator_prototype_intrinsic(env) {
        let prototype = ObjectRef::with_prototype(HashMap::new(), Some(generator_prototype));
        function.define_property(
            "prototype".to_owned(),
            Property::data(Value::Object(prototype), false, true, false),
        );
    }
}

/// Builds the generator object returned by calling a `function*`: an ordinary
/// object whose [[Prototype]] is the function's own `prototype` (when an
/// object) or `%GeneratorPrototype%`. The parameter prologue runs synchronously
/// here (per `FunctionDeclarationInstantiation`), so a binding error throws at
/// the call before the object exists; the object then carries the body-start
/// state for the first resume.
pub(crate) fn make_generator_object(
    function: &Function,
    start: GeneratorStart,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = crate::bytecode::start_suspended_at_body(start, env)?;
    let prototype = generator_object_prototype(function, env);
    let generator = ObjectRef::with_prototype(HashMap::new(), prototype);
    *generator.generator_state().borrow_mut() = Some(state);
    Ok(Value::Object(generator))
}

/// Resolves the [[Prototype]] for a generator object: the function's own
/// `prototype` property when it is an object, otherwise `%GeneratorPrototype%`.
fn generator_object_prototype(function: &Function, env: &CallEnv) -> Option<ObjectRef> {
    if let Some(Value::Object(prototype)) = function
        .own_property("prototype")
        .map(|property| property.value)
    {
        return Some(prototype);
    }
    generator_prototype(env)
}

/// Dispatches the `%GeneratorPrototype%` `next`/`return`/`throw` natives.
pub(crate) fn call_generator_native(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let resume = match native {
        NativeFunction::GeneratorPrototypeNext => {
            Resume::Next(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        NativeFunction::GeneratorPrototypeReturn => {
            Resume::Return(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        NativeFunction::GeneratorPrototypeThrow => {
            Resume::Throw(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        _ => return Ok(None),
    };

    let Value::Object(generator) = &this_value else {
        return Err(not_a_generator());
    };
    if generator.generator_state().borrow().is_none() {
        return Err(not_a_generator());
    }

    let outcome = resume_generator(generator, resume, env)?;
    let result = match outcome {
        GeneratorOutcome::Yield(value) => iterator_result(value, false, env),
        // `yield*` hands back the inner iterator's result object unchanged
        // rather than rebuilding it.
        GeneratorOutcome::YieldDelegate(value) => value,
        // A plain generator body emits no `Op::Await`, so an await suspension
        // never reaches here; treat the awaited value as a plain yield rather
        // than panicking on a malformed body.
        GeneratorOutcome::Await(value) => iterator_result(value, false, env),
        GeneratorOutcome::Return(value) => iterator_result(value, true, env),
    };
    Ok(Some(result))
}

fn iterator_result(value: Value, done: bool, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    object.define_property("value".to_owned(), Property::enumerable(value));
    object.define_property(
        "done".to_owned(),
        Property::enumerable(Value::Boolean(done)),
    );
    Value::Object(object)
}

fn not_a_generator() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: generator method called on a non-generator object".to_owned(),
    }
}
