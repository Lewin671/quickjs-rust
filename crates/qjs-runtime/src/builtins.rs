use std::collections::HashMap;

use crate::{
    Value, array, array_buffer, bigint, boolean, date, error, global, json, map, math, number,
    object, promise, reflect, regexp, set, string, symbol, weak_map, weak_set,
};

pub(crate) fn initialize_builtins(env: &mut HashMap<String, Value>, global_this: &Value) {
    let object_prototype = object::install_object(env, global_this);
    if let Value::Object(global_object) = global_this {
        let _ = global_object.set_prototype(Some(object_prototype.clone()));
    }

    crate::function::install_function(env, global_this, object_prototype.clone());
    global::install_globals(env, global_this);

    bigint::install_bigint(env, global_this, object_prototype.clone());
    number::install_number(env, global_this, object_prototype.clone());
    string::install_string(env, global_this, object_prototype.clone());
    symbol::install_symbol(env, global_this, object_prototype.clone());
    array_buffer::install_array_buffer(env, global_this, object_prototype.clone());
    bigint::install_bigint_well_known_symbols(env);
    string::install_string_well_known_symbols(env);
    boolean::install_boolean(env, global_this, object_prototype.clone());
    date::install_date(env, global_this, object_prototype.clone());
    regexp::install_regexp(env, global_this, object_prototype.clone());
    error::install_error(env, global_this, object_prototype.clone());
    json::install_json(env, global_this, object_prototype.clone());
    promise::install_promise(env, global_this, object_prototype.clone());
    map::install_map(env, global_this, object_prototype.clone());
    weak_map::install_weak_map(env, global_this, object_prototype.clone());
    weak_set::install_weak_set(env, global_this, object_prototype.clone());
    set::install_set(env, global_this, object_prototype.clone());
    math::install_math(env, global_this, object_prototype.clone());
    reflect::install_reflect(env, global_this, object_prototype.clone());
    array::install_array(env, global_this, object_prototype);
}
