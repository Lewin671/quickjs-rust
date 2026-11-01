//! VM storage is selected by type before execution. Synchronous frames are
//! values; suspendable frames keep one stable box from prologue to completion.
//! Monomorphization avoids a storage-tag test on each interpreter operation.
use super::vm::{FrameState, Vm};
use std::ops::{Deref, DerefMut};

pub(super) trait VmFrame<'a>: Deref<Target = FrameState<'a>> + DerefMut {}
pub(super) struct InlineFrame<'a>(pub(super) FrameState<'a>);
impl<'a> Deref for InlineFrame<'a> {
    type Target = FrameState<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for InlineFrame<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<'a> VmFrame<'a> for InlineFrame<'a> {}
impl<'a> VmFrame<'a> for Box<FrameState<'a>> {}
impl<'a, 's> Vm<'a, 's> {
    pub(super) fn into_suspendable(self) -> Vm<'a, 's, Box<FrameState<'a>>> {
        debug_assert!(self.callers.is_empty() && self.pending_frame_entry.is_none());
        Vm {
            runtime: self.runtime,
            current: Box::new(self.current.0),
            callers: Vec::new(),
            pending_frame_entry: None,
            frame_lifetime: std::marker::PhantomData,
        }
    }
}
