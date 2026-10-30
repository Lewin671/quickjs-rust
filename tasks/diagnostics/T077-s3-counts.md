# T077 S3 deterministic development path counts

Source **`dd0905b2cb0236c38619089a27e8342b575dda61`**, 2026-10-29. Instrumentation only;
no runtime changes retained in the source tree. Ten development programs, two
fresh processes each; every repeat has identical full counter maps, exit 0 and
the exact manifest checksum/stdout. No held-out input resolved, read or run.
No wall/cycle/instruction timing collected; release build duration is not evidence.

Machine: macOS-26.6.2-arm64-arm-64bit-Mach-O; rustc 1.95.0 / LLVM 22.1.2, aarch64-apple-darwin.
Build: default release profile plus `perf-counters`; no agents, custom RUSTFLAGS,
A/B binary or performance standing. Counts include startup/library setup.

- [Raw per-program counters, input hashes and stdout](T077-s3-counts.json).
- [Source/build/binary/patch identity](T077-s3-metadata.json).
- [Complete throwaway source patch](T077-s3-instrumentation.patch).
- [Plan and inventory entry point](../proposals/T077-s3-plan.md).

Binary SHA-256: `7da4553ce43032d0611cb7b64be740c9d06fb8483766bc28a1098147be9e2765`.
Patch SHA-256: `9a2cff8d6bdea973e51f00a661ce4f44a5c708fa4bccae721072deaa4ba4cfe9`.
Pinned development manifest SHA-256:
`2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.

## Counters, units and denominators

`ordinary_call_attempts` is **not** a unique JS-call denominator: call_function
increments it before forwarding direct leaves, and call_direct_leaf_function
increments again. The retained patch counts exactly 306,548 such forwards.
Corrected attempted ordinary entries = **27,396,841**. Numeric-plan successes
from wide inline entry add **1,901,920** calls omitted by that legacy counter;
the measured plan-internal call count is zero. Combined routed JS entries here
= **29,298,761**; primitive operations flattened by compile-time optimization
are not runtime call entries. Bound/Proxy dispatches are separate forwarding
events, both zero on this corpus; async/generator callable entries are also zero.

The next table partitions those measured routed entries using the existing
call-site attribution and the added leaf/tier probes. Direct-frame fallback is
`direct_leaf_frames - standalone_ladder_hit`; general category includes its own
slot-seeded preparation. Counts are entry routes, not completed calls or cost.

| Route | Count | Share of routed JS entries |
| --- | ---: | ---: |
| Numeric closed leaf | 5,757 | 0.0196% |
| This-property closed leaf | 256,117 | 0.8742% |
| Caller-env compact | 723,432 | 2.4692% |
| Caller-env wide | 3,448,249 | 11.7693% |
| Standalone compact | 3,518 | 0.0120% |
| Standalone wide | 2,517,405 | 8.5922% |
| Compact inline | 267 | 0.0009% |
| Wide inline | 18,305,031 | 62.4772% |
| Direct-leaf route reaching general frame | 1,624,367 | 5.5441% |
| General call preparation | 512,698 | 1.7499% |
| Wide inline numeric plan | 1,901,920 | 6.4915% |

## Per-program entry and construction counts

O=legacy ordinary attempts; D=duplicate forwards; C/W=compact/wide actual
inline entries; N=wide inline numeric-plan success; F=frame/VM constructions;
E=all eight CallEnv constructor/clone boundaries. The raw file also records
closed leaves, caller/standalone probes and native families for each program.
Pool requests/constructions are **not malloc counts**; zero-capacity Vec and
recycled vectors may allocate nothing. The construction table identifies actual
Vec expression boundaries rather than all JS arrays/objects or all runtime Vecs.

| Program | O | D | C | W | N | F | E |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lodash/transform | 4,791,101 | 53 | 262 | 3,582,244 | 167,064 | 41,661 | 1,661,638 |
| meriyah/parse | 2,318,162 | 14,301 | 0 | 1,303,449 | 0 | 279,539 | 1,126,557 |
| immutable/persistent | 5,438,251 | 150,005 | 5 | 3,479,402 | 141,220 | 487,770 | 1,873,281 |
| moment/dates | 7,683,658 | 141,600 | 0 | 5,182,519 | 216,000 | 938,664 | 8,972,523 |
| diff/compare | 2,462,190 | 0 | 0 | 867,220 | 0 | 77,297 | 6,256,121 |
| papaparse/csv | 1,077,055 | 3 | 0 | 909,494 | 0 | 97 | 2,725,489 |
| js-yaml/document | 2,198,262 | 0 | 0 | 1,718,413 | 1,367,736 | 220,013 | 1,374,157 |
| big.js/arith | 31,237 | 0 | 0 | 22,530 | 0 | 56 | 144,880 |
| matter-js/simulate | 702,670 | 553 | 0 | 544,237 | 9,900 | 3,156 | 282,036 |
| graphlib/paths | 1,000,803 | 33 | 0 | 695,523 | 0 | 89,264 | 570,655 |

| Construction / entry event | Total |
| --- | ---: |
| CallEnv new | 243 |
| CallEnv from_map | 2,788 |
| CallEnv clone | 72,543 |
| CallEnv fresh function frame | 20,665,887 |
| CallEnv minimal leaf frame | 4,145,298 |
| CallEnv fork current frame values | 100,578 |
| CallEnv detached | 0 |
| CallEnv with_locals | 0 |
| Argument Vec pop_arguments | 7,438,472 |
| Argument Vec tier fallback to_vec | 448,836 |
| Argument Vec slice fallback to_vec | 5,259 |
| Bound argument Vec clone/extend | 0 |
| Local vector direct pool request | 1,624,817 |
| Local vector general pool request | 512,700 |
| Operand vector pool request | 2,137,517 |
| Actual frame/VM construction (includes roots/resume) | 2,137,517 |
| Wide exit reconstructing general VM | 442 |
| Virtual-function call entry | 7 |
| Stream selections | 2,137,448 |
| Virtual stream selected | 553,544 |

These eight CallEnv boundaries total **24,987,337**. Argument transport counts
cover pop_arguments, both tier to_vec fallbacks, slice fallback and bound-list
construction. Semantic spread/apply/rest argument lists and vectors internal to
builtin algorithms are outside that total; no claim of a process-wide Vec census.
The S3d implementation must distinguish ordinary transport deletion from those
required semantic allocations. Some general preparations already seed by slice;
do not add direct-entry and generic counters as independent frame counts.

## Admission attempts and declines

A probe records invocation even if a cached negative fact immediately rejects.
Option<Result> Some(Err) is admitted, not declined. Nested ladder rows overlap;
summing them counts work, not unique calls. Numeric-plan run can decline before
success; its root count includes helper graph routes beyond standalone/wide.

| Probe | Attempts | Admitted/success | Declines |
| --- | --- | --- | --- |
| Direct predicate | 31,728,711 | 8,725,213 | 23,003,498 |
| Numeric leaf | 8,598,962 | 5,757 | 8,593,205 |
| This-property leaf | 8,572,330 | 256,117 | 8,316,213 |
| Compact admit | 9,780,195 | 726,961 | 9,053,234 |
| Wide admit | 9,053,214 | 5,965,654 | 3,087,560 |
| Compact inline callee | 568,007 | 267 | 567,740 |
| Wide inline callee | 30,324,024 | 20,206,951 | 10,117,073 |
| Caller ladder including wide | 7,837,423 | 4,171,681 | 3,665,742 |
| Wide caller entry | 4,911,442 | 3,448,249 | 1,463,193 |
| Standalone ladder including wide | 4,145,298 | 2,520,923 | 1,624,375 |
| Wide standalone entry | 4,141,772 | 2,517,405 | 1,624,367 |
| Standalone numeric plan | 3,518 | 0 | 3,518 |
| Wide numeric plan | 2,922,430 | 1,901,920 | 1,020,510 |
| Numeric plan evaluator | 2,561,158 | 1,901,920 | 659,238 |
| Fast global native | 13,912,880 | 5,953,843 | 7,959,037 |
| Guarded Math unary | 690 | 690 | 0 |

## Backedges and execution reach

| Event | Count |
| --- | --- |
| Generic/wide counted backedges | 1,999,826 |
| Numeric-mutation consultation | 1,391,610 |
| Numeric-mutation accepted | 0 |
| Typed-loop consultation (generic + wide) | 1,999,869 |
| Typed-loop accepted | 608,272 |
| Typed-loop declined | 1,391,597 |
| Both generic accelerators declined | 1,391,554 |
| Wide handback consultation | 31 |
| Wide handback accepted | 31 |
| General dispatched operations | 289,414,045 |
| Compact + wide dispatched operations | 831,246,923 |

There are 1,391,610 generic mutation consultations, all declined; 608,272 typed
entries. Typed consultations total 1,999,869 because wide can consult the typed
executor directly; the legacy backedge total only counts successful wide loop
entries, so it is not an all-executor loop-iteration denominator. Internal loop
iterations do not re-enter these admission functions and are not counted here.
Wide handback is an additional recovery probe, not a new JS call.

## Native families and bypass entries

Probes/declines below include cached-family attempts and the ordered scan.
Terminal means Some or Err; core is entered after every family declines.
The terminal-family total plus core equals legacy native_calls **10,986,795**.
Family 7 has no implementation in this build but is still scanned.

| Family | Probes | Declines | Terminal |
| --- | --- | --- | --- |
| 1 arrays | 4,910,155 | 2,810,556 | 2,099,599 |
| 2 array_buffers | 2,810,556 | 2,810,555 | 1 |
| 3 typed_arrays | 2,810,714 | 2,810,553 | 161 |
| 4 data_views | 2,810,553 | 2,810,552 | 1 |
| 5 disposable_stacks | 2,810,552 | 2,810,552 | 0 |
| 6 atomics | 2,810,552 | 2,810,552 | 0 |
| 7 agent (disabled) | 2,810,552 | 2,810,552 | 0 |
| 8 date | 3,039,735 | 2,810,541 | 229,194 |
| 9 math | 4,119,991 | 2,810,523 | 1,309,468 |
| 10 errors | 2,810,523 | 2,810,523 | 0 |
| 11 json | 2,810,523 | 2,810,523 | 0 |
| 12 maps | 2,831,698 | 2,810,517 | 21,181 |
| 13 sets | 2,810,527 | 2,810,510 | 17 |
| 14 weak_maps | 2,810,511 | 2,810,508 | 3 |
| 15 finalization_registries | 2,810,508 | 2,810,508 | 0 |
| 16 weak_refs | 2,810,508 | 2,810,508 | 0 |
| 17 weak_sets | 2,810,508 | 2,810,508 | 0 |
| 18 numbers | 3,074,905 | 2,810,502 | 264,403 |
| 19 promises | 2,810,502 | 2,810,500 | 2 |
| 20 objects | 4,304,677 | 2,810,467 | 1,494,210 |
| 21 reflect | 2,810,467 | 2,810,467 | 0 |
| 22 regexp | 4,331,076 | 2,810,394 | 1,520,682 |
| 23 strings | 4,047,873 | 2,810,361 | 1,237,512 |
| 24 iterator | 2,810,361 | 2,810,361 | 0 |
| 25 generator | 2,810,361 | 2,810,361 | 0 |
| 26 async await reactions | 2,810,361 | 2,810,361 | 0 |
| 27 async generator methods | 2,810,361 | 2,810,361 | 0 |
| 28 async generator reactions | 2,810,361 | 2,810,361 | 0 |

All family probes **86,869,971**; declines **78,693,537**.
Core calls **2,810,361** are never family-cached; their 28 refusals each account
for **78,690,108** declines. The remaining **3,429** declines are family discovery.
Thus direct native binding has more counted reach than merely avoiding a few
cold installation probes. Counts do not measure probe latency or wall savings.

Fast-native successes bypass that table: Math **786,605**, core/global **2,244,003**,
strings **2,749,444**, numbers **148,483**, arrays **25,308**, total **5,953,843**.
The guarded Math opcode adds **690** successful direct primitive operations.
Total explicit native route entries = **16,941,328**; compile-time-flattened
Math operations are operations, not dispatcher calls. NativeEntry migration
must preserve those primitive guards without keeping the Option family ladder.

## Exact reproduction commands

Run only in this worktree at the source revision, with a clean runtime tree.
The patch is a diagnostic artifact, intentionally not production-format/lint
code. It uses local TLS counters, return wrappers and identical semantic calls.
It must never be merged into runtime or used for timing. No runtime patch is
needed to review the committed results. Reproduction is an explicit temporary
patch/reverse, not checkout/merge of another worktree.

```sh
git apply --check tasks/diagnostics/T077-s3-instrumentation.patch
git apply tasks/diagnostics/T077-s3-instrumentation.patch
cargo build --release -p qjs-cli --features perf-counters --target-dir target/t077-s3-inventory/build
PYTHONDONTWRITEBYTECODE=1 python3 - <<'PYRUN'
from pathlib import Path
import os, json, subprocess, hashlib
from tools.benchmark.screen import resolve_cases, DEFAULT_CACHE
work = Path("target/t077-s3-inventory/programs")
work.mkdir(parents=True, exist_ok=True)
cases = resolve_cases(["development"], DEFAULT_CACHE, work)
expected = json.loads(Path("tasks/diagnostics/T077-s3-counts.json").read_text())
binary = Path("target/t077-s3-inventory/build/release/qjs")
env = {k:v for k,v in os.environ.items() if not k.startswith("QJS_")}
assert [c.id for c in cases] == [r["id"] for r in expected]
for case, record in zip(cases, expected):
    assert hashlib.sha256(case.workload.read_bytes()).hexdigest() == record["sha256"]
    for repeat in range(2):
        run = subprocess.run([str(binary), "--raw", str(case.workload)],
                             capture_output=True, text=True, env=env, timeout=600)
        assert run.returncode == 0 and run.stdout == case.expected_stdout
        tail = run.stderr.split("QJS_PERF_COUNTERS\n", 1)[1]
        counters = {}
        for line in tail.splitlines():
            fields = line.split()
            if len(fields) == 2 and fields[1].isdigit():
                counters[fields[0]] = int(fields[1])
        assert counters == record["counters"], (case.id, repeat)
    print(case.id, "checksum and two full counter maps match")
PYRUN
git apply -R tasks/diagnostics/T077-s3-instrumentation.patch
./scripts/check-context.py
PYTHONDONTWRITEBYTECODE=1 ./scripts/check-touched.sh --staged --explain
```

The source Op mapping is checked by extracting the 120 variant identifiers
between enum Op and its closing brace and comparing them to the first column
of [the map](../proposals/T077-s3-op-map.md). No wildcard/excluded Op family.

Verification of this documentation tree: context, whitespace/diff and staged
fast gates pass; no runtime source/Cargo/script change is committed.
Full check/NG/complete exact scans are **implementation-stage gates, not run
for this inventory**; no new conformance or performance standing claimed.
No commit push or merge. The next decision is lead review of the plan/T074,
not another measurement baseline.
