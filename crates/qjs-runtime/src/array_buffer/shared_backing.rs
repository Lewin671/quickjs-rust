//! Cross-thread backing store for `SharedArrayBuffer` under the Test262
//! `$262.agent` harness. The whole module is gated behind the `agents` cargo
//! feature; the default single-threaded build keeps `SharedArrayBuffer` bytes
//! in the per-object `internal_bytes` slot.
//!
//! A `SharedBacking` is shared by `Arc` across the agent cluster: when the main
//! agent broadcasts a buffer, each worker (a separate engine on its own OS
//! thread) wraps the same `Arc` in a fresh local `SharedArrayBuffer` object, so
//! every agent reads and writes one memory region.
//!
//! The store is a coarse `Mutex<Vec<u8>>`. The spec does not require lock-free
//! access; every byte read, every TypedArray/Atomics read-modify-write, and
//! every `grow` takes the lock for its (leaf, non-reentrant) duration. The
//! paired `Condvar` parks `Atomics.wait` callers (Phase 4) — `wait_timeout`
//! releases the mutex while a waiter is parked so other agents can store and
//! notify.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// The shared bytes plus the wait/notify parking primitive for one
/// `SharedArrayBuffer`.
pub(crate) struct SharedBacking {
    data: Mutex<Vec<u8>>,
    waiters: Condvar,
}

/// A cheaply cloned, `Send + Sync` handle to a [`SharedBacking`].
pub(crate) type SharedBackingRef = Arc<SharedBacking>;

impl SharedBacking {
    /// Builds a backing pre-filled with `bytes`.
    pub(crate) fn new(bytes: Vec<u8>) -> SharedBackingRef {
        Arc::new(Self {
            data: Mutex::new(bytes),
            waiters: Condvar::new(),
        })
    }

    /// Locks the bytes, recovering from a poisoned lock (a worker agent may
    /// panic while holding it; the bytes themselves stay valid).
    fn lock(&self) -> MutexGuard<'_, Vec<u8>> {
        self.data
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Runs `f` over an immutable view of the bytes while holding the lock.
    pub(crate) fn with_bytes<T>(&self, f: impl FnOnce(&[u8]) -> T) -> T {
        f(&self.lock())
    }

    /// Runs `f` over the byte vector while holding the lock. Used for in-place
    /// element writes and `grow`.
    pub(crate) fn with_bytes_mut<T>(&self, f: impl FnOnce(&mut Vec<u8>) -> T) -> T {
        f(&mut self.lock())
    }

    /// A copy of the current bytes.
    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.lock().clone()
    }

    /// Replaces the bytes wholesale (TypedArray bulk writes, `grow`, `slice`).
    pub(crate) fn set(&self, bytes: Vec<u8>) {
        *self.lock() = bytes;
    }

    /// The parking primitive backing `Atomics.wait`/`Atomics.notify`. Wired in
    /// Phase 4; exposed here so the registry can park and wake on it.
    #[allow(dead_code)]
    pub(crate) fn waiters(&self) -> &Condvar {
        &self.waiters
    }
}
