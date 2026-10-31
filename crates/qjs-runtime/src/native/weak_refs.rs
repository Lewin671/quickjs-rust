//! Operation entries for weak refs; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, weak_ref};

pub(super) fn weak_ref(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_ref::native_weak_ref(info.callee, argument_values, info.is_construct, env)
}

pub(super) fn weak_ref_prototype_deref(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_ref::native_weak_ref_prototype_deref(info.this_value.clone())
}
