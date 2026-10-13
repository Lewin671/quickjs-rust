//! Scoped physical-site handback injection; absent from production builds.
use crate::Value;
use std::cell::{Cell, RefCell};

thread_local! {
    static TARGET: Cell<Option<usize>> = const {Cell::new(None)};
    static SNAPSHOT: RefCell<Option<(usize,Vec<Value>)>> = const {RefCell::new(None)};
    static UNFORWARDED: Cell<bool> = const {Cell::new(false)};
}

pub(super) fn unforwarded() -> bool {
    UNFORWARDED.with(Cell::get)
}

/// Total Move operations cannot exit in production. Inspect their physical
/// input mapping before copy forwarding removes operands only they discard.
pub(super) fn with_unforwarded<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            UNFORWARDED.with(|value| value.set(self.0));
        }
    }
    let _restore = Restore(UNFORWARDED.with(|value| value.replace(true)));
    f()
}

pub(super) fn take_target(ip: usize) -> bool {
    TARGET.with(|target| {
        if target.get() != Some(ip) {
            return false;
        }
        target.set(None);
        true
    })
}

pub(super) fn record(ip: usize, stack: Vec<Value>) {
    SNAPSHOT.with(|snapshot| *snapshot.borrow_mut() = Some((ip, stack)));
}

pub(super) fn at<T>(ip: usize, f: impl FnOnce() -> T) -> (T, Option<(usize, Vec<Value>)>) {
    struct Restore(Option<usize>, Option<(usize, Vec<Value>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TARGET.with(|target| target.set(self.0));
            SNAPSHOT.with(|snapshot| *snapshot.borrow_mut() = self.1.take());
        }
    }
    let _restore = Restore(
        TARGET.with(|target| target.replace(Some(ip))),
        SNAPSHOT.with(|snapshot| snapshot.borrow_mut().take()),
    );
    let result = f();
    (
        result,
        SNAPSHOT.with(|snapshot| snapshot.borrow_mut().take()),
    )
}
