//! The `Iterator` intrinsic: the `%Iterator.prototype%` object, the `Iterator`
//! global constructor, `Iterator.from`, and the iterator helper methods
//! (`map`/`filter`/`take`/`drop`/`flatMap` plus the eager
//! `reduce`/`toArray`/`forEach`/`some`/`every`/`find`) from the ES2025 Iterator
//! Helpers proposal (27.1).
//!
//! `%Iterator.prototype%` is the shared root of every built-in iterator: array,
//! string, map, set, and generator iterators all inherit from it, so the
//! helpers are available on each. The lazy helpers return Iterator Helper
//! objects whose `[[Prototype]]` is `%IteratorHelperPrototype%`; the eager
//! methods drive the iterator protocol directly and close the source iterator
//! on abrupt completion.

mod eager;
mod from;
mod helpers;
mod protocol;
mod zip;

pub(crate) use zip::ZipState;

use std::collections::HashMap;

use crate::CallEnv;
use crate::{Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, symbol};

/// Intrinsic binding for `%Iterator.prototype%`, propagated into call frames so
/// built-in iterators (built lazily during execution) can reach it for their
/// `[[Prototype]]`.
pub(crate) const ITERATOR_PROTOTYPE_BINDING: &str = "\0IteratorPrototype";

/// Intrinsic binding for `%IteratorHelperPrototype%`, the `[[Prototype]]` of the
/// objects returned by the lazy helper methods.
pub(crate) const ITERATOR_HELPER_PROTOTYPE_BINDING: &str = "\0IteratorHelperPrototype";

/// Intrinsic binding for `%WrapForValidIteratorPrototype%`, the `[[Prototype]]`
/// of the wrapper `Iterator.from` returns for a foreign iterator.
pub(crate) const WRAP_PROTOTYPE_BINDING: &str = "\0WrapForValidIteratorPrototype";

/// The built-in iterator-instance prototypes (e.g. `%ArrayIteratorPrototype%`),
/// each inheriting `%Iterator.prototype%` so the helpers reach every built-in
/// iterator. They carry a kind-specific `next` and a toStringTag.
#[derive(Clone, Copy)]
pub(crate) enum BuiltinIteratorKind {
    Array,
    String,
    Map,
    Set,
    RegExpString,
}

impl BuiltinIteratorKind {
    const fn binding(self) -> &'static str {
        match self {
            Self::Array => "\0ArrayIteratorPrototype",
            Self::String => "\0StringIteratorPrototype",
            Self::Map => "\0MapIteratorPrototype",
            Self::Set => "\0SetIteratorPrototype",
            Self::RegExpString => "\0RegExpStringIteratorPrototype",
        }
    }

    const fn tag(self) -> &'static str {
        match self {
            Self::Array => "Array Iterator",
            Self::String => "String Iterator",
            Self::Map => "Map Iterator",
            Self::Set => "Set Iterator",
            Self::RegExpString => "RegExp String Iterator",
        }
    }

    const fn next_native(self) -> NativeFunction {
        match self {
            Self::Array => NativeFunction::ArrayIteratorPrototypeNext,
            Self::String => NativeFunction::StringIteratorPrototypeNext,
            Self::Map => NativeFunction::MapIteratorPrototypeNext,
            Self::Set => NativeFunction::SetIteratorPrototypeNext,
            Self::RegExpString => NativeFunction::RegExpStringIteratorPrototypeNext,
        }
    }
}

