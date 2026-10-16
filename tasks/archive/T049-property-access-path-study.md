# T049: Property-read path cost study with the adopted compiler recipe

- Status: closed-rejected.
- Implementation: candidate reverted; production runtime remains byte-for-byte
  source-identical to `21e55fe2`. Candidate preserved at `c1c6d13b`.
- Verified at: base `21e55fe2b530be09ea0453fcb87cde3b36423db3`, 2026-10-16;
  30-block current-recipe comparison, exact profiles and feature-only counts.
- Evidence: local artifact, not retained: workspace queue job
  `20261016T101305Z-property-current-baseline`.
- Unresolved: causal attribution of code-layout regressions; broader object
  representation and loop-executor costs are not resolved by this study.
- Next action: none for this exhausted unit. Future work needs fresh
  layout/profile evidence and a newly frozen plan.

## Scope

User-requested research into repeated property-cache probing and reuse of
receiver-own-property-miss proofs. Primary owner: `agent/property-path-study/root`,
workspace worktree `worktrees/quickjs-rust/property-path-study`, base above.
Production candidates, if justified, remain separate mechanisms. Do not repeat
T041's rejected hot-own-slot implementation or the old T033 prototype shortcut
without fresh profiles and a newly frozen plan. Preserve slots/shared cells,
property semantics, getter/Proxy behavior and complete test/benchmark inventories.

Allowed paths: this task, its plans/index rows, focused property/VM/cache code
and tests only after a validated frozen plan. Diagnostic counter additions may
be temporary and feature-gated; never time instrumented executables. No benchmark,
manifest, acceptance-rule, compiler recipe or unrelated runtime changes.

Every benchmark/profile runs through the workspace iMac queue. Timing uses
wall time on this host; instructions are supplemental. Fresh base/base/NG
comparison uses 30 blocks and four qualified workers. Existing main worktree
changes and the other campaign owner's worktree are outside this task.

## Source review before measurement

- Compact wide named reads call `get_prop_named_hot`, then call the cold
  `get_prop_named` on a miss; that function repeats the ordinary-object guards
  and `probe_hot`. Any candidate must preserve direct callers such as captured
  class-field initializers; removing guards from all entry points is unsound.
- Ordinary `Vm::get_named_prop` can try the direct-local cache route, then
  load the same authoritative local and try the cache again. Count actual
  first-probe failures before assigning this path any priority.
- Compact prototype reads record/reuse receiver-own-miss proofs. The ordinary
  VM path does not itself populate them. A proof keyed by one receiver only
  helps repeated receiver identities; the prototype sentinel rotates 64
  receivers, and older T033 attempts already showed low reach and regressions.
- T041's OwnSlot hot-answer candidate is historical negative evidence, not
  a preapproved replacement for the repeated-probe investigation.
- The pinned NG stores property names as atoms and uses shared shape metadata
  plus per-object value slots (`quickjs.c`: `JSShape`, `find_own_property`).
  This project shares literal shapes but also uses small/dynamic name storage.
  These are structural differences, not a measured attribution of current time.

Diagnostic scripts live in workspace `tmp/quickjs-property-study/`; exact
binary profiles are queued as `20261016T101515Z-property-exact-profiles`, and
feature-only counters as `20261016T101622Z-property-path-counts`. Counters also
track hypothetical same-receiver proof reuse without changing the real cache
answers. Diagnostic output must equal the original executable's stdout.

## Current-recipe evidence

The 30-block base/base/NG comparison completed in job
`20261016T101305Z-property-current-baseline`: all 76 cases valid, no imprecise
case or measurement issue; workers-4, wall time. Six light-background samples
were below the disturbance threshold; no DISTURBED result. Both logical
roles have SHA-256 `926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`.
Lane/suite candidate/NG ratios: broad 0.979834, sentinel 1.040336,
JetStream subset 1.163092, Kraken 0.775633, SunSpider 1.088081.
Same-binary candidate/base broad and sentinel means are 0.998839/1.005850;
small point changes are not automatically implementation effects.

Fresh queue SHA-256:
`d1c2d3c4515c711968ac2f42561311a318e57c8f07629f569c91bd5f8ad6fc08`.
Exact hash-map profiles show cold named-read envelopes 59/484, 69/495 and
63/490 main-thread samples. These include necessary work, not removable time.
The temporary feature-only diagnostic passes stdout equality on all 15
selected programs/cases and records 3,780,111 repeated ordinary-object compact
hot probes in hash-map, 1,067,489 in CDjs and 765,509 in class-field raytrace.

The ordinary VM direct-local repeated-probe route occurs only twice in
hash-map and zero times in the other four external programs. Hypothetical
same-receiver missing-own-property proof hits are 35,998 in CDjs, two in
hash-map and zero in the other selected external programs. Most sentinel
loop work is handled by loop plans, not the generic cache handlers; these
counters do not claim there are no actual property accesses. These two VM
leaves do not currently justify independent production candidates.

