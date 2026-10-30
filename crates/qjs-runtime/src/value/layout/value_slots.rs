//! Owning value slots, inline through width two. Descriptor/key/order metadata
//! remains exclusively in Layout. Moving between variants preserves ownership
//! and slot order; heap stores keep capacity after removals to avoid churn.
use crate::Value;
use std::ops::{Deref, DerefMut};

pub(super) enum ValueSlots {
    Empty,
    One(Value),
    Two([Value; 2]),
    Heap(Vec<Value>),
}
impl ValueSlots {
    pub(super) fn from_vec(mut values: Vec<Value>) -> Self {
        match values.len() {
            0 => Self::Empty,
            1 => Self::One(values.pop().expect("one slot")),
            2 => {
                let second = values.pop().expect("second slot");
                Self::Two([values.pop().expect("first slot"), second])
            }
            _ => Self::Heap(values),
        }
    }
    pub(super) fn pair(values: [Value; 2]) -> Self {
        Self::Two(values)
    }
    pub(super) fn reserve(&mut self, additional: usize) {
        let needed = self
            .len()
            .checked_add(additional)
            .expect("slot capacity overflow");
        if needed <= self.capacity() {
            return;
        }
        if let Self::Heap(values) = self {
            values.reserve(additional);
            return;
        }
        let old = std::mem::replace(self, Self::Empty);
        let mut values = Vec::with_capacity(needed);
        match old {
            Self::Empty => {}
            Self::One(value) => values.push(value),
            Self::Two(pair) => values.extend(pair),
            Self::Heap(_) => unreachable!("heap handled before inline growth"),
        }
        *self = Self::Heap(values);
    }
    pub(super) fn push(&mut self, value: Value) {
        *self = match std::mem::replace(self, Self::Empty) {
            Self::Empty => Self::One(value),
            Self::One(first) => Self::Two([first, value]),
            Self::Two(pair) => {
                let mut values = Vec::with_capacity(4);
                values.extend(pair);
                values.push(value);
                Self::Heap(values)
            }
            Self::Heap(mut values) => {
                values.push(value);
                Self::Heap(values)
            }
        };
    }
    pub(super) fn pop(&mut self) -> Option<Value> {
        let (next, value) = match std::mem::replace(self, Self::Empty) {
            Self::Empty => (Self::Empty, None),
            Self::One(value) => (Self::Empty, Some(value)),
            Self::Two([first, second]) => (Self::One(first), Some(second)),
            Self::Heap(mut values) => {
                let value = values.pop();
                (Self::Heap(values), value)
            }
        };
        *self = next;
        value
    }
    pub(super) fn swap_remove(&mut self, index: usize) -> Value {
        match self {
            Self::Heap(values) => values.swap_remove(index),
            _ => self.remove(index), // removing either of two slots has the same order
        }
    }
    pub(super) fn remove(&mut self, index: usize) -> Value {
        assert!(index < self.len(), "valid slot removal");
        let (next, value) = match std::mem::replace(self, Self::Empty) {
            Self::Empty => unreachable!("checked slot"),
            Self::One(value) => (Self::Empty, value),
            Self::Two([first, second]) if index == 0 => (Self::One(second), first),
            Self::Two([first, second]) => (Self::One(first), second),
            Self::Heap(mut values) => {
                let value = values.remove(index);
                (Self::Heap(values), value)
            }
        };
        *self = next;
        value
    }
    pub(super) fn capacity(&self) -> usize {
        match self {
            Self::Heap(values) => values.capacity(),
            _ => 2,
        }
    }
}
impl Deref for ValueSlots {
    type Target = [Value];
    fn deref(&self) -> &[Value] {
        match self {
            Self::Empty => &[],
            Self::One(value) => std::slice::from_ref(value),
            Self::Two(values) => values,
            Self::Heap(values) => values,
        }
    }
}
impl DerefMut for ValueSlots {
    fn deref_mut(&mut self) -> &mut [Value] {
        match self {
            Self::Empty => &mut [],
            Self::One(value) => std::slice::from_mut(value),
            Self::Two(values) => values,
            Self::Heap(values) => values,
        }
    }
}
impl<'a> IntoIterator for &'a mut ValueSlots {
    type Item = &'a mut Value;
    type IntoIter = std::slice::IterMut<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ValueSlots>() == 32);
