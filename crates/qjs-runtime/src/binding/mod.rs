//! Binding metadata shared by compiled plans and live declarative locations.
//!
//! A binding ID is local to its retained plan. Descriptors contain names and
//! routes only; values belong exclusively to frame slots or shared cells.

mod cell;
mod plan;
mod scope;

pub(crate) use cell::BindingCell;
pub(crate) use plan::*;
pub(crate) use scope::*;

#[cfg(test)]
mod scope_tests;

mod table;
pub(crate) use table::DynamicBindings;

mod global_reference;
pub(crate) use global_reference::GlobalReference;
