//! Unobserved ordered construction. Predictions validate each key/descriptor;
//! a mismatch resumes common insertion from exactly the completed prefix.
//! Predictions come only from unique append-only construction, before exposure.
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
        accessors: Option<Box<AccessorSlots>>,
    },
    Generic(NamedStore),
}
#[derive(Clone, Copy)]
enum Key<'a> {
    Text(&'a str),
    Symbol(&'a crate::ObjectRef),
}
impl DataSequence {
    pub(crate) fn new(
        prediction: Option<Rc<Layout>>,
        domain: &AtomDomain,
        prototype: Option<crate::Prototype>,
    ) -> Self {
        let prediction = prediction.filter(|layout| {
            domain.ic_policy().enabled
                && layout.domain.ptr_eq(domain)
                && !layout.dictionary
                && layout.matches_prototype(prototype.as_ref(), 0)
        });
        let phase = if let Some(layout) = prediction {
            let mut values = ValueSlots::Empty;
            values.reserve(layout.len);
            Phase::Predicted {
                layout,
                values,
                accessors: None,
            }
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
        self.insert_property(key, Property::enumerable(value));
    }
    pub(crate) fn insert_property(&mut self, key: &str, property: Property) {
        self.define(Key::Text(key), property);
    }
    pub(crate) fn insert_symbol(&mut self, key: &crate::ObjectRef, property: Property) {
        self.define(Key::Symbol(key), property);
    }
    fn define(&mut self, key: Key<'_>, property: Property) {
        if let Phase::Predicted {
            layout,
            values,
            accessors,
        } = &mut self.phase
        {
            let matches = {
                let data = layout.data.borrow();
                let position = values.len();
                data.keys
                    .get(position)
                    .filter(|expected| {
                        position < layout.len
                            && match key {
                                Key::Text(text) => expected.string_text() == Some(text),
                                Key::Symbol(symbol) => {
                                    expected.symbol().is_some_and(|s| s.ptr_eq(symbol))
                                }
                            }
                            && data.entries[position].attrs == Attributes::of(&property)
                    })
                    .map(|expected| expected.array_index().is_some())
            };
            if let Some(index) = matches {
                if let Some(pair) = property.shared_accessors() {
                    accessors
                        .get_or_insert_with(|| Box::new(HashMap::new()))
                        .insert(values.len() as u32, pair);
                    values.push(Value::Undefined);
                } else {
                    values.push(property.value);
                }
                self.indices += usize::from(index);
                self.seen += 1;
                return;
            }
            let mut prefix: &Rc<Layout> = layout;
            while prefix.len > values.len() {
                prefix = prefix.parent.as_ref().expect("append-only prediction");
            }
            self.phase = Phase::Generic(NamedStore::from_slot_parts(
                prefix.clone(),
                std::mem::replace(values, ValueSlots::Empty),
                accessors.take(),
            ));
        }
        let Phase::Generic(store) = &mut self.phase else {
            unreachable!()
        };
        let (added, index) = match key {
            Key::Text(text) => (
                store.insert_text(text, property),
                crate::value::array_index_property_key(text).is_some(),
            ),
            Key::Symbol(symbol) => {
                let key = store.domain().intern_symbol(symbol);
                let lookup = store.lookup(&key).expect("own domain");
                let added = lookup.position.is_none();
                lookup.define(property);
                (added, false)
            }
        };
        self.indices += usize::from(added && index);
        self.seen += 1;
    }
    /// Finish without acquiring another layout handle for an unused prediction.
    pub(crate) fn finish_store(self) -> (NamedStore, usize) {
        let store = match self.phase {
            Phase::Predicted {
                mut layout,
                values,
                accessors,
            } => {
                while layout.len > values.len() {
                    layout = layout.parent.as_ref().expect("prefix").clone();
                }
                NamedStore::from_slot_parts(layout, values, accessors)
            }
            Phase::Generic(store) => store,
        };
        (store, self.indices)
    }
    pub(crate) fn finish(self) -> (NamedStore, Option<Rc<Layout>>, usize) {
        let seen = self.seen;
        let (store, indices) = self.finish_store();
        let prediction =
            (store.len() == seen && !store.layout.dictionary && store.domain().ic_policy().enabled)
                .then(|| store.layout.clone());
        (store, prediction, indices)
    }
}
