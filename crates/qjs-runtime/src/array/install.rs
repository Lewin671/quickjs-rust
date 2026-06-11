use std::collections::HashMap;

use crate::CallEnv;
use crate::{Function, NativeFunction, ObjectRef, Property, Value, symbol};

pub(crate) fn install_array(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let array_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let array_function = Function::new_native(Some("Array"), 1, NativeFunction::Array, true);
    array_prototype.define_non_enumerable("length".to_owned(), Value::Number(0.0));
    array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_function.clone()),
    );
    define_array_prototype_function(&array_prototype, "at", 1, NativeFunction::ArrayPrototypeAt);
    define_array_prototype_function(
        &array_prototype,
        "concat",
        1,
        NativeFunction::ArrayPrototypeConcat,
    );
    define_array_prototype_function(
        &array_prototype,
        "copyWithin",
        2,
        NativeFunction::ArrayPrototypeCopyWithin,
    );
    define_array_prototype_function(
        &array_prototype,
        "entries",
        0,
        NativeFunction::ArrayPrototypeEntries,
    );
    define_array_prototype_function(
        &array_prototype,
        "every",
        1,
        NativeFunction::ArrayPrototypeEvery,
    );
    define_array_prototype_function(
        &array_prototype,
        "fill",
        1,
        NativeFunction::ArrayPrototypeFill,
    );
    define_array_prototype_function(
        &array_prototype,
        "flat",
        0,
        NativeFunction::ArrayPrototypeFlat,
    );
    define_array_prototype_function(
        &array_prototype,
        "flatMap",
        1,
        NativeFunction::ArrayPrototypeFlatMap,
    );
    define_array_prototype_function(
        &array_prototype,
        "filter",
        1,
        NativeFunction::ArrayPrototypeFilter,
    );
    define_array_prototype_function(
        &array_prototype,
        "find",
        1,
        NativeFunction::ArrayPrototypeFind,
    );
    define_array_prototype_function(
        &array_prototype,
        "findIndex",
        1,
        NativeFunction::ArrayPrototypeFindIndex,
    );
    define_array_prototype_function(
        &array_prototype,
        "findLast",
        1,
        NativeFunction::ArrayPrototypeFindLast,
    );
    define_array_prototype_function(
        &array_prototype,
        "findLastIndex",
        1,
        NativeFunction::ArrayPrototypeFindLastIndex,
    );
    define_array_prototype_function(
        &array_prototype,
        "forEach",
        1,
        NativeFunction::ArrayPrototypeForEach,
    );
    define_array_prototype_function(
        &array_prototype,
        "includes",
        1,
        NativeFunction::ArrayPrototypeIncludes,
    );
    define_array_prototype_function(
        &array_prototype,
        "join",
        1,
        NativeFunction::ArrayPrototypeJoin,
    );
    define_array_prototype_function(
        &array_prototype,
        "indexOf",
        1,
        NativeFunction::ArrayPrototypeIndexOf,
    );
    define_array_prototype_function(
        &array_prototype,
        "keys",
        0,
        NativeFunction::ArrayPrototypeKeys,
    );
    define_array_prototype_function(
        &array_prototype,
        "lastIndexOf",
        1,
        NativeFunction::ArrayPrototypeLastIndexOf,
    );
    define_array_prototype_function(
        &array_prototype,
        "map",
        1,
        NativeFunction::ArrayPrototypeMap,
    );
    define_array_prototype_function(
        &array_prototype,
        "pop",
        0,
        NativeFunction::ArrayPrototypePop,
    );
    define_array_prototype_function(
        &array_prototype,
        "push",
        1,
        NativeFunction::ArrayPrototypePush,
    );
    define_array_prototype_function(
        &array_prototype,
        "reduce",
        1,
        NativeFunction::ArrayPrototypeReduce,
    );
    define_array_prototype_function(
        &array_prototype,
        "reduceRight",
        1,
        NativeFunction::ArrayPrototypeReduceRight,
    );
    define_array_prototype_function(
        &array_prototype,
        "reverse",
        0,
        NativeFunction::ArrayPrototypeReverse,
    );
    define_array_prototype_function(
        &array_prototype,
        "shift",
        0,
        NativeFunction::ArrayPrototypeShift,
    );
    define_array_prototype_function(
        &array_prototype,
        "slice",
        2,
        NativeFunction::ArrayPrototypeSlice,
    );
    define_array_prototype_function(
        &array_prototype,
        "some",
        1,
        NativeFunction::ArrayPrototypeSome,
    );
    define_array_prototype_function(
        &array_prototype,
        "sort",
        1,
        NativeFunction::ArrayPrototypeSort,
    );
    define_array_prototype_function(
        &array_prototype,
        "splice",
        2,
        NativeFunction::ArrayPrototypeSplice,
    );
    define_array_prototype_function(
        &array_prototype,
        "toString",
        0,
        NativeFunction::ArrayPrototypeToString,
    );
    define_array_prototype_function(
        &array_prototype,
        "toLocaleString",
        0,
        NativeFunction::ArrayPrototypeToLocaleString,
    );
    define_array_prototype_function(
        &array_prototype,
        "toReversed",
        0,
        NativeFunction::ArrayPrototypeToReversed,
    );
    define_array_prototype_function(
        &array_prototype,
        "toSpliced",
        2,
        NativeFunction::ArrayPrototypeToSpliced,
    );
    define_array_prototype_function(
        &array_prototype,
        "toSorted",
        1,
        NativeFunction::ArrayPrototypeToSorted,
    );
    define_array_prototype_function(
        &array_prototype,
        "unshift",
        1,
        NativeFunction::ArrayPrototypeUnshift,
    );
    define_array_prototype_function(
        &array_prototype,
        "values",
        0,
        NativeFunction::ArrayPrototypeValues,
    );
    symbol::define_well_known_iterator_alias(env, &array_prototype, "values");
    define_array_prototype_function(
        &array_prototype,
        "with",
        2,
        NativeFunction::ArrayPrototypeWith,
    );
    define_array_unscopables(env, &array_prototype);
    array_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(array_prototype)),
    );
    symbol::define_species_accessor(env, &array_function);
    define_array_function(&array_function, "from", 1, NativeFunction::ArrayFrom);
    define_array_function(
        &array_function,
        "fromAsync",
        1,
        NativeFunction::ArrayFromAsync,
    );
    define_array_function(&array_function, "isArray", 1, NativeFunction::ArrayIsArray);
    define_array_function(&array_function, "of", 0, NativeFunction::ArrayOf);

    let array_value = Value::Function(array_function);
    env.insert_realm("Array".to_owned(), array_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Array".to_owned(), array_value);
    }
}

fn define_array_prototype_function(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

fn define_array_function(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some(key),
            length,
            native,
            false,
        ))),
    );
}

fn define_array_unscopables(env: &CallEnv, prototype: &ObjectRef) {
    let Some(symbol) = symbol::unscopables_symbol(env) else {
        return;
    };

    let unscopables = ObjectRef::new(HashMap::new());
    for key in [
        "copyWithin",
        "entries",
        "fill",
        "find",
        "findIndex",
        "findLast",
        "findLastIndex",
        "flat",
        "flatMap",
        "includes",
        "keys",
        "toReversed",
        "toSorted",
        "toSpliced",
        "values",
    ] {
        unscopables.define_property(key.to_owned(), Property::enumerable(Value::Boolean(true)));
    }

    prototype.define_symbol_property(
        symbol,
        Property::data(Value::Object(unscopables), false, false, true),
    );
}
