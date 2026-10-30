# T077 S3a initial report (rejected by independent review)

Historical submission at `c8d15de4`, runtime `27d503a6`. The independent review
rejected the native guard: it rejected ordinary base programs and could not
prevent Rust stack overflow. Its limits, lock-in tests and realm state are removed.
Prior safety/local-complete claims below are the original submission's claims,
not current guarantees. Current evidence lives in [S3a results](../diagnostics/T077-s3a-results.md).


**Independent review at `c8d15de4` rejected the native guard and the prior local-
complete claim. Review repair is in progress.** The guard rejected ordinary base
programs and did not prove protection from stack overflow. The old depth/budget
claims below describe that rejected implementation, not current guarantees.
Current review results will supersede those claims; the base Rust-stack gap is S3d-owned.

Owner branch `agent/t077-s3a-native-entries/codex`, base `d19d3aa6`.
Implementation present at `27d503a6`; native/recycler tests and counters pass;
full check, 257 NG fixtures and complete configured exact coverage pass.
No push, merge, iMac timing or held-out input access. Formal acceptance remains
with the lead.

## Replacement and API

All 592 NativeFunction variants have exhaustive installation binding (584 without
agents), including internal reactions and a throwing TDZ-marker entry. Each entry
selects one operation directly; shared semantic helpers still live in their family.
NativeFunction identity remains for guards/semantics, not ordinary dispatch.

The source ABI is `fn(&mut CallEnv, CallInfo<'_>, &[Value]) -> Result<Value,
RuntimeError>`. CallInfo borrows callee, receiver, optional new.target and active
realm; CallEnv supplies the current caller services. Native VM call/construct
borrows from a detached owning operand vector, restored before catch/finally.
Host/callback and compact/wide tier entries borrow slices. Semantic retained
arguments, bound lists, Proxy trap arrays, apply/spread/rest lists still own values.

## Small corrections and measured costs

- The current interpreter has no explicit Context/activation directory. Use its
  existing OperandStack take/direct-restore owner; S3c/d can replace that owner without
  changing the native ABI. No new driver or backend is introduced.
- Checked native re-entry needs a realm-local depth and numeric stack-anchor cell
  in `function/env.rs`.
  It is shared by callback frames, releases on return/throw, and limits synchronous
  native depth to 256 and native-boundary stack growth to 1 MiB in both profiles. No global mutable state or RefCell borrow spans callbacks.
  This is the only service-state addition; S3b still replaces CallEnv directly.
- `FunctionData` grows from 296 to 304 bytes on aarch64: the direct entry pointer
  replaces a one-byte family cache. Its explicit size test now admits that word;
  the cold-header and lazy-map assertions remain. No new dependency/crate/unsafe.
- Constructor prototype lookup could previously exit before restoring new.target.
  The common slice constructor now restores it on that error as well as on callee
  return/throw. The focused test preserves exception identity and outer new.target.

## Initial verification

At the worktree based on `d19d3aa6`, nine `native::tests` pass: retained host and
bound values; nested native/JS callbacks; callback throw and caller operands;
async reaction retention; distinct-realm construction with explicit new.target;
prototype-getter throw; shared depth/error release; TDZ marker.
The first whole-runtime run had 2,381 passing tests and only the deliberate
header-size assertion failure (296 budget versus the new 304); that assertion
was updated with the direct-entry cost. Final full results follow below.

Before counters: the original retained instrumentation patch builds successfully
on `d19d3aa6`; all ten development checksums/stdout and two complete counter maps
per program reproduce [the inventory](../diagnostics/T077-s3-counts.md) exactly. In particular:
86,869,971 family probes, 78,693,537 declines, 10,986,795 family/core calls,
7,438,472 pop-argument vectors, 448,836 tier-fallback vectors and 5,259
slice-fallback vectors. These are construction events, not malloc or time.

On the implementation worktree, `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`
passes: default clippy, agents/perf-counters clippy and selected feature tests,
workspace tests (2,391 runtime tests), benchmark/script/context/size gates and
5,170 curated Test262 cases. `./scripts/compare-qjs.sh` passes all 257 fixtures.
The first full run preceded a mechanical constant-fold simplification in two
async-generator entry adapters; the pre-commit fast gate verifies the final source.

The subset honors TEST262_DIR. The exact-scan scripts hardcode their local
submodule path, so both pinned top-level submodules were initialized in this
worktree (no nested submodules, no main-checkout edits, no harness/config edits).

## Depth-limit correction after the first commit

