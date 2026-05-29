use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use super::{ObjectRef, Value};

/// Array storage reference.
#[derive(Clone)]
pub struct ArrayRef {
    elements: Rc<RefCell<Vec<Value>>>,
    extensible: Rc<Cell<bool>>,
    sealed: Rc<Cell<bool>>,
    frozen: Rc<Cell<bool>>,
    prototype: Rc<RefCell<Option<Option<ObjectRef>>>>,
}

impl ArrayRef {
    pub(crate) fn new(elements: Vec<Value>) -> Self {
        Self {
            elements: Rc::new(RefCell::new(elements)),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            prototype: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.elements, &other.elements)
    }

    pub(crate) fn len(&self) -> usize {
        self.elements.borrow().len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.elements.borrow().is_empty()
    }

    pub(crate) fn get(&self, index: usize) -> Option<Value> {
        self.elements.borrow().get(index).cloned()
    }

    pub(crate) fn to_vec(&self) -> Vec<Value> {
        self.elements.borrow().clone()
    }

    pub(crate) fn push(&self, value: Value) -> usize {
        let mut elements = self.elements.borrow_mut();
        elements.push(value);
        elements.len()
    }

    pub(crate) fn pop(&self) -> Option<Value> {
        self.elements.borrow_mut().pop()
    }

    pub(crate) fn shift(&self) -> Option<Value> {
        let mut elements = self.elements.borrow_mut();
        if elements.is_empty() {
            None
        } else {
            Some(elements.remove(0))
        }
    }

    pub(crate) fn unshift(&self, values: &[Value]) -> usize {
        let mut elements = self.elements.borrow_mut();
        elements.splice(0..0, values.iter().cloned());
        elements.len()
    }

    pub(crate) fn reverse(&self) {
        self.elements.borrow_mut().reverse();
    }

    pub(crate) fn replace_with(&self, values: Vec<Value>) {
        if self.frozen.get() {
            return;
        }
        if values.len() > self.elements.borrow().len() && !self.extensible.get() {
            return;
        }
        *self.elements.borrow_mut() = values;
    }

    pub(crate) fn fill(&self, start: usize, end: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        for element in &mut elements[start..end] {
            *element = value.clone();
        }
    }

    pub(crate) fn set(&self, index: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        if index >= elements.len() {
            if !self.extensible.get() {
                return;
            }
            elements.resize(index + 1, Value::Undefined);
        }
        if self.frozen.get() {
            return;
        }
        elements[index] = value;
    }

    pub(crate) fn set_len(&self, length: usize) {
        let mut elements = self.elements.borrow_mut();
        if self.frozen.get() {
            return;
        }
        if length > elements.len() && !self.extensible.get() {
            return;
        }
        elements.resize(length, Value::Undefined);
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        self.sealed.set(true);
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get() && self.sealed.get()
    }

    pub(crate) fn freeze(&self) {
        self.seal();
        self.frozen.set(true);
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.extensible.get() && self.sealed.get() && self.frozen.get()
    }

    pub(crate) fn prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.prototype.borrow().clone()
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        if matches!(
            self.prototype.borrow().as_ref(),
            Some(current) if same_prototype(current, &prototype)
        ) {
            return Ok(());
        }
        if !self.extensible.get() {
            return Err(());
        }
        *self.prototype.borrow_mut() = Some(prototype);
        Ok(())
    }
}

impl fmt::Debug for ArrayRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArrayRef")
            .field("len", &self.elements.borrow().len())
            .finish()
    }
}

fn same_prototype(left: &Option<ObjectRef>, right: &Option<ObjectRef>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}
