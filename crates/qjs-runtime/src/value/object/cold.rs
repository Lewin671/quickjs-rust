//! Typed private payloads keep spec-internal scalars out of named properties.
//! OnceCell payload kind never changes; optional general extensions stay stable.
use super::*;
pub(super) enum ColdPayload {
    General(Box<ObjectColdData>),
    Date(Box<DateData>),
}
pub(super) struct DateData {
    value: Cell<f64>,
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
        }
    }
    pub(super) fn cold_if_present(&self) -> Option<&ObjectColdData> {
        match self.cold.get()? {
            ColdPayload::General(data) => Some(data),
            ColdPayload::Date(data) => data.extra.get().map(Box::as_ref),
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
            ColdPayload::General(_) => panic!("Date branding requires a fresh receiver"),
        }
    }
    #[inline(always)]
    pub(crate) fn date_value(&self) -> Option<f64> {
        match self.0.cold.get()? {
            ColdPayload::Date(data) => Some(data.value.get()),
            ColdPayload::General(_) => None,
        }
    }
}
