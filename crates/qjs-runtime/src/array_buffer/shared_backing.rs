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
//! The store is a coarse lock over the bytes plus the `Atomics.wait` waiter
//! list. The spec does not require lock-free access; every byte read, every
//! TypedArray/Atomics read-modify-write, and every `grow` takes the lock for its
//! (leaf, non-reentrant) duration.
//!
//! The lock is `parking_lot` with **fair unlocking** (`unlock_fair`), not
//! `std::sync::Mutex`. A JavaScript `Atomics` busy-loop
//! (`while (Atomics.load(...) === 0) {}` / `$262.agent.waitUntil`) is pervasive
//! in the agent tests; under the platform's non-fair lock (macOS
//! `os_unfair_lock`) a spinning agent can re-acquire the lock immediately and
//! starve a writer on another agent thread indefinitely, hanging the test. Fair
//! unlocking hands the lock to the next queued waiter, bounding the wait. The
//! paired `Condvar` parks `Atomics.wait` callers; `wait_for` releases the lock
//! while a waiter is parked so other agents can store bytes and notify.

use std::sync::Arc;

use parking_lot::{Condvar, Mutex, MutexGuard};

/// A parked `Atomics.wait` caller, keyed by the byte offset it waits on. The
/// `notified` flag is set by `Atomics.notify` so the woken thread distinguishes
/// a real wake from a spurious one.
struct Waiter {
    id: u64,
    byte_offset: usize,
    notified: bool,
}

/// Bytes plus the live waiter list, guarded by one mutex so a wait can compare
/// the value and park atomically and a notify can flip waiter flags atomically.
struct Inner {
    bytes: Vec<u8>,
    waiters: Vec<Waiter>,
    next_id: u64,
}

/// The outcome of an `Atomics.wait` on shared memory.
pub(crate) enum WaitOutcome {
    /// The value did not match the comparand; the agent did not block.
    NotEqual,
    /// The agent was woken by `Atomics.notify`.
    Ok,
    /// The timeout elapsed before any notify.
    TimedOut,
}

/// The shared bytes plus the wait/notify parking primitive for one
/// `SharedArrayBuffer`.
pub(crate) struct SharedBacking {
    inner: Mutex<Inner>,
    condvar: Condvar,
}

/// A cheaply cloned, `Send + Sync` handle to a [`SharedBacking`].
pub(crate) type SharedBackingRef = Arc<SharedBacking>;

impl SharedBacking {
    /// Builds a backing pre-filled with `bytes`.
    pub(crate) fn new(bytes: Vec<u8>) -> SharedBackingRef {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                bytes,
                waiters: Vec::new(),
                next_id: 0,
            }),
            condvar: Condvar::new(),
        })
    }

    /// Runs `f` over an immutable view of the bytes, then hands the lock to the
    /// next queued waiter (fair unlock) so a busy-looping agent cannot starve a
    /// writer on another thread.
    ///
    /// Reads are overwhelmingly the hot path of a JS `Atomics` spin
    /// (`while (Atomics.load(...) === 0) {}` / `$262.agent.waitUntil`). Yielding
    /// the timeslice after each read keeps such a spin cooperative: it stops the
    /// agent pegging a core, which on an oversubscribed CI runner would starve
    /// unrelated tests sharing the machine of CPU.
    pub(crate) fn with_bytes<T>(&self, f: impl FnOnce(&[u8]) -> T) -> T {
        let guard = self.inner.lock();
        let result = f(&guard.bytes);
        MutexGuard::unlock_fair(guard);
        std::thread::yield_now();
        result
    }

    /// Runs `f` over the byte vector, then fair-unlocks. Used for in-place
    /// element writes and `grow`.
    pub(crate) fn with_bytes_mut<T>(&self, f: impl FnOnce(&mut Vec<u8>) -> T) -> T {
        let mut guard = self.inner.lock();
        let result = f(&mut guard.bytes);
        MutexGuard::unlock_fair(guard);
        result
    }

    /// A copy of the current bytes.
    pub(crate) fn snapshot(&self) -> Vec<u8> {
        let guard = self.inner.lock();
        let bytes = guard.bytes.clone();
        MutexGuard::unlock_fair(guard);
        bytes
    }

    /// Replaces the bytes wholesale (TypedArray bulk writes, `grow`, `slice`).
    /// The waiter list is preserved (`grow` never invalidates a waited offset).
    pub(crate) fn set(&self, bytes: Vec<u8>) {
        let mut guard = self.inner.lock();
        guard.bytes = bytes;
        MutexGuard::unlock_fair(guard);
    }

    /// `Atomics.wait`: if `matches` (evaluated under the lock against the current
    /// bytes) is false, returns `NotEqual` without blocking. Otherwise parks
    /// until `Atomics.notify` wakes this waiter (`Ok`) or `timeout` elapses
    /// (`TimedOut`). `None` timeout waits indefinitely.
    pub(crate) fn wait(
        &self,
        byte_offset: usize,
        timeout: Option<std::time::Duration>,
        matches: impl FnOnce(&[u8]) -> bool,
    ) -> WaitOutcome {
        let mut guard = self.inner.lock();
        if !matches(&guard.bytes) {
            MutexGuard::unlock_fair(guard);
            return WaitOutcome::NotEqual;
        }
        let id = guard.next_id;
        guard.next_id += 1;
        guard.waiters.push(Waiter {
            id,
            byte_offset,
            notified: false,
        });
        // An absolute deadline keeps the timeout correct across spurious wakeups
        // (a relative `wait_for` would restart the clock each time).
        let deadline = timeout.map(|duration| std::time::Instant::now() + duration);
        let outcome = loop {
            if waiter_notified(&guard, id) {
                break WaitOutcome::Ok;
            }
            match deadline {
                None => self.condvar.wait(&mut guard),
                Some(deadline) => {
                    let result = self.condvar.wait_until(&mut guard, deadline);
                    if result.timed_out() {
                        break if waiter_notified(&guard, id) {
                            WaitOutcome::Ok
                        } else {
                            WaitOutcome::TimedOut
                        };
                    }
                }
            }
        };
        guard.waiters.retain(|waiter| waiter.id != id);
        MutexGuard::unlock_fair(guard);
        outcome
    }

    /// `Atomics.notify`: wakes up to `count` agents waiting on `byte_offset`
    /// (all when `count` is `None`), in FIFO order. Returns the number woken.
    pub(crate) fn notify(&self, byte_offset: usize, count: Option<usize>) -> usize {
        let mut guard = self.inner.lock();
        let mut woken = 0;
        for waiter in guard.waiters.iter_mut() {
            if count.is_some_and(|count| woken >= count) {
                break;
            }
            if waiter.byte_offset == byte_offset && !waiter.notified {
                waiter.notified = true;
                woken += 1;
            }
        }
        if woken > 0 {
            self.condvar.notify_all();
        }
        MutexGuard::unlock_fair(guard);
        woken
    }
}

/// Whether the waiter with `id` is still parked and has been notified. A missing
/// waiter (already removed) counts as notified so the loop exits cleanly.
fn waiter_notified(inner: &Inner, id: u64) -> bool {
    inner
        .waiters
        .iter()
        .find(|waiter| waiter.id == id)
        .is_none_or(|waiter| waiter.notified)
}
