//! Binding metadata shared by compiled plans and live declarative locations.
//!
//! A binding ID is local to its retained plan. Descriptors contain names and
//! routes only; values belong exclusively to frame slots or shared cells.

mod cell;
mod plan;

pub(crate) use cell::BindingCell;
pub(crate) use plan::*;
