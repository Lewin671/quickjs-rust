//! Releasing long ownership chains without recursion.
//!
//! Dropping the last reference to an object drops its property values, and
//! a value that was itself the last reference to an object drops that
//! object's values in turn: a linked list of `{ next }` nodes released from
//! its head recursed once per node and overflowed the native stack at about
//! 100,000 nodes. An object or array now moves each child it alone owns onto
//! a work list instead, and the list is drained by a loop.

use super::Value;

/// Moves `value` onto `pending` when it is the last strong reference to an
/// object or array, whose own children would otherwise be dropped from
/// inside this drop. Anything else stays for the ordinary drop.
#[inline]
pub(super) fn defer_if_last(value: &mut Value, pending: &mut Vec<Value>) {
    let last = match value {
        Value::Object(object) => object.is_last_reference(),
        Value::Array(array) => array.is_last_reference(),
        _ => false,
    };
    if last {
        pending.push(std::mem::replace(value, Value::Undefined));
    }
}

/// Drops every deferred value, deferring their own children onto the same
/// list, so the depth of the chain never reaches the native stack.
#[cold]
#[inline(never)]
pub(super) fn release(mut pending: Vec<Value>) {
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(object) => object.release_into(&mut pending),
            Value::Array(array) => array.release_into(&mut pending),
            _ => {}
        }
    }
}
