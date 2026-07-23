use std::rc::Rc;

use crate::{
    NativeFunction, ObjectRef, Value,
    value::{ArrayRef, OwnDataPropertyRead},
};

use super::super::super::vm::Vm;

#[derive(Clone, Debug)]
pub(super) enum DynamicLimit {
    LocalNumber(usize),
    LocalArrayLength(usize),
    OwnDataNumber(OwnDataSource),
}

impl DynamicLimit {
    pub(super) fn number_slot(&self) -> Option<usize> {
        match self {
            Self::LocalNumber(slot) => Some(*slot),
            Self::LocalArrayLength(_) | Self::OwnDataNumber(_) => None,
        }
    }

    pub(super) fn array_length_slot(&self) -> Option<usize> {
        match self {
            Self::LocalArrayLength(slot) => Some(*slot),
            Self::LocalNumber(_) | Self::OwnDataNumber(_) => None,
        }
    }

    pub(super) fn required_slot(&self) -> Option<usize> {
        match self {
            Self::LocalNumber(slot) | Self::LocalArrayLength(slot) => Some(*slot),
            Self::OwnDataNumber(source) => source.owner.local_slot(),
        }
    }

    pub(super) fn additional_authority_slot(&self) -> Option<usize> {
        match self {
            // Local Number limits are remapped into `local_slots`; those
            // original VM slots are already checked by the primary chain.
            Self::LocalNumber(_) => None,
            Self::LocalArrayLength(slot) => Some(*slot),
            Self::OwnDataNumber(source) => source.owner.local_slot(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OwnDataOwner {
    DirectThis,
    Local(usize),
}

impl OwnDataOwner {
    pub(super) fn local_slot(&self) -> Option<usize> {
        match self {
            Self::DirectThis => None,
            Self::Local(slot) => Some(*slot),
        }
    }

    fn resolve(&self, vm: &Vm<'_>) -> Option<ObjectRef> {
        let value = match self {
            Self::DirectThis => vm.direct_this.as_ref()?.clone(),
            Self::Local(slot) => vm.local_slot_value(*slot)?,
        };
        let Value::Object(object) = value else {
            return None;
        };
        if crate::symbol::is_symbol_primitive(&object)
            || crate::typed_array::is_typed_array_object(&object)
            || object.is_module_namespace_exotic()
        {
            return None;
        }
        Some(object)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OwnDataSource {
    pub(super) owner: OwnDataOwner,
    pub(super) key: Rc<str>,
}

impl OwnDataSource {
    fn resolve_value(&self, vm: &Vm<'_>) -> Option<Value> {
        match self.owner.resolve(vm)?.own_data_property_read(&self.key) {
            OwnDataPropertyRead::Data(value) => Some(value),
            OwnDataPropertyRead::Missing | OwnDataPropertyRead::NeedsSlowPath => None,
        }
    }

    pub(super) fn resolve_number(&self, vm: &Vm<'_>) -> Option<f64> {
        match self.resolve_value(vm)? {
            Value::Number(value) => Some(value),
            _ => None,
        }
    }

    fn resolve_array(&self, vm: &Vm<'_>) -> Option<ArrayRef> {
        match self.resolve_value(vm)? {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ArraySource {
    Local(usize),
    OwnData(OwnDataSource),
}

impl ArraySource {
    pub(super) fn local_slot(&self) -> Option<usize> {
        match self {
            Self::Local(slot) => Some(*slot),
            Self::OwnData(source) => source.owner.local_slot(),
        }
    }

    pub(super) fn resolve(&self, vm: &Vm<'_>) -> Option<ArrayRef> {
        match self {
            Self::Local(slot) => match vm.locals.get(*slot) {
                Some(Some(Value::Array(array))) => Some(array.clone()),
                _ => None,
            },
            Self::OwnData(source) => source.resolve_array(vm),
        }
    }
}

pub(super) fn native_math_round_is_current(vm: &Vm<'_>) -> bool {
    let Some(Value::Object(math)) = vm.env.get("Math") else {
        return false;
    };
    if crate::symbol::is_symbol_primitive(&math)
        || crate::typed_array::is_typed_array_object(&math)
        || math.is_module_namespace_exotic()
    {
        return false;
    }
    let Some(global_this) = vm.cached_global_this() else {
        return false;
    };
    let OwnDataPropertyRead::Data(Value::Object(global_math)) =
        global_this.own_data_property_read("Math")
    else {
        return false;
    };
    if !global_math.ptr_eq(&math) {
        return false;
    }
    let OwnDataPropertyRead::Data(Value::Function(round)) = math.own_data_property_read("round")
    else {
        return false;
    };
    round.native_kind() == Some(NativeFunction::MathRound)
}
