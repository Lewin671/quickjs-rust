# T077 S3a implementation and review repair

- Status: independent-review repairs locally complete; depth parity, counters, full gate, NG fixtures and fresh complete exact scan all pass. Lead formal acceptance remains pending.
- Implementation: direct operation entries and borrowed arguments retained; no native/realm depth or stack limit. The rejected original report is [historical](../archive/T077-s3a-initial-results.md).
- Verified at: runtime `03c8680c`: full check (2,391 runtime tests; 5,170 curated cases), 257 NG fixtures, debug/release depth parity, twenty development counter runs and all 42,672 configured exact Rust cases pass; zero gaps/not-run.
- Evidence: [depth records](T077-s3a-depth-parity.json), [depth reproducer](T077-s3a-depth-probe.py); fresh [counter rows](T077-s3a-review-counts.json), [temporary patch](T077-s3a-review-instrumentation.patch), [build identities](T077-s3a-review-metadata.json). Older counters remain bound to `27d503a6`.
- Unresolved: lead formal performance acceptance/integration only. The known base deep Rust-stack gap remains S3d-owned.
- Next action: lead reviews the repaired candidate and formal acceptance; S3b still needs accepted S2/T074 contracts. No push/merge by this owner.

## What the review corrected

The earlier native limit was an unjustified import from the broader S3 inventory,
not a response to a demonstrated S3a-only crash. The code added a limit before
comparing against base. The later 300-level reduce stress **already aborts in base
in debug**, while both base and repaired release complete it with result 0.
The depth-only guard, debug-specific variant, 1 MiB realm anchor and tests that
required those errors are deleted. No substitute limit is introduced.

Deep Rust recursion remains a known base gap. Base and repaired CLI both abort
for a forEach walk at 1,000 levels. S3d owns a runtime-wide anchored stack-pointer
guard with host-configured budget, a CLI engine thread with explicit stack size,
and frame-counted JS depth after pure-JS calls no longer recurse in Rust. This
S3a change does not claim protection from stack overflow.

## Hot-path repair and scope

- CallInfo borrows callee/receiver and records `is_construct`. CallEnv is the
  single service/context authority. There is no per-call realm Rc clone/drop.
- No entry used the old new_target value, only its presence. The construct flag
  replaces that eager lookup/clone; semantic constructors still fetch actual
  new.target from CallEnv at the point their existing algorithms need it.
- The proven NativeEntry pointer is passed to the thin inline call adapter;
  it never reloads/unwraps native_entry with an expect.
- Common dispatch borrows callee; detached VM operand slots transfer callee and
  receiver by ownership. call_function_slice no longer clones a native callee
  again to enter the dispatcher.
- The base callback direct-leaf shortcut is restored before general dispatch.
  In debug, the rejected slice wrapper entered the 3,936-byte generic dispatch
  frame even for admitted JS callbacks. Base's shortcut used a 224-byte slice
  frame; the repaired shortcut uses 192 bytes and bypasses that generic frame.
  These are disassembly stack reservations, not timing measurements.
- No stack_native_callee probe remains. Existing fixed-arity JS fast paths keep
  their original order; generic call/direct-eval/resolved-call fallthroughs and
  constructors borrow their owning operand block independent of callee kind.
  Therefore no new native-entry header load is inserted into JS->JS admission.
  This also removes some ordinary JS Vec transport earlier than S3d, without an
  activation, binding or opcode rewrite. Remaining semantic/virtual/super lists
  will be counted separately; no borrowed list escapes its caller owner.
- The recycler repair is retained: restore the actual owner directly, without
  recycling the empty detachment placeholder. Constructor new.target restoration
  on prototype-getter throw is retained. No S1 property/layout path changed.

CallInfo's construct-mode shape is the lead-approved correction to the earlier
borrowed-new.target/realm proposal. S3b changes the CallEnv service to Context;
actual new.target/realm remain lazy service capabilities, not an eager adapter.

## Base/head depth parity

Default-feature CLI, aarch64 macOS, same toolchain/recipes, ordinary main-thread
stack (no stack override). Base Rust sources were temporarily restored from
`d19d3aa6` **only in this worktree**, verified byte-for-byte by git diff, built,
then restored before edits. A second pair of binaries removes only the guard and
unused realm field; a third pair includes all hot-path fixes. Core dumps were
disabled in the external probe runner. No elapsed-time/cycle measurement.

Values are largest successful depths found by exponential/binary probing on this
host; they are not portable limits or promises. All base-success maxima and
explicit feedback landmarks also succeed on repaired head. `>=4096` is the
probe cap, not an engine-added limit.

| Case | Base debug | Repaired debug | Base release | Repaired release |
| --- | ---: | ---: | ---: | ---: |
| forEach tree walk | 143 | 173 | 712 | 802 |
| map tree walk | 142 | 172 | 706 | 796 |
| reduce tree walk | 176 | 225 | 744 | 828 |
| f.call recursion | 341 | 433 | 2,034 | 3,337 |
| f.apply recursion | 338 | 353 | 1,879 | 2,108 |
| Reflect.apply recursion | 309 | 354 | 1,674 | 2,234 |
| generator yield* chain | 142 | 146 | 665 | 712 |
| plain recursion inside callback | >=4096 | >=4096 | >=4096 | >=4096 |