[The frozen single-hot-probe unit](../performance-units/compact-named-read-single-hot-probe.json)
validates against that queue and three exact profile receipts. It gives one
attempt, hash-map <=0.98, every other case <=1.03. No runtime edit existed
when the plan was frozen. Compiler recipes, order files and portfolios stay
fixed. This is a research candidate, not a retained optimization.

## Candidate correctness before timing

The candidate removes only the second `probe_hot` in the compact cold named
read. Captured class-field initializers explicitly perform their first probe;
wide callers already do so. No cache entry kind, layout guard, object storage,
compiler setting or order file changed. Three focused tests cover descriptor
and prototype changes, Proxy/primitive/null fallbacks and live captured-member
field initialization.

Transport snapshot `a9e0f56d3061b5d19fed6b82b5aef060589a7dcd` passed the fast and full gates,
then the exact optimized CLI passed all 5170 curated Test262 cases and all NG
fixtures under UTC/C locale in job
`20261016T104613Z-property-single-probe-correctness`. Subsequent task prose
changes do not alter any Rust/build input. Optimized candidate SHA-256:
`c07f0af58c956041e966bdf8b4e40e8f1599188280ce1b8f5d00a2bd9a1f232a`.
Timing is still pending; this is not a retention claim.

## One screen and closure

Candidate `c1c6d13b1e54416e731a95078e10bc91b291d318`, binary SHA above, ran
job `20261016T105327Z-property-single-probe-screen` with all 76 cases, five
pairs each. Logical engine paths have equal lengths. Both actual optimized
binaries also pass the three focused semantic assertions before timing.
Load average 1.15 before/1.10 after; no disturbance or load warning. All five paired ratios
are retained; no favorable rerun was performed.

Hash-map wall ratios: `[1.006519, 0.982182, 0.977115, 0.991311, 0.981446]`;
median `0.982182` misses the frozen `0.98` target, and one pair fails to
improve. Control medians also fail: branch_arithmetic `1.085409`, global_read
`1.124588`, local_read `1.083982`, property_dynamic_read `1.031830`.
CDjs median is `0.993893`; class-field raytrace `0.989708`.
Geomeans of case medians: broad `1.012411`, sentinels `1.006730`, external
`0.998252`. These are screening observations, not formal decision evidence.
The screen exits 3/FAIL, exhausts the one-attempt budget and does not receive
a 30-block candidate formal run. No optimization is retained.

Static address-range disassembly audit in job
`20261016T110950Z-property-machine-code-audit` confirms a code-size side effect:
compact cold named read shrinks 1936->1312 bytes (adjacent-symbol distance,
including alignment); field initializer grows 512->1104. The ordinary VM entry
moves from `0x100115020` to `0x100115270` (+592 bytes), while its size is still
9360 bytes and its instruction-mnemonic sequence is identical. Wide executor
address/size/mnemonic sequence are unchanged. This is not a byte/operand
identity claim, and no layout-only counterfactual was timed: layout is a
supported explanation to investigate, not a proven sole cause of regressions.
Two earlier static extraction scripts failed to find local symbol labels;
the address-range extraction succeeded without any performance remeasurement.

Candidate fast/full gates, optimized 5170-case Test262 subset, UTC NG fixtures
and CI38890193342 pass. The candidate was not rescanned on all 42672 configured
Test262 cases and has no formal promotion standing. Restoring its two runtime
files makes every Rust/build/tool/benchmark input identical to `21e55fe2`;
restoration snapshot `e70a064cb2a872d8156c689deec5b677e2f69a3d` passes the fast and full gates in job
`20261016T111505Z-property-study-restoration-check`, including all 5170 curated
cases. Closure after that check changes only task prose, not any build input.

## Deliverables and next research direction

The Chinese report and reproducible input/result tables are local artifacts,
not retained in the repository: workspace `outputs/quickjs-property-study/`
contains `report.md`, `study.json`, `screen-cases.csv`, profile counts and the
candidate patch. Queue directories named above retain raw pairs, binaries,
profiles, diagnostic patches and execution-script snapshots.

The tested duplicate-probe deletion does not justify production adoption.
The ordinary VM leaf/proof changes have low observed reach, and the generic
cache edits do not address most sentinel loop work. Prioritize build-specific
hot-symbol/layout verification, then price the actual loop/wide execution
paths. Integer property-name IDs, shared shape coverage and value transport
remain structural hypotheses, not measured benefits or a reason to rewrite
objects/GC now. Historical rejected hot-own-slot and prototype shortcuts stay
closed until new evidence supports a separate plan.
