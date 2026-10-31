# S2 follow-up: prototype memory and complexity

- Status: round-1 historical qualification; lead finds additional hot-path and owned-child memory costs; continue in [round 2](T077-s2fu-round2.md).
- Implementation: present at `590edc60`; all five requested mechanisms, direct guards, lazy prototype state and shared compact edge boxes.
- Verified at: `590edc60`, 2026-10-31: full check (5,170 curated), 268 NG fixtures cached/off/verify (verify at the default 10 seconds), eight x86_64 graph tests; fast gate runs 2,505 runtime tests.
- Evidence: this record, [iterations](../archive/T077-s2-followup-iterations.md), [review programs/collector](t077-s2-followup/measure.py).
- Unresolved: round-2 repairs and final acceptance; this record establishes round 1 only.
- Next action: follow the ordered lead feedback in the round-2 record; runner-ref transfers are now explicitly authorized.

## Scope and source

Integration base `1e6e2a2b`; review reference `d38f30a7`;
branch `agent/t077-s2-followup/codex`. Runtime edits stay in `value/layout/`,
`value/object.rs` cycle checking, `bytecode/named_property_cache/` and tests.
No function/env/call/context, bindings or `vm_bindings.rs` edits. No new
dependency, unsafe, push or merge. The [prototype contract](../../docs/design/core-rebuild-prototypes.md)
owns the final architecture; the [object contract](../../docs/design/core-rebuild-objects.md)
owns physical sizes and IC semantics.

| Runtime increment | Change |
| --- | --- |
| `f35bf831` | Iterative identity-set cycle walk; reject virtual constant admission before fetching; flatten constant load |
| `6a9c615d` | Lazy per-object validity state, shared holder shapes, inline ordinary edges, cold dictionary epochs; amortized reparent and armed-subtree invalidation |
| `9cd9c467` | Publish deferred roots once at IC admission |
| `3dc2a684` | Separate never-issued seeds from all semantic roots; direct layout hit guards |
| `590edc60` | Preserve weak edge-box sharing for compact array/function authorities |

Tests/collector increments: `635ca6f8`, `a24f95e3`, `d30838c6`.
The named iterations and deliberately interrupted scans are historical only;
no partial scan receives a burndown row.

## Final native correctness

