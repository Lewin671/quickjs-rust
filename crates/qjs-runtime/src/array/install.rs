use std::collections::HashMap;

use crate::CallEnv;
use crate::{Function, NativeFunction, ObjectRef, Property, Value, symbol};

pub(crate) fn install_array(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let array_prototype =
        ObjectRef::with_prototype(env.realm().atoms(), HashMap::new(), Some(object_prototype));
    array_prototype.mark_array_prototype_exotic();
    env.realm()
        .initialize_array_prototype(array_prototype.clone());
    let array_function = Function::new_native(
        env.realm().atoms(),
        Some("Array"),
        1,
        NativeFunction::Array,
        true,
    );
    array_prototype.define_property(
        "length".to_owned(),
        Property::data(Value::Number(0.0), false, true, false),
    );
    array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_function.clone()),
    );
    array_prototype.define_native_methods(&[
        ("at", 1, NativeFunction::ArrayPrototypeAt),
        ("concat", 1, NativeFunction::ArrayPrototypeConcat),
        ("copyWithin", 2, NativeFunction::ArrayPrototypeCopyWithin),
        ("entries", 0, NativeFunction::ArrayPrototypeEntries),
        ("every", 1, NativeFunction::ArrayPrototypeEvery),
        ("fill", 1, NativeFunction::ArrayPrototypeFill),
        ("flat", 0, NativeFunction::ArrayPrototypeFlat),
        ("flatMap", 1, NativeFunction::ArrayPrototypeFlatMap),
        ("filter", 1, NativeFunction::ArrayPrototypeFilter),
        ("find", 1, NativeFunction::ArrayPrototypeFind),
        ("findIndex", 1, NativeFunction::ArrayPrototypeFindIndex),
        ("findLast", 1, NativeFunction::ArrayPrototypeFindLast),
        (
            "findLastIndex",
            1,
            NativeFunction::ArrayPrototypeFindLastIndex,
        ),
        ("forEach", 1, NativeFunction::ArrayPrototypeForEach),
        ("includes", 1, NativeFunction::ArrayPrototypeIncludes),
        ("join", 1, NativeFunction::ArrayPrototypeJoin),
        ("indexOf", 1, NativeFunction::ArrayPrototypeIndexOf),
        ("keys", 0, NativeFunction::ArrayPrototypeKeys),
        ("lastIndexOf", 1, NativeFunction::ArrayPrototypeLastIndexOf),
        ("map", 1, NativeFunction::ArrayPrototypeMap),
        ("pop", 0, NativeFunction::ArrayPrototypePop),
        ("push", 1, NativeFunction::ArrayPrototypePush),
        ("reduce", 1, NativeFunction::ArrayPrototypeReduce),
        ("reduceRight", 1, NativeFunction::ArrayPrototypeReduceRight),
        ("reverse", 0, NativeFunction::ArrayPrototypeReverse),
        ("shift", 0, NativeFunction::ArrayPrototypeShift),
        ("slice", 2, NativeFunction::ArrayPrototypeSlice),
        ("some", 1, NativeFunction::ArrayPrototypeSome),
        ("sort", 1, NativeFunction::ArrayPrototypeSort),
        ("splice", 2, NativeFunction::ArrayPrototypeSplice),
        ("toString", 0, NativeFunction::ArrayPrototypeToString),
        (
            "toLocaleString",
            0,
            NativeFunction::ArrayPrototypeToLocaleString,
        ),
        ("toReversed", 0, NativeFunction::ArrayPrototypeToReversed),
        ("toSpliced", 2, NativeFunction::ArrayPrototypeToSpliced),
        ("toSorted", 1, NativeFunction::ArrayPrototypeToSorted),
        ("unshift", 1, NativeFunction::ArrayPrototypeUnshift),
        ("values", 0, NativeFunction::ArrayPrototypeValues),
    ]);
    symbol::define_well_known_iterator_alias(env, &array_prototype, "values");
    // Share %Array.prototype.values% so an arguments object's
    // `[Symbol.iterator]` is the same function object (spec %ArrayProto_values%).
    if let Some(values) = array_prototype.own_property("values") {
        env.insert_realm(
            crate::function::ARRAY_PROTO_VALUES_INTRINSIC.to_owned(),
            values.value.clone(),
        );
    }
    define_array_prototype_function(
        &array_prototype,
        "with",
        2,
        NativeFunction::ArrayPrototypeWith,
    );
    define_array_unscopables(env, &array_prototype);
    array_function.properties.borrow_mut().insert_in(
        env.realm().atoms(),
        "prototype",
        Property::fixed_non_enumerable(Value::Object(array_prototype)),
    );
    symbol::define_species_accessor(env, &array_function);
    define_array_function(
        env.realm().atoms(),
        &array_function,
        "from",
        1,
        NativeFunction::ArrayFrom,
    );
    define_array_function(
        env.realm().atoms(),
        &array_function,
        "fromAsync",
        1,
        NativeFunction::ArrayFromAsync,
    );
    define_array_function(
        env.realm().atoms(),
        &array_function,
        "isArray",
        1,
        NativeFunction::ArrayIsArray,
    );
    define_array_function(
        env.realm().atoms(),
        &array_function,
        "of",
        0,
        NativeFunction::ArrayOf,
    );

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
        Value::Function(Function::new_native(
            &prototype.store_domain(),
            Some(key),
            length,
            native,
            false,
        )),
    );
}

fn define_array_function(
    atoms: &crate::property::atom::AtomDomain,
    function: &Function,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    function.properties.borrow_mut().insert_in(
        atoms,
        key,
        Property::non_enumerable(Value::Function(Function::new_native(
            atoms,
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

    let unscopables = ObjectRef::new_in(env.realm().atoms(), HashMap::new());
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