/// Returns the shared `[[Prototype]]` for a built-in iterator instance of the
/// given kind, so iterator-creation sites can inherit the helpers.
pub(crate) fn builtin_iterator_prototype(
    env: &CallEnv,
    kind: BuiltinIteratorKind,
) -> Option<ObjectRef> {
    match env.get(kind.binding()) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Returns `%Iterator.prototype%` from the current environment.
pub(crate) fn iterator_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(ITERATOR_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Returns `%IteratorHelperPrototype%` from the current environment.
fn iterator_helper_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(ITERATOR_HELPER_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Installs `%Iterator.prototype%`, `%IteratorHelperPrototype%`, and the
/// `Iterator` global constructor.
pub(crate) fn install_iterator(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let iterator_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));

    // Lazy helper methods.
    for (name, length, native) in [
        ("map", 1, NativeFunction::IteratorPrototypeMap),
        ("filter", 1, NativeFunction::IteratorPrototypeFilter),
        ("take", 1, NativeFunction::IteratorPrototypeTake),
        ("drop", 1, NativeFunction::IteratorPrototypeDrop),
        ("flatMap", 1, NativeFunction::IteratorPrototypeFlatMap),
        // Eager methods.
        ("reduce", 1, NativeFunction::IteratorPrototypeReduce),
        ("toArray", 0, NativeFunction::IteratorPrototypeToArray),
        ("forEach", 1, NativeFunction::IteratorPrototypeForEach),
        ("some", 1, NativeFunction::IteratorPrototypeSome),
        ("every", 1, NativeFunction::IteratorPrototypeEvery),
        ("find", 1, NativeFunction::IteratorPrototypeFind),
    ] {
        iterator_prototype.define_non_enumerable(
            name.to_owned(),
            Value::Function(Function::new_native(Some(name), length, native, false)),
        );
    }

    // `%Iterator.prototype%[Symbol.iterator]` returns `this`.
    if let Some(iterator_symbol) = symbol::iterator_symbol(env) {
        iterator_prototype.define_symbol_property(
            iterator_symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.iterator]"),
                0,
                NativeFunction::IteratorPrototypeIterator,
                false,
            ))),
        );
    }
    if let Some(dispose_symbol) = symbol::dispose_symbol(env) {
        iterator_prototype.define_symbol_property(
            dispose_symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.dispose]"),
                0,
                NativeFunction::IteratorPrototypeDispose,
                false,
            ))),
        );
    }

    // `%Iterator.prototype%[Symbol.toStringTag]` is a validating accessor pair,
    // and `constructor` is likewise an accessor (27.1.4.1/27.1.4.2). Both reject
    // a write whose receiver is `%Iterator.prototype%` itself and otherwise set
    // an own property on the receiver.
    if let Some(tag_symbol) = symbol::to_string_tag_symbol(env) {
        iterator_prototype.define_symbol_property(
            tag_symbol,
            Property::accessor(
                Some(Value::Function(Function::new_native(
                    Some("get [Symbol.toStringTag]"),
                    0,
                    NativeFunction::IteratorPrototypeToStringTagGet,
                    false,
                ))),
                Some(Value::Function(Function::new_native(
                    Some("set [Symbol.toStringTag]"),
                    1,
                    NativeFunction::IteratorPrototypeToStringTagSet,
                    false,
                ))),
                false,
                true,
            ),
        );
    }
    iterator_prototype.define_property(
        "constructor".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get constructor"),
                0,
                NativeFunction::IteratorPrototypeConstructorGet,
                false,
            ))),
            Some(Value::Function(Function::new_native(
                Some("set constructor"),
                1,
                NativeFunction::IteratorPrototypeConstructorSet,
                false,
            ))),
            false,
            true,
        ),
    );

    // `%IteratorHelperPrototype%` inherits `%Iterator.prototype%`, exposes
    // `next`/`return`, and carries the "Iterator Helper" toStringTag.
    let helper_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(iterator_prototype.clone()));
    helper_prototype.define_non_enumerable(
        "next".to_owned(),
        Value::Function(Function::new_native(
            Some("next"),
            0,
            NativeFunction::IteratorHelperPrototypeNext,
            false,
        )),
    );
    helper_prototype.define_non_enumerable(
        "return".to_owned(),
        Value::Function(Function::new_native(
            Some("return"),
            0,
            NativeFunction::IteratorHelperPrototypeReturn,
            false,
        )),
    );
    if let Some(tag_symbol) = symbol::to_string_tag_symbol(env) {
        helper_prototype.define_symbol_property(
            tag_symbol,
            Property::data(
                Value::String("Iterator Helper".to_owned().into()),
                false,
                false,
                true,
            ),
        );
    }

    // The `Iterator` constructor: not directly constructable, subclassable.
    let iterator_function =
        Function::new_native(Some("Iterator"), 0, NativeFunction::Iterator, true);
    iterator_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(iterator_prototype.clone())),
    );
    iterator_function.properties.borrow_mut().insert(
        "__quickjsRustRealmIteratorPrototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(iterator_prototype.clone())),
    );
    iterator_function.properties.borrow_mut().insert(
        "from".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("from"),
            1,
            NativeFunction::IteratorFrom,
            false,
        ))),
    );
    iterator_function.properties.borrow_mut().insert(
        "concat".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("concat"),
            0,
            NativeFunction::IteratorConcat,
            false,
        ))),
    );
    iterator_function.properties.borrow_mut().insert(
        "zip".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("zip"),
            1,
            NativeFunction::IteratorZip,
            false,
        ))),
    );
    iterator_function.properties.borrow_mut().insert(
        "zipKeyed".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("zipKeyed"),
            1,
            NativeFunction::IteratorZipKeyed,
            false,
        ))),
    );

    let wrap_prototype = from::build_wrap_prototype(env, &iterator_prototype);

    // Build the per-kind built-in iterator prototypes, each inheriting
    // %Iterator.prototype% so array/string/map/set iterators expose the helpers.
    for kind in [
        BuiltinIteratorKind::Array,
        BuiltinIteratorKind::String,
        BuiltinIteratorKind::Map,
        BuiltinIteratorKind::Set,
        BuiltinIteratorKind::RegExpString,
    ] {
        let prototype = ObjectRef::with_prototype(HashMap::new(), Some(iterator_prototype.clone()));
        prototype.define_non_enumerable(
            "next".to_owned(),
            Value::Function(Function::new_native(
                Some("next"),
                0,
                kind.next_native(),
                false,
            )),
        );
        if let Some(tag_symbol) = symbol::to_string_tag_symbol(env) {
            prototype.define_symbol_property(
                tag_symbol,
                Property::data(
                    Value::String(kind.tag().to_owned().into()),
                    false,
                    false,
                    true,
                ),
            );
        }
        env.insert_realm(kind.binding().to_owned(), Value::Object(prototype));
    }

    env.insert_realm(
        ITERATOR_PROTOTYPE_BINDING.to_owned(),
        Value::Object(iterator_prototype),
    );
    env.insert_realm(
        ITERATOR_HELPER_PROTOTYPE_BINDING.to_owned(),
        Value::Object(helper_prototype),
    );
    env.insert_realm(
        WRAP_PROTOTYPE_BINDING.to_owned(),
        Value::Object(wrap_prototype),
    );

    let value = Value::Function(iterator_function);
    env.insert_realm("Iterator".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Iterator".to_owned(), value);
    }
}

