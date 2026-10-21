# T056 native allocation diagnosis (historical producers; never timing)

[Inventory](../../archive/T056-m1-stageA-allocation-inventory.md) owns sizes,
classes and request counts; [diagnosis](../../archive/T056-m1-stageA-allocation-diagnosis.md)
owns the fixes, instruction checks and handoff. These commands do not change
services, allocator defaults, frozen plans or attempt budgets. They were run
through the serialized iMac queue **without --timing**. Do not use their
instrumented process wall times as performance evidence.

- `type-sizes.sh`: archives frozen f58c696e/base and8f7d97c7/candidate, builds
  x86_64 agents with the release recipe plus diagnostic `-Zprint-type-sizes`
  (`RUSTC_BOOTSTRAP=1` only in each compiler child). Retains both complete type
  inventories and field layouts. These binaries are not canonical receipts.
- `classes.sh`: builds a standalone C probe against macOS libSystem; for
  requests1..4096 records `malloc_good_size` and actual `malloc_size`, then
  frees each block. It is not linked to the engine and adds no engine FFI.
- `native-profile.sh`: uses the **original canonical** base and8f binaries,
  `MallocStackLoggingNoCompact=1` only in the child, and `/usr/bin/malloc_history
  PID -allEvents -noContent`. The CLI remains alive waiting for REPL input;
  no source or allocator wrapper is added to Rust. It retains gzipped events,
  every grouped qjs stack, request histograms, image/source/raw hashes, and
  matching empty wrappers. Whole unpack is unchanged inside a one-line
  `eval(JSON source)` REPL wrapper; string_slice uses100000 iterations with
  the same wrapper. Startup, wrapper parsing and profiler self-allocation are
  included, so compare the empty control too. The wrapper is diagnostic,
  not the benchmark harness's timing invocation.

The native event `size` is the requested size, not usable or live bytes.
`rounded16_counts` is a nominal request model: only1..256 is treated as the
nano-class table, verified by `classes.sh`; the >256 tail is an event count,
not a claim that all large allocations round to16. VM_ALLOC and FREE are
separate; realloc allocation sides are ALLOC events. Runtime symbols may be
merged by LTO, so attribute through semantic callers, not a generic allocator's
last symbol alone. Stack logging perturbs heap placement; no latency claim.

Run only in a fresh runner output directory (the queue provides BENCH_OUT):

```sh
bash tasks/diagnostics/t056-allocation-probe/type-sizes.sh
bash tasks/diagnostics/t056-allocation-probe/classes.sh
bash tasks/diagnostics/t056-allocation-probe/native-profile.sh
```

The native profiler requires the two frozen canonical cache paths already
built; verify their image hashes against the inventory before running. These
historical scripts are not the new lead correctness/pre-gate/triage chain.
