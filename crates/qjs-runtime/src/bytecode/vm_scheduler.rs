use crate::{RuntimeError, function::PreparedDirectLeafCall};

use super::{vm::Vm, vm_frame::FrameRun, vm_result::Completion};

const MAX_RECYCLED_FRAME_BUFFERS: usize = 64;

impl<'a> Vm<'a> {
    /// Runs the active frame and any eligible ordinary direct-leaf children on
    /// one explicit frame stack. Every other call shape stays in the existing
    /// recursive fallback path.
    pub(super) fn run_completion(&mut self) -> Result<Completion, RuntimeError> {
        loop {
            match self.run_current_frame() {
                Ok(FrameRun::DirectCall) => {
                    let prepared = self
                        .pending_direct_call
                        .take()
                        .expect("direct-call signal requires a prepared-call mailbox value");
                    debug_assert!(self.pending_direct_call.is_none());
                    self.push_direct_leaf_frame(prepared);
                }
                Ok(FrameRun::Complete(completion)) if self.frames.is_empty() => {
                    debug_assert!(
                        self.pending_direct_call.is_none(),
                        "completed frame retained a prepared direct call"
                    );
                    return Ok(completion);
                }
                Ok(FrameRun::Complete(Completion::Return(value))) => {
                    debug_assert!(
                        self.pending_direct_call.is_none(),
                        "completed frame retained a prepared direct call"
                    );
                    self.restore_parent_frame();
                    self.stack.push(value);
                }
                Ok(FrameRun::Complete(_)) => {
                    debug_assert!(
                        self.pending_direct_call.is_none(),
                        "completed frame retained a prepared direct call"
                    );
                    self.restore_error_to_parent(RuntimeError {
                        thrown: None,
                        message: "yield evaluated outside a generator body".to_owned(),
                    })?;
                }
                Err(error) => {
                    debug_assert!(
                        self.pending_direct_call.is_none(),
                        "errored frame retained a prepared direct call"
                    );
                    self.restore_error_to_parent(error)?;
                }
            }
        }
    }

    fn push_direct_leaf_frame(&mut self, prepared: PreparedDirectLeafCall) {
        let child = self.build_direct_leaf_frame(prepared);
        let parent = std::mem::replace(&mut self.current, child);
        self.frames.push(parent);
    }

    fn restore_error_to_parent(&mut self, mut error: RuntimeError) -> Result<(), RuntimeError> {
        loop {
            if self.frames.is_empty() {
                return Err(error);
            }
            self.restore_parent_frame();
            match self.handle_call_result(Err(error)) {
                Ok(None) => return Ok(()),
                Ok(Some(_)) => unreachable!("an error cannot produce a call value"),
                Err(next) => error = next,
            }
        }
    }

    fn restore_parent_frame(&mut self) {
        let parent = self
            .frames
            .pop()
            .expect("restoring a parent requires a suspended frame");
        let mut child = std::mem::replace(&mut self.current, parent);
        let buffers = child.take_recyclable_buffers();
        drop(child);
        if self.recycled_frame_buffers.len() < MAX_RECYCLED_FRAME_BUFFERS {
            self.recycled_frame_buffers.push(buffers);
        }
    }
}