The guard-removed intermediate also improves seven routes, but release reduce
was 733 versus base 744. Restoring the callback shortcut and reducing metadata/
callee ownership fixes that residual frame-growth regression (final 828).
The original 300-level reduce stress aborts in **both** debug binaries and passes
in **both** release binaries: it was not evidence for a new S3a limit.

```sh
python3 tasks/diagnostics/T077-s3a-depth-probe.py \
  --root target/t077-s3a-review/base-build --label base-d19d3aa6 \
  --output target/t077-s3a-review/base-depth.json
python3 tasks/diagnostics/T077-s3a-depth-probe.py \
  --root target/t077-s3a-review/fixed-build --label repaired \
  --output target/t077-s3a-review/repaired-depth.json
```

## Fresh deterministic counters

At `03c8680c`, the ten original development inputs have matching SHA-256 and
stdout, with two identical complete counter maps each. Native direct entries
still exactly equal baseline native_calls for every program. No timing or held-out
input was used. Removed probe/copy sites have explicit zero counters; a live
pop-argument check also confirms no plain native reaches the old Vec site.

| Event | Base d19d3aa6 | Prior S3a 27d503a6 | Reviewed 03c8680c |
| --- | ---: | ---: | ---: |
| family probes | 86,869,971 | 0 | 0 |
| family declines | 78,693,537 | 0 | 0 |
| family/core calls → direct entries | 10,986,795 | 10,986,795 | 10,986,795 |
| pop-argument Vec constructions | 7,438,472 | 1,942,541 | 0 |
| tier fallback Vec copies | 448,836 | 0 | 0 |
| host/callback slice fallback Vec copies | 5,259 | 0 | 0 |

The removed pop constructions are partitioned exactly, **for each input**, into
5,481,535 native call borrows + 14,396 native construct borrows + 1,941,761 other
call borrows + 780 other construct borrows. These sum to 7,438,472. There is no
native-kind admission probe in production: the diagnostic patch alone classifies
these counters. Retained values, spread/apply/rest/bound lists and native-created
JS arrays are not transport copies in this census. No process-wide allocation or
wall-time reduction is claimed. Runtime source contains no counter patch.

```sh
git apply --check tasks/diagnostics/T077-s3a-review-instrumentation.patch
git apply tasks/diagnostics/T077-s3a-review-instrumentation.patch
cargo build --release -p qjs-cli --features perf-counters --target-dir target/t077-s3a/counter-build
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=. python3 tasks/diagnostics/T077-s3a-counter-probe.py
# Writes target/t077-s3a-review/reproduced-counts.json for comparison with the
# committed T077-s3a-review-counts.json (all full maps must match).
git apply -R tasks/diagnostics/T077-s3a-review-instrumentation.patch
```

## Final gates

At frozen runtime `03c8680c`:

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 TEST262_JOBS=4 ./scripts/check.sh`: all stages pass, including 2,391 runtime tests and 5,170 curated Test262 cases.
- `QJS_CLI_BIN="$PWD/target/release/qjs" COMPARE_QJS_JOBS=4 ./scripts/compare-qjs.sh`: all 257 fixtures pass (clean agents-enabled release CLI).
- Full development counter reproduction passes as above; the cleaned reproducer independently re-ran all twenty maps and matched the saved data. The counter patch is absent from production source.
- Complete configured exact scan passed on the same frozen binary with
  `QJS_AGENTS=1 QJS_CLI_PROFILE=release QJS_CLI_BIN="$PWD/target/release/qjs" TEST262_TIMEOUT_RETRIES=1 TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=6 ./scripts/find-qjsng-gaps.sh --exact --all --no-recommend --out-dir target/t077-s3a-review/exact`.

The detached runner completed normally with exit 0, including its serial
aggregation and cleanup; no recovery was needed. The original corpus is 53,572
paths, with the unchanged 10,900 NG-config exclusions and 42,672 configured cases.
**Rust: 42,672 pass; 0 fail, timeout or not-run. NG: 42,602 pass; 70 fail; 0 timeout.
Actionable gaps and NG-pass/Rust-not-run are both zero.** All configured Rust
cases pass, so no configured pass is lost. NG nonpasses are recorded as observed;
no cause is inferred. The new ledger line is for `03c8680c`; no older line is edited.

```sh
./scripts/test262-burndown.sh --report target/t077-s3a-review/exact \
  --commit 03c8680c --source local-exact-release-agents
```

The full command/environment, binary/source pins and result hashes are in
[T077-s3a-review-metadata.json](T077-s3a-review-metadata.json). The native API still
uses CallEnv as its only service authority, with borrowed callee/receiver and a
construct flag; no Context adapter, dependency, crate, unsafe or FFI was added.
The FunctionData header remains the earlier measured 304 bytes (base 296).
S1 property/layout/linked-code paths are untouched. Debug-only obsolete build
cache was cleaned after all gates; the frozen release scan binary was retained
and hash-verified. Free disk stayed above 8 GiB.

## Handoff

This applies the independent review completely: no S3a recursion limit or realm
state; all requested metadata/clone/probe/expect fixes; both-profile depth parity;
fresh full gate, comparison, counters and complete exact coverage/burndown.
The original safety claims are explicitly rejected in the historical report.
The actual 300-level debug crash was pre-existing at base; the avoidable callback
generic frame was removed, and every measured base-success ceiling and feedback
landmark works on head.

No push, merge, timing job or held-out access. The lead owns formal aggregate
acceptance and integration/rebase with S1. S3b still requires accepted S2 and
approved T074 contracts; S3d owns the pre-existing runtime-wide stack-policy gap.
