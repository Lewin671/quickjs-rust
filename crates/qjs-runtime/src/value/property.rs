use std::rc::Rc;

use super::Value;

#[derive(Clone, Debug)]
struct AccessorState {
    get: Option<Value>,
    set: Option<Value>,
}

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) value: Value,
    // Property lookup clones descriptors frequently, so share the cold state
    // and copy it only when a descriptor mutation changes an accessor half.
    accessors: Option<Rc<AccessorState>>,
    pub(crate) enumerable: bool,
    pub(crate) writable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    pub(crate) fn data(value: Value, enumerable: bool, writable: bool, configurable: bool) -> Self {
        Self {
            value,
            accessors: None,
            enumerable,
            writable,
            configurable,
        }
    }

    pub(crate) fn accessor(
        get: Option<Value>,
        set: Option<Value>,
        enumerable: bool,
        configurable: bool,
    ) -> Self {
        Self {
            value: Value::Undefined,
            accessors: Some(Rc::new(AccessorState { get, set })),
            enumerable,
            writable: false,
            configurable,
        }
    }

    pub(crate) fn enumerable(value: Value) -> Self {
        Self {
            value,
            accessors: None,
            enumerable: true,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn non_enumerable(value: Value) -> Self {
        Self {
            value,
            accessors: None,
            enumerable: false,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn fixed_non_enumerable(value: Value) -> Self {
        Self::data(value, false, false, false)
    }

    pub(crate) fn is_accessor(&self) -> bool {
        self.accessors.is_some()
    }

    pub(crate) fn getter(&self) -> Option<&Value> {
        self.accessors
            .as_deref()
            .and_then(|accessor| accessor.get.as_ref())
    }

    pub(crate) fn setter(&self) -> Option<&Value> {
        self.accessors
            .as_deref()
            .and_then(|accessor| accessor.set.as_ref())
    }

    #[cfg(test)]
    fn accessor_parts(&self) -> Option<(Option<&Value>, Option<&Value>)> {
        self.accessors
            .as_deref()
            .map(|accessor| (accessor.get.as_ref(), accessor.set.as_ref()))
    }

    pub(crate) fn into_accessor_parts(self) -> Option<(Option<Value>, Option<Value>)> {
        self.accessors
            .map(|accessors| match Rc::try_unwrap(accessors) {
                Ok(accessors) => (accessors.get, accessors.set),
                Err(accessors) => (accessors.get.clone(), accessors.set.clone()),
            })
    }

    pub(crate) fn merge_missing_accessor_halves(&mut self, existing: Self) {
        let (existing_get, existing_set) = existing
            .into_accessor_parts()
            .expect("only accessor properties can provide accessor halves");
        let accessors = Rc::make_mut(
            self.accessors
                .as_mut()
                .expect("only accessor properties can merge accessor halves"),
        );
        if accessors.get.is_none() {
            accessors.get = existing_get;
        }
        if accessors.set.is_none() {
            accessors.set = existing_set;
        }
    }

    pub(crate) fn set_getter(&mut self, get: Option<Value>) {
        Rc::make_mut(
            self.accessors
                .as_mut()
                .expect("only accessor properties have a getter slot"),
        )
        .get = get;
    }

    pub(crate) fn set_setter(&mut self, set: Option<Value>) {
        Rc::make_mut(
            self.accessors
                .as_mut()
                .expect("only accessor properties have a setter slot"),
        )
        .set = set;
    }

    pub(crate) fn make_non_configurable(&mut self) {
        self.configurable = false;
    }

    pub(crate) fn make_non_writable(&mut self) {
        self.writable = false;
    }

    pub(crate) fn freeze_data(&mut self) {
        self.make_non_configurable();
        self.make_non_writable();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Property;
    use crate::Value;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn property_stays_within_four_machine_words() {
        assert_eq!(std::mem::size_of::<Property>(), 32);
    }

    #[test]
    fn data_property_keeps_accessor_storage_cold() {
        let property = Property::data(Value::Number(3.0), true, true, false);

        assert!(!property.is_accessor());
        assert!(property.accessors.is_none());
        assert_eq!(property.value, Value::Number(3.0));
        assert_eq!(property.accessor_parts(), None);
    }

    #[test]
    fn accessor_property_preserves_both_halves_and_data_invariants() {
        let getter = Value::Number(1.0);
        let setter = Value::Number(2.0);
        let mut property = Property::accessor(Some(getter.clone()), None, true, true);

        assert!(property.is_accessor());
        assert_eq!(property.value, Value::Undefined);
        assert!(!property.writable);
        assert_eq!(property.getter(), Some(&getter));
        assert_eq!(property.setter(), None);

        property.set_setter(Some(setter.clone()));
        assert_eq!(
            property.accessor_parts(),
            Some((Some(&getter), Some(&setter)))
        );
    }

    #[test]
    fn empty_accessor_remains_distinct_from_a_data_property() {
        let property = Property::accessor(None, None, false, false);

        assert!(property.is_accessor());
        assert_eq!(property.accessor_parts(), Some((None, None)));
        assert_eq!(property.value, Value::Undefined);
        assert!(!property.writable);
    }

    #[test]
    fn accessor_merge_only_fills_missing_halves() {
        let kept_getter = Value::Number(1.0);
        let inherited_getter = Value::Number(2.0);
        let inherited_setter = Value::Number(3.0);
        let mut property = Property::accessor(Some(kept_getter.clone()), None, true, true);

        property.merge_missing_accessor_halves(Property::accessor(
            Some(inherited_getter),
            Some(inherited_setter.clone()),
            true,
            true,
        ));

        assert_eq!(
            property.accessor_parts(),
            Some((Some(&kept_getter), Some(&inherited_setter)))
        );
    }

    #[test]
    fn accessor_clone_shares_cold_state_until_mutation() {
        let getter = Value::Number(1.0);
        let setter = Value::Number(2.0);
        let mut property = Property::accessor(Some(getter.clone()), None, true, true);
        let cloned = property.clone();

        assert!(Rc::ptr_eq(
            property.accessors.as_ref().unwrap(),
            cloned.accessors.as_ref().unwrap()
        ));

        property.set_setter(Some(setter.clone()));

        assert!(!Rc::ptr_eq(
            property.accessors.as_ref().unwrap(),
            cloned.accessors.as_ref().unwrap()
        ));
        assert_eq!(
            property.accessor_parts(),
            Some((Some(&getter), Some(&setter)))
        );
        assert_eq!(cloned.accessor_parts(), Some((Some(&getter), None)));
    }
}
