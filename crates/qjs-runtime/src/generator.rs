//! Generator object intrinsics: `%GeneratorPrototype%` and the
//! `next`/`return`/`throw` protocol methods. The suspend/resume state machine
//! lives in `bytecode::vm_generator`; this module wires the prototype, builds
//! generator objects when a `function*` is called, and adapts the resume
//! outcomes into iterator-result objects.

use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    bytecode::{GeneratorOutcome, GeneratorStart, GeneratorState, Resume, resume_generator},
    object_prototype, symbol,
};

/// Intrinsic binding for `%GeneratorPrototype%`, propagated into call frames so
/// a `function*` can reach it when building its generator objects.
pub(crate) const GENERATOR_PROTOTYPE_BINDING: &str = "\0GeneratorPrototype";

/// Installs `%GeneratorPrototype%` (with `next`/`return`/`throw`,
/// `Symbol.iterator`, and the `Generator` toStringTag) and the
/// `%GeneratorFunction.prototype%` object, recording both under intrinsic
/// bindings so generator function calls can wire generator objects.
pub(crate) fn install_generator(
    env: &mut HashMap<String, Value>,
    _global_this: &Value,
    object_prototype: ObjectRef,
) {
    // The IteratorPrototype layer is modeled by giving %GeneratorPrototype% the
    // ordinary object prototype and its own Symbol.iterator returning `this`.
    let generator_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));

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

    // The %GeneratorFunction% / %GeneratorFunction.prototype% intrinsic chain
    // (so `Object.getPrototypeOf(g).constructor` walks to GeneratorFunction) is
    // a follow-up: the runtime cannot yet use a function as a [[Prototype]]
    // value, so wiring it here would not match observable identity.
    let _ = &object_prototype;

    env.insert(
        GENERATOR_PROTOTYPE_BINDING.to_owned(),
        Value::Object(generator_prototype),
    );
}

/// Returns `%GeneratorPrototype%` from the current environment.
fn generator_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    match env.get(GENERATOR_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Builds the generator object returned by calling a `function*`: an ordinary
/// object whose [[Prototype]] is the function's own `prototype` (when an
/// object) or `%GeneratorPrototype%`, carrying the captured call frame as its
/// initial `SuspendedStart` state.
pub(crate) fn make_generator_object(
    function: &Function,
    start: GeneratorStart,
    env: &HashMap<String, Value>,
) -> Value {
    let prototype = generator_object_prototype(function, env);
    let generator = ObjectRef::with_prototype(HashMap::new(), prototype);
    *generator.generator_state().borrow_mut() =
        Some(GeneratorState::SuspendedStart(Box::new(start)));
    Value::Object(generator)
}

/// Resolves the [[Prototype]] for a generator object: the function's own
/// `prototype` property when it is an object, otherwise `%GeneratorPrototype%`.
fn generator_object_prototype(
    function: &Function,
    env: &HashMap<String, Value>,
) -> Option<ObjectRef> {
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
    env: &mut HashMap<String, Value>,
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
        GeneratorOutcome::Return(value) => iterator_result(value, true, env),
    };
    Ok(Some(result))
}

fn iterator_result(value: Value, done: bool, env: &HashMap<String, Value>) -> Value {
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
