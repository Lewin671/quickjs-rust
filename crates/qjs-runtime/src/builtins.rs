use std::collections::HashMap;

use crate::{
    Value, array, boolean, date, error, global, json, map, math, number, object, reflect, regexp,
    string, symbol,
};

pub(crate) fn initialize_builtins(env: &mut HashMap<String, Value>, global_this: &Value) {
    let object_prototype = object::install_object(env, global_this);

    crate::function::install_function(env, global_this, object_prototype.clone());
    global::install_globals(env, global_this);

    number::install_number(env, global_this, object_prototype.clone());
    string::install_string(env, global_this, object_prototype.clone());
    symbol::install_symbol(env, global_this, object_prototype.clone());
    boolean::install_boolean(env, global_this, object_prototype.clone());
    date::install_date(env, global_this, object_prototype.clone());
    regexp::install_regexp(env, global_this, object_prototype.clone());
    error::install_error(env, global_this, object_prototype.clone());
    json::install_json(env, global_this, object_prototype.clone());
    map::install_map(env, global_this, object_prototype.clone());
    math::install_math(env, global_this, object_prototype.clone());
    reflect::install_reflect(env, global_this, object_prototype.clone());
    array::install_array(env, global_this, object_prototype);
}