/// Dispatches the `Iterator` constructor, `Iterator.from`, the
/// `%Iterator.prototype%` accessors and helper methods, and the
/// `%IteratorHelperPrototype%` `next`/`return` methods.
pub(crate) fn call_iterator_native(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let result = match native {
        NativeFunction::Iterator => native_iterator_constructor(is_construct, env)?,
        NativeFunction::IteratorFrom => from::native_iterator_from(argument_values, env)?,
        NativeFunction::IteratorConcat => helpers::native_iterator_concat(argument_values, env)?,
        NativeFunction::IteratorZip => helpers::native_iterator_zip(argument_values, env)?,
        NativeFunction::IteratorZipKeyed => {
            helpers::native_iterator_zip_keyed(argument_values, env)?
        }
        NativeFunction::IteratorPrototypeToStringTagGet => {
            return Ok(Some(Value::String("Iterator".to_owned().into())));
        }
        NativeFunction::IteratorPrototypeToStringTagSet => {
            let key = symbol::to_string_tag_symbol(env).map(SetterKey::Symbol);
            setter_ignoring_prototype(this_value, key, argument_values, env)?
        }
        NativeFunction::IteratorPrototypeConstructorGet => match env.get("Iterator") {
            Some(value) => value.clone(),
            None => Value::Undefined,
        },
        NativeFunction::IteratorPrototypeConstructorSet => setter_ignoring_prototype(
            this_value,
            Some(SetterKey::Named("constructor")),
            argument_values,
            env,
        )?,
        NativeFunction::IteratorPrototypeDispose => native_iterator_dispose(this_value, env)?,
        NativeFunction::IteratorPrototypeMap
        | NativeFunction::IteratorPrototypeFilter
        | NativeFunction::IteratorPrototypeTake
        | NativeFunction::IteratorPrototypeDrop
        | NativeFunction::IteratorPrototypeFlatMap => {
            helpers::native_lazy_helper(native, this_value, argument_values, env)?
        }
        NativeFunction::IteratorHelperPrototypeNext => {
            helpers::native_helper_next(this_value, env)?
        }
        NativeFunction::IteratorHelperPrototypeReturn => {
            helpers::native_helper_return(this_value, env)?
        }
        NativeFunction::WrapForValidIteratorPrototypeNext => {
            from::native_wrap_next(this_value, env)?
        }
        NativeFunction::WrapForValidIteratorPrototypeReturn => {
            from::native_wrap_return(this_value, env)?
        }
        NativeFunction::IteratorPrototypeReduce
        | NativeFunction::IteratorPrototypeToArray
        | NativeFunction::IteratorPrototypeForEach
        | NativeFunction::IteratorPrototypeSome
        | NativeFunction::IteratorPrototypeEvery
        | NativeFunction::IteratorPrototypeFind => {
            eager::native_eager_helper(native, this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn native_iterator_dispose(this_value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let return_method = crate::property_value(this_value.clone(), "return", env)?;
    if !matches!(return_method, Value::Undefined | Value::Null) {
        if !matches!(return_method, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: iterator return is not a function".to_owned(),
            });
        }
        crate::call_function(return_method, this_value, Vec::new(), env, false)?;
    }
    Ok(Value::Undefined)
}

/// The `Iterator` constructor (27.1.1.1): throws a TypeError when called
/// without `new` or with `new.target` equal to `Iterator` itself. A subclass
/// constructor (different `new.target`) receives its ordinary `this` object,
/// already created with the correct prototype, so it is returned unchanged.
fn native_iterator_constructor(is_construct: bool, env: &CallEnv) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator is not directly constructable".to_owned(),
        });
    }
    let new_target = env.get(crate::NEW_TARGET_BINDING);
    let directly = match (new_target, env.get("Iterator")) {
        (Some(Value::Function(target)), Some(Value::Function(iterator))) => {
            target.ptr_eq(&iterator)
        }
        (None, _) | (Some(Value::Undefined), _) => true,
        _ => false,
    };
    if directly {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator is an abstract base class and cannot be directly \
                      constructed"
                .to_owned(),
        });
    }
    // The subclass instance was created by `construct_function` from
    // `new.target.prototype`; native constructors return `this`, so yield it.
    Ok(Value::Undefined)
}