`66301001` passed unit/NG/subset gates but failed the tenth development program:
`graphlib/paths` threw `RangeError: native call depth limit exceeded` at the
initial limit 32. A temporary non-timing, unbounded diagnostic (never production)
ran that same checksum successfully and measured maximum native nesting **158**.
The native budget is raised to 256; a general 48-level nested reduce-callback
regression test accompanies it. The boundary limit/release unit test now uses the
single declared budget. The old candidate is not claimed as a complete result.
The obsolete exact scan was stopped while still partial; no burndown was recorded
for it. The subsequent candidates and final verification below supersede that incomplete result.

## Corrected candidate counters

At `27d503a68e0e5179fd47e8cecb242becf6dca87c`, all ten development programs pass
with the original workload SHA-256 and exact stdout. Two fresh processes per
program produce identical complete counter maps. The raw [after counts](../diagnostics/T077-s3a-counts.json),
[temporary patch](../diagnostics/T077-s3a-instrumentation.patch) and [identities](../diagnostics/T077-s3a-metadata.json)
are retained. Counter instrumentation is absent from production source.
No timing/held-out run. The original full-map baseline was reproduced at `d19d3aa6`.

| Event | Before | After |
| --- | ---: | ---: |
| Family probes | 86,869,971 | 0 |
| Family declines | 78,693,537 | 0 |
| Family/core calls → installed direct entries | 10,986,795 | 10,986,795 |
| pop_arguments Vec constructions, all routes | 7,438,472 | 1,942,541 |
| Plain native reaching pop_arguments Vec | not separately counted | 0 |
| Tier fallback slice-to-Vec constructions | 448,836 | 0 |
| Host/callback slice fallback-to-Vec constructions | 5,259 | 0 |
| Native stack slice call entries | not separately counted | 5,481,535 |
| Native stack slice construct entries | not separately counted | 14,396 |

For **each** program, the pop-argument construction decrease equals native stack
slice call + construct entries; total **5,495,931**. Remaining pop constructions
are non-native routes, including general JS calls. The shared slice API also
removes former tier/host fallback copies for bytecode callees, without changing
JS activation ownership or implementing S3d. Semantic argument arrays and values
retained by native algorithms are outside this transport census.

Removed probe/fallback counter names are initialized to zero by the diagnostic
collector because their production sites no longer exist; source search verifies
that deletion. A separate live pop_arguments check counts any plain native that
still reaches the Vec site. Native direct entries exactly equal baseline native
calls for every input. Maximum synchronous native nesting is 158 (graphlib);
the other nine programs reach 2–6. Maximum observed native stack growth is
673,216 bytes (graphlib) under the diagnostic build. These counts establish mechanism removal,
not speed or process-wide allocation counts.

## Reproduce after counters

