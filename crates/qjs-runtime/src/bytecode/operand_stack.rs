//! Owned operand values and a data-only cleared-storage pool handle.
//! No bytecode or service borrow survives with an activation.
use crate::Value;
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub(super) struct OperandStack {
    pool: Rc<super::frame_pool::GeneralPool>,
    values: Vec<Value>,
}

impl OperandStack {
    pub(super) fn new(pool: Rc<super::frame_pool::GeneralPool>) -> Self {
        let values = pool.take_operands();
        Self { pool, values }
    }

    pub(super) fn pool(&self) -> &super::frame_pool::GeneralPool {
        &self.pool
    }

    pub(super) fn take(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.values)
    }
}

impl Deref for OperandStack {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for OperandStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl Drop for OperandStack {
    fn drop(&mut self) {
        self.pool.put_operands(std::mem::take(&mut self.values));
    }
}
