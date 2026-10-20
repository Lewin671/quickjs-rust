# T056 design inputs and source checks

- Status: historical; the source checks and report limits read when the v2 design was written.
- Implementation: none; design input only.
- Verified at: design base `6af6230036f917f6b8df4dc51aa6402a7ef367ce` (paths as inspected there).
- Evidence: this record; workspace-local `tmp/perf-gap-analysis/REPORT*.md` are not retained.
- Unresolved: none here; [T056](../T056-interpreter-core-v2.md) owns current status.
- Next action: none.

Moved from the active T056 task file (text unchanged, relative links adjusted) to keep it within its budget.

## Source checks and input limits

Read the task entry, architecture, protected env/cell invariants, suspension
invariants, performance knowledge/workflow and T022 before designing.
Paths below are relative to `crates/qjs-runtime/src/`, inspected at the base.

| Finding | Source / implication |
| --- | --- |
| Frame calls still construct a nested Vm. | `bytecode/vm.rs` eval entry points and `function/call.rs::run_prepared_bytecode_call`; `bytecode/frame_stack.rs` is dormant. |
| Activating the driver alone repeats the moved-frame design. | `push_frame` uses `mem::replace(&mut self.current, frame)` and stores complete FrameStates; REPORT W2's claim that this path avoids movement is false. |
| Cold splitting already exists. | `bytecode/vm.rs::ColdFrame`; old 704-byte measurements are historical, not today's measured frame size. |
| Switching requires a view boundary. | `bytecode/frame_program.rs` owns code through a retained handle and forbids frame replacement under a live view. |
| All four property stores still compare string names. | `value/object.rs::PropertyStorage` and `ObjectLiteralShape`; `value/object/dynamic_storage.rs` maintains its own name-to-slot index. |
| Existing caches are not one uniform shape authority. | `bytecode/named_property_cache.rs` has Exact/LiteralShape/OwnSlot/SharedSlot/prototype entries, weak identities and revisions. |
| Value is a 16-byte enum of immediate values and Rc-like handles. | `value/mod.rs`, including its size test; clone/drop is ownership work, not necessarily removable work. |
| Scalar replacement already handles virtual reads directly. | `bytecode/vm_virtual_object.rs::LoadVirtualValue` reads the frame bank; it need not perform a named-property cache probe. |
| Accelerator decline restores ordinary execution. | `bytecode/vm_loop_dispatch.rs`; `virtual_object.rs` proves identity/escape and treats suspension as escape. Exact current workload reach still needs counters. |

Inputs were workspace-local `tmp/perf-gap-analysis/REPORT.md` and REPORT-2.md,
read for this design. They are not retained evidence; the durable facts needed
to motivate the design are recorded here, with their limitations:

- Lead-verified input: hash-map counted 7,020,006 OwnSlot hot rejects versus
  3,599,982 hits. Counts establish reach, not removable latency. The input
  identifies job `20261016T123302Z-general-fresh-route-counts` as diagnostic,
  never a timing binary.
- Lead-verified input: when typed-loop declines, ordinary named reads were
  estimated around 50 ns versus NG 11.6 ns. These are supplied path-cost
  estimates, not a new measurement here or proof that ICs explain the gap.
- The reports bind the 76-case baseline to binary
  `926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`,
  job `20261016T122024Z-general-current-baseline`; source main `6af62300`,
  executable source `9a0eeeca`. [T053](T053-ordinary-own-slot-hot-read-fat-lto.md)
  records that exact baseline and its byte-identical restoration.
- Lead-verified NG input: 8-byte JSValue and atom/shape lookup with no per-site
  IC at pinned `f7830186043e4488f2998759d60a514faf07cbc9`. This worktree's
  submodules are uninitialized; NG source/ABI was not independently inspected.
  No assertion about NG's call-stack implementation is needed for our design.
- REPORT's 845/866 inclusive AES call-chain samples include the work of all
  callees. They are not a removable-call-overhead fraction or a 10–30% forecast.
  Reports' expected percentage wins remain unverified hypotheses.
- REPORT-2's escape/warm/cache experiments suggest virtual receivers block
  tier seeding. Source supports the distinct virtual bank, not the claim that
  every fallback read executes GetPropNamed or that all excess cost is IC work.
  Unit 0 must count actual lowered opcodes and admission/exit reasons.
- Reusing compiled eval source needs fresh instantiation/identity and scope
  validation; no general eval cache is smuggled into frame or atom migration.
  Held-out npm versions, licenses and plain-shell compatibility in REPORT-2
  were not verified here. Pin runnable inputs and validate outputs before use.

