use crate::{Function, ObjectRef, Property, Value};

pub(crate) fn function_own_property_keys(function: &Function) -> Vec<String> {
    function.own_property_keys()
}

pub(crate) fn function_own_property_descriptor(function: &Function, key: &str) -> Option<Property> {
    if let Some(prop) = function.own_property(key) {
        return Some(prop);
    }
    // Non-strict functions expose `caller` and `arguments` as own data
    // properties (value null) so they shadow the throwing accessors on
    // Function.prototype. Strict functions inherit the prototype accessors.
    if !function.is_strict && (key == "caller" || key == "arguments") {
        return Some(Property::data(Value::Null, false, false, true));
    }
    None
}

pub(crate) fn function_delete_own_property(function: &Function, key: &str) -> bool {
    function.delete_own_property(key)
}

pub(crate) fn function_own_symbol_property_descriptor(
    function: &Function,
    symbol: &ObjectRef,
) -> Option<Property> {
    function.own_symbol_property(symbol)
}

pub(crate) fn function_delete_own_symbol_property(function: &Function, symbol: &ObjectRef) -> bool {
    function.delete_own_symbol_property(symbol)
}

pub(crate) fn function_own_property_names(function: &Function) -> Vec<String> {
    function.own_property_names()
}

pub(crate) fn function_own_property_symbols(function: &Function) -> Vec<ObjectRef> {
    function.own_property_symbols()
}
