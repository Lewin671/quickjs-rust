# T066 follow-up: remote-free test allocates inside its observation window

- Status: closed-landed — repair verified on the owner branch; no integration performed.
- Implementation: present; only `allocator::tests::remote_free_is_drained_and_reused_by_the_owner`
  changes. Production allocator code and hot paths are unchanged.
- Verified at: base `87e27774d6284a5ec6f3942eb999a865bf639df6` plus allocator source blob
  `c14df33e4a7a160d97498a152d87b5c8e63d0801`, 2026-10-22, macOS arm64, Rust 1.95.0;
  before/after repetitions and all gates below passed on this source.
- Evidence: this record; transient process logs and allocation probe under `target/` are not retained.
- Unresolved: none for the reported failure.
- Next action: lead review and integration of the local `agent/t066-flaky-remote-free/codex` commit;
  this owner does not push or merge.

## Root cause

This is a test bug, not a demonstrated remote-free queue, epoch, or ownership bug.
The assertion at original `allocator.rs:936` samples the owner page's `live` before
two calls to `Barrier::wait()`, and expects it to stay unchanged afterwards.
A remote free does preserve `live`; the barrier can independently allocate on the owner.

In this toolchain, `std/src/sync/barrier.rs::Barrier::wait` calls its condition
variable's wait/notify operation. On macOS, `std/src/sys/sync/condvar/pthread.rs::Condvar::get`
initializes a `OnceBox` with `Box::pin(pal::Condvar::new())`. The allocation is
48 bytes, alignment 8, so the CLI allocator serves it from the same class as the
test's 48-byte block. The thread that first initializes the cold barrier owns
that allocation. If the owner wins, its page correctly gains one live block.

A temporary diagnostic wrapper around the **unmodified** allocator recorded the
layout and returned address without allocating (const TLS `Cell`s), enabling
tracing only between the snapshot and the end of the barrier waits. A 10 ms
delay outside that observation window forced each arrival order:

| First arrival | Before first wait | After first wait | After remote free / second wait | Allocation traced on owner |
| --- | ---: | ---: | ---: | --- |
| Owner | 1 | 2 | 2 | size 48, align 8; allocation and test block both mask to page `0x7cb450000` |
| Parent | 1 | 1 | 1 | none |

The count already increases **before** the remote free, directly separating the
barrier allocation from the allocator operation under suspicion. The parent-first
case explains frequent passes. Other test threads have separate TLS pools; their
allocations do not increment this page's owner-only `live`. Parallel execution
changes arrival order, and unrelated class epochs can request scans, but neither
is needed to explain the failure: the exact single test also reproduces it.

Review of `release`, `drain_remote`, `scan_remote`, and `allocate` found no allocator
change needed for this failure. A remote push leaves `live` counted, publishes the
stack entry before incrementing the class epoch, and the owner subtracts the
drained count. The traced values satisfy those invariants. This diagnosis does
not claim to prove the absence of every possible allocator race.

## Repair

Use scoped threads and stack-owned atomics to publish the address and acknowledge
completion of the remote free. The owner only loads an atomic and calls
`spin_loop` between its two `live` observations. Reserve the pointer collection
before that window, so later pushes cannot allocate either.

Keep the exact `live == saved_live` assertion. Additionally check that the remote
head is the published block, that ordinary allocation eventually returns that
same pointer with its patterned tail intact, that draining subtracts exactly one
from the page's count, and that the remote head is then null. Deallocate all
requests made while waiting for reuse; the old test leaked those live requests.
No serialization, relaxed assertion, production code change, dependency, or
performance measurement is involved.

## Reproduction and verification

Build with `cargo test -p qjs-cli --no-run`. The emitted default debug unit binary
for this checkout was `target/debug/deps/qjs-15bbaf0cd7660b53`. Both revisions used
the following exact-test stress method (8 concurrent processes, libtest parallel
mode enabled, fresh process and thread pools each iteration):

```python
import concurrent.futures, pathlib, subprocess
binary = str(pathlib.Path("target/debug/deps/qjs-15bbaf0cd7660b53").resolve())
def run(i):
    result = subprocess.run([
        binary, "allocator::tests::remote_free_is_drained_and_reused_by_the_owner",
        "--exact", "--test-threads=8",
    ], capture_output=True, text=True, timeout=60)
    return i, result.returncode
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
    results = list(pool.map(run, range(1, 2001)))
print([i for i, code in results if code])
```

| Check | Before (`87e27774`) | After (source blob above) |
| --- | --- | --- |
| Exact test, 2,000 iterations | 2 failures, iterations 723 and 1748 | 2,000 clean, zero failures |
| All 27 CLI unit tests, 200 iterations, 4 processes, `--test-threads=8` | 200 clean | 200 clean |

Both reproduced failures have the original `left: 2`, `right: 1` assertion at
line 936, followed by the `owner panicked` join failure at line 959. Neither
repetition run used diagnostic allocation tracing or forced-arrival delays. The
controlled arrival-order probe was a separate diagnosis, not the before/after
stress method.

Final gates (all passed):

- `cargo test -p qjs-cli --release --features agents`: 27 unit tests and 5 integration tests.
- `./scripts/check-touched.sh --staged --explain`: context, format, workspace Clippy,
  size guard, and CLI tests. No focused Test262 slice is selected for a CLI-only test change.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`:
  all stages, including default workspace tests, agents/perf-counters lint and tests,
  732 benchmark-tool tests, 35 script tests, hook isolation, and 5,170 Test262 subset cases.
- A byte-for-byte comparison against the base confirms everything before the
  `#[cfg(test)] mod tests` boundary in `allocator.rs` is unchanged.

Build logs and the temporary probe live under `target/`; handoff cleanup uses
`cargo clean` followed by `git checkout -- target` to restore the tracked benchmark record.
