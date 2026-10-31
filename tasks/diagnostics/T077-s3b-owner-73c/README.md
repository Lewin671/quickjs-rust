# S3b owner checks at 73c93d8f

Candidate `73c93d8f`, baseline main `3cc12c6f`. This record is in progress;
complete exact scans and valid public86 triage remain required. Lead alone runs
formal/held-out/RSS and accepts/integrates. No origin push or merge into main.

The full `check.sh` passes at this source, including **5,171 curated Test262
cases**. Default runtime **2,636**, agents/perf-counters **2,691**, each separate
binding stress mode and IC verification **2,636** pass. Combined binding modes
plus IC-off pass **2,594 semantic/identity tests**; 42 cache-enabled mechanism
checks instead run in the complete IC-verification-on suite. One initial combined
run retained a new main cache-prediction assertion and failed it; it is explicitly
classified with those IC-enabled checks. No repository test or Test262 exclusion
is added. All **299 NG scripts** pass separately in default, IC-off, verify,
forced exposure, released-scope poison and combined configurations.
[Modules](modules.json): each of the three .mjs fixtures matches Rust/NG exit and
stdout in all six modes with TZ=UTC, including job completion.

[Base](base-counts.json) and [candidate](candidate-counts.json) reproduce all ten
pinned development inputs/stdout/full counter maps twice. They contain input,
stdout, binary and repeated full-map hashes. Manifest SHA-256 is
`2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.
The identical [baseline instrumentation](../T077-s3b-round1/base-instrumentation.patch)
is applied only to a source archive of main, after its production depth builds.
From the repository root, [generate inputs](generate-inputs.py), then invoke
`collect-counts.py --binary <binary> --source <sha> --output <file>`.
The collector uses release + perf-counters and removes QJS_*
environment variables; these binaries never supply timing results.

| Actual event | Base | Candidate |
| --- | ---: | ---: |
| CallEnv constructions | 19,031,946 | 0 |
| CallEnv clones | 72,543 | 0 |
| Caller-environment equality proofs | 26,477,573 | 0 |
| Binding snapshot calls | 72,101 | 0 |
| Frame fork calls | 100,578 | 0 |
| Apply-back calls | 5,082,372 | 0 |

Historical **24,987,337** belongs to the older `dd0905b2` count receipt; current
predecessor construction/clone events total **19,104,489**. These are API/event
counts, not avoided mallocs. Candidate new work stays separate: **457,653** actual
CellStorage allocations (including global caches), **328,929** slot promotions,
**5,368** ScopeLease allocations; **2,137,508** local pool requests cause **673**
new local-vector allocations. Logical token events **29,298,771** equal
**29,298,761** routed ordinary JS entries plus ten host roots. Ordinary host
forwardings remain **199,691**, without double-counting entries. S3a native
argument-pop/family-probe/decline counts remain zero on both endpoints.

[Base depth](base-depth.json), [candidate depth](candidate-depth.json) and
[frozen landmarks](depth-landmarks.json) use the original S3a probe on ordinary
CLI main-thread stacks, default debug/release builds, no stack override or new
engine guard. Every one of the sixteen base-success maxima and each successful
frozen landmark passes on candidate. The 4096 cap is a probe bound.

| Release case | Base maximum | Candidate maximum |
| --- | ---: | ---: |
| forEach / map / reduce | 796 / 790 / 829 | 872 / 865 / 919 |
| call / apply / Reflect.apply | 3234 / 2066 / 2188 | 3338 / 2160 / 2294 |
| yield* | 714 | 784 |
| plain recursion inside callback | >=4096 | >=4096 |

Both architectures build production default/agents/perf-counters configurations.
[Metadata](metadata.json) records available frozen binary hashes. Six release
warnings are inherited from accepted main's debug-only identity/layout helpers.
[Deletion audit](deletion-audit.json) finds no removed execution identifier and
retains all **592 NativeEntry arms**. Dependencies, third-party pins, layout and
IC constant-admission submodules match main; iterative allocation-free cycle
walkers and their tests remain. The x86 order has its own
[merged-source generation receipt](../T077-s3b-merged-order/README.md).

Final exact/burndown and public86 result links will be recorded separately after
they complete. Public86's first merged attempt collected no measurements because
the qualified `iMac.local` hostname now reports `iMac.lan`; a full host audit
matches every other identity/recipe field. Qualification remains pending an
explicit decision; thresholds have not been changed.
