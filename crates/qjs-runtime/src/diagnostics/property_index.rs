//! Diagnostic property maps preserve the production key hash/hasher and table
//! operations while exposing equality calls. Owned query keys are diagnostic
//! overhead, excluded from JS payload counts. The disabled aliases are the
//! original maps, with no wrapper or extra query allocation.
use super::mechanism::{PropertyScope, name_eq};
use crate::{Property, value::name_hash::NameHasher};
use std::collections::{HashMap, hash_map::RandomState};
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct Key<K>(K);
impl<K: AsRef<str>> PartialEq for Key<K> {
    fn eq(&self, other: &Self) -> bool {
        name_eq(self.0.as_ref(), other.0.as_ref())
    }
}
impl<K: AsRef<str>> Eq for Key<K> {}
impl<K: Hash> Hash for Key<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug)]
pub(crate) struct PropertyMap<K, V, S>(HashMap<Key<K>, V, S>);
impl<K, V, S: Default> Default for PropertyMap<K, V, S> {
    fn default() -> Self {
        Self(HashMap::with_hasher(S::default()))
    }
}
impl<K, V, S> PropertyMap<K, V, S>
where
    K: AsRef<str> + Hash + for<'a> From<&'a str>,
    S: BuildHasher,
{
    pub(crate) fn with_capacity(capacity: usize) -> Self
    where
        S: Default,
    {
        Self(HashMap::with_capacity_and_hasher(capacity, S::default()))
    }
    pub(crate) fn entry(&mut self, key: K) -> PropertyEntry<'_, K, V> {
        let scope = PropertyScope::enter();
        PropertyEntry {
            entry: self.0.entry(Key(key)),
            _scope: scope,
        }
    }
    pub(crate) fn get(&self, key: &str) -> Option<&V> {
        let _scope = PropertyScope::enter();
        self.0.get(&Key(K::from(key)))
    }
    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        let _scope = PropertyScope::enter();
        self.0.get_mut(&Key(K::from(key)))
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        let _scope = PropertyScope::enter();
        self.0.insert(Key(key), value)
    }
    pub(crate) fn remove(&mut self, key: &str) -> Option<V> {
        let _scope = PropertyScope::enter();
        self.0.remove(&Key(K::from(key)))
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn keys(&self) -> impl ExactSizeIterator<Item = &K> {
        self.0.keys().map(|key| &key.0)
    }
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (&K, &V)> {
        self.0.iter().map(|(key, value)| (&key.0, value))
    }
    pub(crate) fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = (&K, &mut V)> {
        self.0.iter_mut().map(|(key, value)| (&key.0, value))
    }
    pub(crate) fn values(&self) -> impl ExactSizeIterator<Item = &V> {
        self.0.values()
    }
    pub(crate) fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut V> {
        self.0.values_mut()
    }
}
impl<K, V, S> FromIterator<(K, V)> for PropertyMap<K, V, S>
where
    K: AsRef<str> + Hash,
    S: BuildHasher + Default,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let _scope = PropertyScope::enter();
        // Preserve collect's size hint/reservation, including its rehash count.
        Self(
            iter.into_iter()
                .map(|(key, value)| (Key(key), value))
                .collect(),
        )
    }
}

pub(crate) type PropertyIndex = PropertyMap<Rc<str>, usize, BuildHasherDefault<NameHasher>>;
pub(crate) type FunctionPropertyMap = PropertyMap<String, Property, BuildHasherDefault<NameHasher>>;
pub(crate) type ArrayPropertyMap = PropertyMap<String, Property, TracedRandomState>;

/// Arrays retain std's independently seeded hasher, not NameHasher.
#[derive(Debug, Default)]
pub(crate) struct TracedRandomState(RandomState);
impl BuildHasher for TracedRandomState {
    type Hasher = TracedHasher<<RandomState as BuildHasher>::Hasher>;
    fn build_hasher(&self) -> Self::Hasher {
        TracedHasher(self.0.build_hasher())
    }
}
pub(crate) struct TracedHasher<H>(H);
impl<H: Hasher> Hasher for TracedHasher<H> {
    fn finish(&self) -> u64 {
        super::mechanism::name_hash();
        self.0.finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        super::mechanism::name_hash_bytes(bytes.len());
        self.0.write(bytes);
    }
    fn write_u8(&mut self, value: u8) {
        self.0.write_u8(value);
    }
}

pub(crate) struct PropertyEntry<'a, K, V> {
    entry: std::collections::hash_map::Entry<'a, Key<K>, V>,
    _scope: PropertyScope,
}
impl<'a, K: AsRef<str> + Hash, V> PropertyEntry<'a, K, V> {
    pub(crate) fn or_insert_with(self, default: impl FnOnce() -> V) -> &'a mut V {
        self.entry.or_insert_with(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counted_keys_and_wrapped_std_hasher_preserve_string_hashes() {
        let state = RandomState::new();
        for name in [
            "",
            "x",
            "prototype",
            "length",
            "a-longer-than-eight-byte-name",
            "\u{1f600}",
            "\u{f0000}",
        ] {
            let plain = state.hash_one(name);
            let mut traced = TracedHasher(state.build_hasher());
            Key(name.to_owned()).hash(&mut traced);
            assert_eq!(plain, traced.finish(), "{name:?}");
            let mut plain = NameHasher::default();
            name.hash(&mut plain);
            let mut wrapped = NameHasher::default();
            Key(Rc::<str>::from(name)).hash(&mut wrapped);
            assert_eq!(plain.finish(), wrapped.finish());
        }
    }
}
