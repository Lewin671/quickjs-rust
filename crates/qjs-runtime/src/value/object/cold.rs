//! Typed private payloads keep spec-internal scalars out of named properties.
//! OnceCell payload kind never changes; optional general extensions stay stable.
use super::*;
pub(super) enum ColdPayload {
    General(Box<ObjectColdData>),
    Date(Box<DateData>),
    String(Box<StringData>),
}
pub(super) struct DateData {
    value: Cell<f64>,
    extra: OnceCell<Box<ObjectColdData>>,
}
pub(super) struct StringData {
    value: RefCell<crate::JsString>,
    extra: OnceCell<Box<ObjectColdData>>,
}
impl ObjectData {
    pub(super) fn cold(&self) -> &ObjectColdData {
        match self
            .cold
            .get_or_init(|| ColdPayload::General(Box::default()))
        {
            ColdPayload::General(data) => data,
            ColdPayload::Date(data) => data.extra.get_or_init(Box::default),
            ColdPayload::String(data) => data.extra.get_or_init(Box::default),
        }
    }
    pub(super) fn cold_if_present(&self) -> Option<&ObjectColdData> {
        match self.cold.get()? {
            ColdPayload::General(data) => Some(data),
            ColdPayload::Date(data) => data.extra.get().map(Box::as_ref),
            ColdPayload::String(data) => data.extra.get().map(Box::as_ref),
        }
    }
}
impl ObjectRef {
    pub(crate) fn set_date_value(&self, value: f64) {
        match self.0.cold.get_or_init(|| {
            ColdPayload::Date(Box::new(DateData {
                value: Cell::new(value),
                extra: OnceCell::new(),
            }))
        }) {
            ColdPayload::Date(data) => data.value.set(value),
            ColdPayload::General(_) | ColdPayload::String(_) => {
                panic!("Date branding requires a fresh receiver")
            }
        }
    }
    #[inline(always)]
    pub(crate) fn date_value(&self) -> Option<f64> {
        match self.0.cold.get()? {
            ColdPayload::Date(data) => Some(data.value.get()),
            ColdPayload::General(_) | ColdPayload::String(_) => None,
        }
    }
}

impl ObjectRef {
    /// Own private StringData never materializes indexed or general metadata.
    pub(crate) fn string_data(&self) -> Option<crate::JsString> {
        match self.0.cold.get()? {
            ColdPayload::String(data) => Some(data.value.borrow().clone()),
            _ => None,
        }
    }
    pub(crate) fn set_string_data(&self, text: crate::JsString) {
        match self.0.cold.get() {
            Some(ColdPayload::String(data)) => *data.value.borrow_mut() = text,
            Some(_) => panic!("String branding requires a fresh receiver"),
            None => assert!(
                self.0
                    .cold
                    .set(ColdPayload::String(Box::new(StringData {
                        value: RefCell::new(text),
                        extra: OnceCell::new(),
                    })))
                    .is_ok()
            ),
        }
    }
    /// The caller prepared the final String flavor and own length descriptor.
    pub(crate) fn from_string_named(properties: NamedStore, text: crate::JsString) -> Self {
        let object = Self::from_ready_named(properties, 0);
        object.set_string_data(text);
        object.0.set_flag(flags::STRING_INDICES, true);
        object
    }
}
