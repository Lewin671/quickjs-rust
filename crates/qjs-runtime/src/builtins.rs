use crate::{
    CallEnv, Value, array, array_buffer, async_function, async_generator, bigint, boolean,
    data_view, date, error, generator, global, iterator, json, map, math, number, object, promise,
    proxy, reflect, regexp, set, string, symbol, typed_array, weak_map, weak_set,
};

pub(crate) fn initialize_builtins(env: &mut CallEnv, global_this: &Value) {
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

    // `%Iterator.prototype%` needs well-known symbols (Symbol.iterator,
    // Symbol.toStringTag) and must exist before any built-in iterator
    // prototype (generator, array/string/map/set iterators) inherits it.
    iterator::install_iterator(env, global_this, object_prototype.clone());
    array_buffer::install_array_buffer(env, global_this, object_prototype.clone());
    typed_array::install_typed_arrays(env, global_this, object_prototype.clone());
    data_view::install_data_view(env, global_this, object_prototype.clone());
    bigint::install_bigint_well_known_symbols(env);
    string::install_string_well_known_symbols(env);
    boolean::install_boolean(env, global_this, object_prototype.clone());
    date::install_date(env, global_this, object_prototype.clone());
    regexp::install_regexp(env, global_this, object_prototype.clone());
    error::install_error(env, global_this, object_prototype.clone());
    json::install_json(env, global_this, object_prototype.clone());
    promise::install_promise(env, global_this, object_prototype.clone());
    proxy::install_proxy(env, global_this, object_prototype.clone());
    map::install_map(env, global_this, object_prototype.clone());
    weak_map::install_weak_map(env, global_this, object_prototype.clone());
    weak_set::install_weak_set(env, global_this, object_prototype.clone());
    set::install_set(env, global_this, object_prototype.clone());
    math::install_math(env, global_this, object_prototype.clone());
    reflect::install_reflect(env, global_this, object_prototype.clone());
    array::install_array(env, global_this, object_prototype.clone());
    async_function::install_async_function(env, global_this, object_prototype.clone());
    async_generator::install_async_generator(env, global_this, object_prototype.clone());
    generator::install_generator(env, global_this, object_prototype);
}