/// Property key targeted by the validating accessor setters.
enum SetterKey {
    Named(&'static str),
    Symbol(ObjectRef),
}

/// SetterThatIgnoresPrototypeProperties (27.1.4): rejects a non-object receiver
/// and a write whose receiver is `%Iterator.prototype%`, otherwise sets an own
/// data property on the receiver (overwriting an existing own property's value,
/// or creating an enumerable/writable/configurable one).
fn setter_ignoring_prototype(
    this_value: Value,
    key: Option<SetterKey>,
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(receiver) = &this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: receiver must be an object".to_owned(),
        });
    };
    if let Some(Value::Object(prototype)) = env.get(ITERATOR_PROTOTYPE_BINDING)
        && receiver.ptr_eq(&prototype)
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot set property on %Iterator.prototype%".to_owned(),
        });
    }
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match key {
        Some(SetterKey::Named(name)) => {
            if let Some(mut existing) = receiver.own_property(name) {
                existing.value = value;
                receiver.define_property(name.to_owned(), existing);
            } else {
                receiver.define_property(name.to_owned(), Property::enumerable(value));
            }
        }
        Some(SetterKey::Symbol(symbol)) => {
            if let Some(mut existing) = receiver.own_symbol_property(&symbol) {
                existing.value = value;
                receiver.define_symbol_property(symbol, existing);
            } else {
                receiver.define_symbol_property(symbol, Property::enumerable(value));
            }
        }
        None => {}
    }
    Ok(Value::Undefined)
}
