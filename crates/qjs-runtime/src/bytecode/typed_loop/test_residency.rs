//! A test-only residency override, isolated to one test thread and scoped by
//! a drop guard. This is test instrumentation, not runtime state: production
//! always uses MAX_NATIVE_ITERATIONS. A reduced crate-wide test constant would
//! change other tests' entry/decline counts and hide unintended tier changes.

use std::cell::Cell;

thread_local! {
    static LIMIT: Cell<Option<u64>> = const { Cell::new(None) };
}

pub(super) fn limit_or(default: u64) -> u64 {
    LIMIT.with(|limit| limit.get().unwrap_or(default))
}

pub(super) fn with_limit<T>(iterations: u64, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<u64>);
    impl Drop for Restore {
        fn drop(&mut self) {
            LIMIT.with(|limit| limit.set(self.0));
        }
    }
    let _restore = Restore(LIMIT.with(|limit| limit.replace(Some(iterations))));
    f()
}