At `590edc60`, full `check.sh` passes with default, agents and perf-counters
checks, tooling/context checks and all 5,170 curated cases. All 268 NG fixtures
pass cached/release, IC-off/release and IC-verify/debug. The final quiet-host debug run uses
two jobs and the default 10-second fixture timeout; all 268 pass. The earlier
loaded-host timeout and 60-second rerun are [recorded in history](../archive/T077-s2-followup-iterations.md#final-debug-verification-timeout).
Eight focused graph/ownership tests pass on each of arm64 and x86_64 (Rosetta).
This checks both representations, not iMac performance. Physical assertions
remain 64-byte NamedStore, 72-byte RefCell and 96-byte ordinary payload; compact
RefCell<NamedStorage> stays 32 bytes. Actual release `--agent` smoke passes.

Final release `agents` CLI SHA-256:
`c592a375cef71ace12fad265cfd45ca6c92c525323b2a1ab7bdd59f2ff32bf58`.

## Complete configured exact scans

Both complete modes pass all 42,672 configured cases with zero failures,
timeouts or not-run cases and zero actionable gaps. Each four-shard set covers
all 53,572 tracked JavaScript paths exactly once; the 10,900 NG configuration
skips are unchanged. NG passes 42,602 and fails 70; Rust passes all 70
reference-only failures. Cached and IC-off case outcomes are identical.

| Mode | Rust pass / fail / timeout / not-run | Burndown commit |
| --- | --- | --- |
| Cached | 42,672 / 0 / 0 / 0 | `590edc60` |
| IC-off | 42,672 / 0 / 0 / 0 | `bc2bc371` |

[Retained summaries and validation](t077-s2-followup/exact/validated.json).
Canonical sorted case-results SHA-256, identical for both modes:
`a633d502aaf8a01bf43641fb020f843eeeec372c604767ab1c6ebe3465c1b6aa`.
All eight commands exit 0. Both execute the pinned `590edc60` CLI, with four
partitions `--all --engine both --shard I/4`, one worker per shard,
QJS_AGENTS=1 and a 20-second timeout. No sampling or exclusion changes.
Scoped git diff over crates, scripts, Cargo files, .cargo and rust-toolchain.toml
is empty between the two recorded commits; only IC policy differs.
The complete-scan rows are appended in `docs/conformance/burndown.jsonl`.

## Final arm64 measurements

Three alternating pairs per script, frozen isolated release/agents builds:
pre-S2 `d38f30a7`, integration `1e6e2a2b`, candidate `590edc60`. Tables show
medians of whole-process wall time and per-process maximum RSS (decimal MB).
Both mandatory batches have zero observed competing compiler/qjs/test load.
All non-timer stdout values agree across builds and samples. Raw evidence:
[memory](t077-s2-followup/memory-arm64.json), [complexity](t077-s2-followup/complexity-arm64.json),
[recipe, compiler and collector hashes](t077-s2-followup/measurement-receipt.json).

| Program | RSS pre-S2 / integration / head MB | Wall pre-S2 / integration / head ms |
| --- | ---: | ---: |
| mem1 | 119.49 / 1324.38 / 118.96 | 334.9 / 621.1 / 312.3 |
| mem2 | 58.31 / 287.03 / 54.56 | 142.2 / 191.6 / 132.5 |
| mem3 | 6.16 / 5.98 / 6.05 | 1169.4 / 1432.0 / 1380.1 |
| mem4 | 162.32 / 394.22 / 158.65 | 254.4 / 332.4 / 246.9 |
| mem5 | 5.96 / 5.46 / 5.51 | 559.9 / 455.0 / 535.3 |
| mem6 | 46.91 / 201.67 / 46.48 | 84.4 / 108.3 / 78.1 |
| mem7 | 35.54 / 35.00 / 35.09 | 66.5 / 62.3 / 62.5 |
| rep | 23.99 / 93.91 / 22.46 | 81.0 / 2484.0 / 78.7 |
| casc1 | 33.08 / 145.97 / 30.47 | 75.0 / 676.0 / 69.3 |
| casc2 | 31.47 / 142.70 / 30.51 | 94.0 / 408.1 / 88.4 |
| ch | 8.54 / 13.84 / 6.50 | 11712.0 / 11719.5 / 1250.2 |
| deepcls | 8.93 / 13.24 / 12.93 | 579.0 / 514.6 / 499.1 |

Every mem1..mem7 RSS target passes: worst head/pre-S2 ratio **0.9956**,
below 1.10. The 6000-deep setPrototypeOf build is 11.719 → 1.250 s; the whole
reparent program is 2.484 → 0.079 s. Internal Date.now windows (integration →
head): reparent **2408 → 21 ms**, cascade 1 **581 → 0 ms**, cascade 2
**294 → 0 ms**; zero is at millisecond clock resolution.

**Remaining local regression:** mem5 is **1.1764× integration (+17.6%)** wall.
This diagnostic is not an aggregate-neutrality or promotion claim. Deep-class
RSS remains 12.93 MB versus pre-S2 8.93 MB (integration 13.24 MB); the stated
memory target covers the seven memory programs. The fixed-hardware iMac
RSS/public86 gates remain mandatory and unrun.

The first final-source memory batch was rejected in full after observing a
competing Cargo process (PID 32230); it is not pooled with these samples.
The collector enforces a five-second quiet preflight and 0.5-second load checks.
It excludes its own process tree and handles truncated non-UTF8 ps text with
replacement decoding; the validated collector hash is in the receipt.

## Round-1 iMac boundary (superseded by lead feedback)

This owner submitted no iMac job in round 1. The required wrapper invokes `git push` to transfer
runner refs, which conflicts with the explicit no-push instruction. Clarification
was requested during round 1; round-2 feedback now explicitly allows runner-ref
transfers. Lead measurements are recorded in the round-2 record. The queue was read successfully;
its observed 2026-10-31 13:58 CST clock was outside 02:45–05:00. Recheck time and
queue before a subsequently authorized `run --timing` submission.

The outstanding iMac gates are x86 RSS and public86 against integration base
`1e6e2a2b`. Public86 uses the existing 86 public selection IDs, never held-out.
No aggregate-neutrality claim is made without that run. Also audit/regenerate
`crates/qjs-cli/hot-functions-x86_64.order`: line 253 still names the now-inlined
ConstantValue::load (absent in the arm64 release symbol table). The x86 release
image is unverified; the order file is outside this owner's runtime boundary.
