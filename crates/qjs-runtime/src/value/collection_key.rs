//! Hashing for `Map` and `Set` keys.
//!
//! Both collections compare keys with SameValueZero, which is neither Rust's
//! `PartialEq` for [`Value`] nor plain bit equality: `NaN` matches itself,
//! `+0` matches `-0`, strings match by UTF-16 code unit, and everything else
//! matches by reference. [`CollectionKey`] wraps a key with exactly those
//! semantics plus a hash consistent with them, so the collections can index
//! their insertion-ordered storage instead of scanning it.

use std::hash::{Hash, Hasher};

use super::Value;
use crate::string::surrogate_escape_code_unit;

/// A `Map`/`Set` key compared and hashed by SameValueZero.
#[derive(Clone, Debug)]
pub(crate) struct CollectionKey(Value);

impl CollectionKey {
    pub(crate) fn new(value: &Value) -> Self {
        Self(value.clone())
    }
}

impl PartialEq for CollectionKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_value_zero(&other.0)
    }
}

impl Eq for CollectionKey {}

impl Hash for CollectionKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            Value::Number(number) => {
                0_u8.hash(state);
                // Every NaN is one key, and the two zeroes are one key.
                let bits = if number.is_nan() {
                    f64::NAN.to_bits()
                } else if *number == 0.0 {
                    0_f64.to_bits()
                } else {
                    number.to_bits()
                };
                bits.hash(state);
            }
            Value::String(value) => {
                1_u8.hash(state);
                // Two buffers that differ in UTF-8 can still be the same
                // JavaScript string — an escaped surrogate pair and the scalar
                // it encodes — so the hash runs over code units, matching
                // `js_string_eq`.
                if value.is_ascii() {
                    state.write(value.as_bytes());
                } else {
                    for character in value.chars() {
                        match surrogate_escape_code_unit(character) {
                            Some(unit) => state.write_u16(unit),
                            None => {
                                let mut buffer = [0; 2];
                                for unit in character.encode_utf16(&mut buffer) {
                                    state.write_u16(*unit);
                                }
                            }
                        }
                    }
                }
                state.write_u8(0xff);
            }
            Value::Boolean(value) => {
                2_u8.hash(state);
                value.hash(state);
            }
            Value::Null => 3_u8.hash(state),
            Value::Undefined => 4_u8.hash(state),
            Value::BigInt(value) => {
                5_u8.hash(state);
                value.hash(state);
            }
            Value::Object(value) => reference_hash(6, value.address(), state),
            Value::Array(value) => reference_hash(7, value.address(), state),
            Value::Function(value) => reference_hash(8, value.address(), state),
            Value::Map(value) => reference_hash(9, value.address(), state),
            Value::Set(value) => reference_hash(10, value.address(), state),
            Value::Proxy(value) => reference_hash(11, value.address(), state),
        }
    }
}

fn reference_hash<H: Hasher>(tag: u8, address: usize, state: &mut H) {
    tag.hash(state);
    address.hash(state);
}

#[cfg(test)]
mod tests {
    use super::CollectionKey;
    use crate::{JsString, Value};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash(value: &Value) -> u64 {
        let mut hasher = DefaultHasher::new();
        CollectionKey::new(value).hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn same_value_zero_keys_hash_alike() {
        assert_eq!(hash(&Value::Number(0.0)), hash(&Value::Number(-0.0)));
        assert_eq!(
            hash(&Value::Number(f64::NAN)),
            hash(&Value::Number(-f64::NAN))
        );
        assert_eq!(
            CollectionKey::new(&Value::Number(f64::NAN)),
            CollectionKey::new(&Value::Number(f64::NAN))
        );
        assert_eq!(
            CollectionKey::new(&Value::Number(0.0)),
            CollectionKey::new(&Value::Number(-0.0))
        );
    }

    #[test]
    fn strings_that_share_code_units_share_a_key() {
        let pair = Value::String(JsString::from("\u{1F600}"));
        let halves = Value::String(JsString::from(crate::string::string_from_code_units(&[
            0xD83D, 0xDE00,
        ])));
        assert_eq!(CollectionKey::new(&pair), CollectionKey::new(&halves));
        assert_eq!(hash(&pair), hash(&halves));
    }

    #[test]
    fn distinct_primitives_stay_distinct() {
        assert_ne!(
            CollectionKey::new(&Value::Number(1.0)),
            CollectionKey::new(&Value::Boolean(true))
        );
        assert_ne!(
            CollectionKey::new(&Value::Null),
            CollectionKey::new(&Value::Undefined)
        );
        assert_ne!(
            CollectionKey::new(&Value::String(JsString::from("a"))),
            CollectionKey::new(&Value::String(JsString::from("b")))
        );
    }
}
