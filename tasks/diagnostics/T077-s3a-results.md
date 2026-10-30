# T077 S3a implementation and verification

Owner branch `agent/t077-s3a-native-entries/codex`, base `d19d3aa6`.
Implementation present; full check and NG fixtures pass; exact/counter gates pending. No push, merge,
iMac timing or held-out input access. Lead owns formal aggregate acceptance.

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
  existing OperandStack take/replace owner; S3c/d can replace that owner without
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

## Verification in progress

At the worktree based on `d19d3aa6`, nine `native::tests` pass: retained host and
bound values; nested native/JS callbacks; callback throw and caller operands;
async reaction retention; distinct-realm construction with explicit new.target;
prototype-getter throw; shared depth/error release; TDZ marker.
The first whole-runtime run had 2,381 passing tests and only the deliberate
header-size assertion failure (296 budget versus the new 304); that assertion
was updated with the direct-entry cost. Final full results follow below.

Before counters: the original retained instrumentation patch builds successfully
on `d19d3aa6`; all ten development checksums/stdout and two complete counter maps
per program reproduce [the inventory](T077-s3-counts.md) exactly. In particular:
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
for it. The final candidate must run the complete scan and all ten counters again.

## Corrected candidate counters

At `b2ead44bfbd310f064002bda4b93b927be3bd9e3`, all ten development programs pass
with the original workload SHA-256 and exact stdout. Two fresh processes per
program produce identical complete counter maps. The raw [after counts](T077-s3a-counts.json),
[temporary patch](T077-s3a-instrumentation.patch) and [identities](T077-s3a-metadata.json)
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
the other nine programs reach 2–6. These counts establish mechanism removal,
not speed or process-wide allocation counts.

## Reproduce after counters

Run at the runtime revision above, with clean runtime source. The patch only adds
counts; it neither disables the depth bound nor changes semantic operations.
The earlier unlimited-depth diagnostic is not this patch and is not retained as
production evidence. For baseline reproduction use [the original recipe](T077-s3-counts.md#exact-reproduction-commands)
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
stopped before completion; neither is a conformance record. A fresh final scan
is required for the profile-budget revision.

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
work and boundary restoration. Twelve focused debug tests pass; optimized tests
and final checks follow. The budget bounds native-to-native callback segments;
the existing pure-JS Rust recursion is still S3d's explicit-stack work.