Run at the runtime revision above, with clean runtime source. The patch only adds
counts; it neither disables the depth bound nor changes semantic operations.
The earlier unlimited-depth diagnostic is not this patch and is not retained as
production evidence. For baseline reproduction use [the original recipe](../diagnostics/T077-s3-counts.md#exact-reproduction-commands)
at `d19d3aa6` (runtime/tools match the original inventory source).

```sh
git apply --check tasks/diagnostics/T077-s3a-instrumentation.patch
git apply tasks/diagnostics/T077-s3a-instrumentation.patch
cargo build --release -p qjs-cli --features perf-counters --target-dir target/t077-s3a/counter-build
PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
from pathlib import Path
import hashlib, json, os, subprocess
from tools.benchmark.screen import resolve_cases, DEFAULT_CACHE
work = Path('target/t077-s3a/programs')
work.mkdir(parents=True, exist_ok=True)
cases = resolve_cases(['development'], DEFAULT_CACHE, work)
records = json.loads(Path('tasks/diagnostics/T077-s3a-counts.json').read_text())
assert [c.id for c in cases] == [r['id'] for r in records]
env = {k:v for k,v in os.environ.items() if not k.startswith('QJS_')}
for case, record in zip(cases, records):
    assert hashlib.sha256(case.workload.read_bytes()).hexdigest() == record['sha256']
    for repeat in range(2):
        run = subprocess.run(['target/t077-s3a/counter-build/release/qjs', '--raw',
                              str(case.workload)], capture_output=True, text=True,
                             env=env, timeout=600)
        assert run.returncode == 0 and run.stdout == case.expected_stdout
        counters = {}
        for line in run.stderr.split('QJS_PERF_COUNTERS\n', 1)[1].splitlines():
            fields = line.split()
            if len(fields) == 2 and fields[1].isdigit():
                counters[fields[0]] = int(fields[1])
        assert counters == record['counters'], (case.id, repeat)
    print(case.id, 'checksum/stdout and both complete counter maps match')
PY
git apply -R tasks/diagnostics/T077-s3a-instrumentation.patch
```

## Deletion audit

`native.rs` removes **243** old source lines, including the entire **195-line**
family selector block and the cached probe flow; it adds 65 lines for the new ABI
and boundary in the first implementation commit (the depth repair adds one net
line). `function/value.rs` removes the cache field and all four initializers;
its direct-entry addition is eight added/nine removed lines at the first commit.
The 24 `native/*` family dispatchers plus iterator/generator/async reaction
selectors become operation-specific entries; no Option family fallback remains.
There are 2,476 gross runtime source deletions in the first commit, including
rewritten adapters. This is not a net source-size reduction: operation-specific
adapters and the exhaustive installation table add source. Primitive guarded
fast paths remain as planned; current executor retirement belongs to S3e.

## Debug boundary stress correction

A further 300-level reduce-callback stress at `b2ead44b` caught RangeError in
the release CLI, but overflowed Rust's larger debug stack frames before reaching
the 256-level guard in the debug CLI. The boundary now uses budgets **32 with
Rust debug assertions / 256 optimized**. This is a resource bound, not operation
or program recognition. The admission regression uses 32 debug / 48 optimized
levels, and a new deep-reentry unit requires catchable RangeError plus a subsequent
successful native call. All eleven focused debug tests pass. Optimized counters
remain the intended 256-budget measurement. Both obsolete exact scans were
stopped before completion; neither is a conformance record. That profile-budget revision was subsequently superseded by the stack-space
guard below.

## Stack-space guard correction

The optimized library's ordinary 2 MiB Rust test thread also overflowed before a
256-depth-only guard (`ae5e009e`), even though the optimized CLI's larger host
stack caught the same stress. A count alone cannot account for different Rust
frame sizes. The final boundary combines depth 256 with a **1 MiB stack-growth
budget from its outermost native entry**, for both profiles. It compares numeric
addresses of local markers using safe `ptr::from_ref(...).addr()` and absolute
distance, in either growth direction; it never retains or dereferences a pointer.
One realm-local numeric anchor is reset at outermost return/throw. CallInfo and
the S3b service migration ABI are unchanged; there is no FFI/unsafe/global state.
The shallow arithmetic regression uses 16 debug / 48 optimized levels; a separate
300-level reentry test requires catchable RangeError, successful subsequent native
work and boundary restoration. Twelve focused tests pass in both debug and optimized library builds;
the final full/NG/exact results are recorded below. The budget bounds native-to-native callback segments;
the existing pure-JS Rust recursion is still S3d's explicit-stack work.

## Final revision verification commands

Runtime revision `27d503a68e0e5179fd47e8cecb242becf6dca87c`:

- `cargo test -p qjs-runtime native::tests`: 12 passed.
- `cargo test --release -p qjs-runtime native`: 51 passed, including all 12 focused native tests, the operand-capacity regression, and the
  300-level callback rejection on the normal Rust test-thread stack. An existing
  release-only unused-import warning in `string/canonical/tests.rs` is outside
  this native-entry change and was not edited.
- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 TEST262_JOBS=4 ./scripts/check.sh`: passed all stages, including 2,395 runtime tests and 5,170 curated cases.
- `QJS_CLI_BIN="$PWD/target/release/qjs" COMPARE_QJS_JOBS=4 ./scripts/compare-qjs.sh`:
  all 257 fixtures pass, using the clean agents-enabled release binary.
- All ten development programs also pass once on that clean non-counter binary,
  with exact manifest stdout; the diagnostic two-run full-map checks pass as above.
- `QJS_AGENTS=1 QJS_CLI_PROFILE=release QJS_CLI_BIN="$PWD/target/release/qjs" TEST262_TIMEOUT_RETRIES=1 TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=4 ./scripts/find-qjsng-gaps.sh --exact --all --no-recommend --out-dir target/t077-s3a/exact-27d503a6`:
  all 42,672 configured Rust cases passed; see the complete aggregation/recovery record below.

Scope validation at this revision passes with the native/family/call-routing,
mechanical service-state and documentation paths declared; no S1 property/object/
layout/linked-code path changed. Runtime source is clean and contains no diagnostic
patch. Disk remains above 8 GiB. No timing job or held-out input was used.

## Operand recycler restoration correction

The new native detach sites must restore with direct Vec ownership assignment,
not `OperandStack::replace`: that API recycles the displaced vector, which here is
always the empty take() placeholder. Repeated native calls could fill the pool
with zero-capacity buffers and discard the useful caller buffer at frame teardown.
Both new sites now restore the actual owner without recycling that placeholder;
the pool/activation subsystem itself is unchanged. This preserves the existing
recycler invariant instead of introducing allocation work at native return.
The `2bd8d91a` exact scan was stopped while partial for this final routing change;
no burndown was recorded for it. The final source checks and counter rerun pass
as recorded above.

The operand-capacity regression evaluates forty native calls against reused
bytecode twice, then checks that its recycler still retains a usable allocation.
It passes with direct restoration; a temporary negative control using the old
recycle-empty restoration fails exactly with `native return discarded the caller's
reusable allocation`. The old restoration was restored only for that control and
is absent from the candidate. This is a construction/reuse test, not timing.

## Complete exact result and aggregation recovery

Runtime `27d503a68e0e5179fd47e8cecb242becf6dca87c`, 2026-10-30 UTC:
**53,572 corpus paths = 10,900 unchanged NG-config exclusions + 42,672 executed
configured cases. Rust: 42,672 pass, 0 fail, 0 timeout, 0 not-run. NG: 42,602
pass, 70 fail, 0 timeout. Gaps and NG-pass/Rust-not-run: zero.** Every configured
case passed Rust, so no prior configured pass is lost. The NG nonpasses are
reported as observed; no cause or new NG standing is inferred here.

The execution-session reset during continuation stopped the shell's serial fold
at 20,232 report rows **after every worker finished**. The original full queue,
42,672 canonical worker JSONL records and 42,672 status files for each engine
survived. Recovery did not re-run, reinterpret or replace any engine result:

- Queue indices are exactly 1–42,672, with no per-engine skip reasons.
- Every worker's path matches its queue row; both statuses match its canonical
  JSONL; all original partially folded rows match the recovered records.
- Those records plus all 10,900 original config-skip rows are a bijection with
  the pinned submodule's **entire tracked test corpus**: no missing/duplicate path.
- `scripts/test262-aggregate.py --require-complete-parity` over that complete
  recovered file passes. The local provenance label is explicitly used instead
  of its default CI label. This is a completed scan with recovered aggregation,
  not a partial-scan burndown or a claim that the interrupted shell exited 0.

The [recovery audit and hashes](../diagnostics/T077-s3a-exact-recovery.json) retain this conclusion.
The schema-1 result is appended to [the conformance ledger](../../docs/conformance/burndown.jsonl)
under `27d503a6`, source `local-exact-release-agents`, using the runbook's
`--entry` path. Earlier obsolete partial scans have no ledger entries.

```sh
python3 scripts/test262-aggregate.py \
  --ng-cases target/t077-s3a/exact-27d503a6/cases-complete.jsonl \
  --rust-cases target/t077-s3a/exact-27d503a6/cases-complete.jsonl \
  --commit 27d503a68e0e5179fd47e8cecb242becf6dca87c \
  --require-complete-parity \
  --summary-out target/t077-s3a/exact-27d503a6/coverage.md \
  --burndown-out target/t077-s3a/exact-27d503a6/burndown.jsonl \
  --comparison-cases-out target/t077-s3a/exact-27d503a6/comparison.jsonl
# Set the generated entry's source to local-exact-release-agents (a local run).
./scripts/test262-burndown.sh --entry target/t077-s3a/exact-27d503a6/burndown.jsonl
```

Additional deletion detail at `66301001`: 24 native family files replace 1,875
old lines with 4,915 lines of operation-specific adapters; the four iterator/
suspension owners replace 223 old lines with 620 entry/helper lines. The explicit
installation table and focused tests add source as well; no net shrink is claimed.

## Handoff and remaining decisions

S3a's implementation, counters and local correctness exit are complete on the
owner branch; no push, merge, iMac timing or held-out access. No new dependency,
crate, unsafe or FFI. The FunctionData header cost remains 296 → 304 bytes on
this host; stack-growth observation is a general synchronous boundary guard,
while pure-JS Rust recursion remains S3d's declared replacement.

The lead must run formal S3a comparisons against the accepted milestone/fixed
baseline and decide all four aggregate gates, then integration/rebase with S1.
No performance acceptance or production landing is claimed here. S3b still
requires accepted S2 and approved T074 contracts; preserve the native boundary's
shared depth/stack state while mechanically replacing its CallEnv service.
