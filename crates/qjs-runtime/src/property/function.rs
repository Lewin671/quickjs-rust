use crate::{Function, ObjectRef, Property};

pub(crate) fn function_own_property_keys(function: &Function) -> Vec<String> {
    function.own_property_keys()
}

pub(crate) fn function_own_property_descriptor(function: &Function, key: &str) -> Option<Property> {
    function.own_property(key)
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
