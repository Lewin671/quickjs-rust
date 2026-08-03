use super::super::MatchState;
use super::super::repeat_visited::RepeatVisited;

#[derive(Clone, Copy)]
pub(super) enum CaptureRepeatWork {
    Expand { state: usize, count: usize },
    Accept { state: usize, count: usize },
}

impl CaptureRepeatWork {
    fn state(self) -> usize {
        match self {
            Self::Expand { state, .. } | Self::Accept { state, .. } => state,
        }
    }
}

#[derive(Default)]
pub(super) struct CaptureRepeatScratch {
    pub(super) work: Vec<CaptureRepeatWork>,
    pub(super) expanded: RepeatVisited,
}

impl CaptureRepeatScratch {
    pub(super) fn is_empty(&self) -> bool {
        self.work.is_empty() && self.expanded.is_empty()
    }
}

struct StateSlot {
    state: Option<MatchState>,
    active: bool,
}

/// Reusable capture-bearing branch storage for first-match repetition.
///
/// Active work items own slot ids, while free slots retain their capture
/// vector capacity across candidate starts. Taking a state temporarily leaves
/// its active slot empty so nested matching cannot reuse it.
#[derive(Default)]
pub(super) struct CaptureStatePool {
    slots: Vec<StateSlot>,
    free: Vec<usize>,
}

impl CaptureStatePool {
    pub(super) fn acquire_clone(&mut self, source: &MatchState) -> usize {
        if let Some(slot) = self.free.pop() {
            let entry = &mut self.slots[slot];
            debug_assert!(!entry.active);
            let state = entry.state.as_mut().expect("free capture state slot");
            state.clone_from(source);
            entry.active = true;
            slot
        } else {
            let slot = self.slots.len();
            self.slots.push(StateSlot {
                state: Some(source.clone()),
                active: true,
            });
            slot
        }
    }

    pub(super) fn take(&mut self, slot: usize) -> MatchState {
        let entry = &mut self.slots[slot];
        debug_assert!(entry.active);
        entry.state.take().expect("active capture state slot")
    }

    pub(super) fn put(&mut self, slot: usize, state: MatchState) {
        let entry = &mut self.slots[slot];
        debug_assert!(entry.active);
        debug_assert!(entry.state.is_none());
        entry.state = Some(state);
    }

    pub(super) fn index(&self, slot: usize) -> usize {
        let entry = &self.slots[slot];
        debug_assert!(entry.active);
        entry
            .state
            .as_ref()
            .expect("active capture state slot")
            .index
    }

    pub(super) fn release(&mut self, slot: usize) {
        let entry = &mut self.slots[slot];
        debug_assert!(entry.active);
        debug_assert!(entry.state.is_some());
        entry.active = false;
        self.free.push(slot);
    }

    pub(super) fn release_work(&mut self, work: &mut Vec<CaptureRepeatWork>) {
        for item in work.drain(..) {
            self.release(item.state());
        }
    }

    pub(super) fn is_clear(&self) -> bool {
        self.free.len() == self.slots.len()
            && self
                .slots
                .iter()
                .all(|slot| !slot.active && slot.state.is_some())
    }
}
