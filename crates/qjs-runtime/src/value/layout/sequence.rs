//! Parse-local ordered data construction. Prediction retains only string schema
//! metadata; a mismatch resumes generic insertion from the completed prefix.
use super::*;

pub(crate) struct DataSequence {
    phase: Phase,
    seen: usize,
    indices: usize,
}
enum Phase {
    Predicted {
        layout: Rc<Layout>,
        values: ValueSlots,
    },
    Generic(NamedStore),
}
impl DataSequence {
    pub(crate) fn new(
        prediction: Option<Rc<Layout>>,
        domain: &AtomDomain,
        prototype: Option<crate::Prototype>,
    ) -> Self {
        let phase = if let Some(layout) = prediction {
            let mut values = ValueSlots::Empty;
            values.reserve(layout.len);
            Phase::Predicted { layout, values }
        } else {
            Phase::Generic(NamedStore::empty_with_prototype(domain, prototype, 0))
        };
        Self {
            phase,
            seen: 0,
            indices: 0,
        }
    }
    pub(crate) fn insert(&mut self, key: &str, value: Value) {
        if let Phase::Predicted { layout, values } = &mut self.phase {
            let matches = layout
                .keys()
                .get(values.len())
                .filter(|expected| expected.as_str() == key)
                .map(|expected| expected.array_index().is_some());
            if let Some(index) = matches {
                values.push(value);
                self.indices += usize::from(index);
                self.seen += 1;
                return;
            }
            let mut prefix: &Rc<Layout> = layout;
            while prefix.len > values.len() {
                prefix = prefix.parent.as_ref().expect("append-only prediction");
            }
            let prefix = prefix.clone();
            self.phase = Phase::Generic(NamedStore::from_slots(
                prefix,
                std::mem::replace(values, ValueSlots::Empty),
            ));
        }
        let Phase::Generic(store) = &mut self.phase else {
            unreachable!()
        };
        if store.insert_text(key, Property::enumerable(value))
            && crate::value::array_index_property_key(key).is_some()
        {
            self.indices += 1;
        }
        self.seen += 1;
    }
    pub(crate) fn finish(self) -> (NamedStore, Option<Rc<Layout>>, usize) {
        let (store, prediction) = match self.phase {
            Phase::Predicted { layout, values } => {
                let mut layout = layout;
                while layout.len > values.len() {
                    layout = layout.parent.as_ref().expect("prefix").clone();
                }
                (NamedStore::from_slots(layout.clone(), values), Some(layout))
            }
            Phase::Generic(store) => {
                let prediction = (store.len() == self.seen
                    && !store.layout.dictionary
                    && store.domain().ic_policy().enabled)
                    .then(|| store.layout.clone());
                (store, prediction)
            }
        };
        (store, prediction, self.indices)
    }
}
