# T044: Typed loops on selected virtual scalar banks

- Status: rejected; both frozen screen attempts failed and the budget is exhausted.
- Implementation: runtime and experimental layout restored to 27ca5140 in the
  lead checkout; removal gate pending. Rejected source 102cb2d2 is retained on
  agent/rejected/selected-stream-virtual-bank-attempt2 for diagnosis.
- Verified at: 292e75ff and 102cb2d2, complete local/CI/configured Test262
  checks; exact iMac screen binaries and all pairs. Neither passed the screen.
- Evidence: [frozen plan](../performance-units/selected-stream-readonly-virtual-bank-typed-loops.json),
  two screen records, static/profile and correctness conclusions below.
- Unresolved: performance retention. No formal comparison was run or allowed.
- Next action: verify removal, preserve the generic tools/semantic fixtures and
  ledger, then select another unit from fresh measured evidence. Reopening
  needs a new profile and newly frozen plan, not another run of these candidates.

## Measured boundary

Fresh comparison `20261013T071551Z-campaign-derived-this-27ca5140` covers all 25 broad, six sentinel and
45 external cases. Source `27ca5140`, binary
`b97911efb7bfc31df8bd672922b64888aa16b8c5f2cc049d60f4720e2b94a1c2`. Property_read is `6.0097x` NG and
`0.9981x` old main; array_read is `3.8335x` NG and `0.9981x` old main. Broad/sentinel geomeans versus NG
are `1.078263/1.084706`. External JetStream/Kraken/SunSpider subset geomeans are
`1.247017/0.829364/1.179602`. This is a current standing, not a speed claim. The bundle is complete, no
DISTURBED flag or measurement issues, but its readiness is inconclusive: captured_write,
property_dynamic_read and string-fasta have insufficient precision. No favorable rerun is ordered.

Original broad profiles keep workload bytes/argv and use 16x iterations only for stacks. The first
collector's trailing-newline command was rejected and preserved. Corrected job
`20261013T092227Z-campaign-broad3-valid-receipts` provides six exact-binary receipts; hashes, fractions
and queue identity live in the frozen plan. Inclusive parents/children are not added; the envelope is
not wholly removable. Validation passed before freeze, without changing workload, binary, thresholds or
budget.

Supplemental job `20261013T090655Z-virtual-bank-active-stream-trace` compiles only the existing
perf-counters feature at the same source. All three broad regions compile (`TLOK`) and then report
`TLRUN ... Declined`: property reads, property writes and array reads. The trace is not the exact
release binary's performance evidence. Source inspection explains the missing physical input:
FrameProgramView selects lowered code, while LoopPlanView/typed compilation uses original code. Lowering
replaces object/array alias locals with Undefined and stores actual fields in Vm.virtual_values.
Ordinary invariant value hoisting already exists; adding a duplicate heap-property hoist is not the fix.

## Target architecture and stages

A typed program and its handback belong to one selected executable variant, not just one Bytecode owner.
Original/full/data-only variants cache their own plans; their code and stack/deopt coordinates cannot be
mixed. The LoopFrame boundary reads current virtual scalar slots without materializing a receiver. Wide
frames without a bank retain their original path. Existing numeric mutation plan ownership and frame
restart boundaries remain intact.

1. Establish a selected-stream compile/cache view and a read-only bank input    interface. Preserve all
original/wide behavior, stack effects and restart    ownership; do not add normal dispatch opcodes or a
new hot VM field. 2. Translate eligible lowered virtual loads to existing scalar register inputs. Seed
actual current primitive values on every entry/resume, guarded before    coercion. Reject related bank
stores/init/discard/materialization and unknown    effects across the region. Object fields and
fixed-element arrays share this    scalar-bank path. Preserve exact stack/locals and selected IP on
every exit.

Known receiver/key operand discard on virtual-load instructions is a stack effect to preserve; the
rejected discard means bank lifetime/content clearing. Variant changes must reset or map
declined-program bits; array-index identity from an original/wide cache cannot name a different lowered
program.

Both stages complete before the bounded final screen. Heap-property store promotion, cross-activation
value caching, binding-name ABI, AST/dependencies, unsafe and disabling virtual lowering around
benchmark shapes are excluded. Future bank writes are a separate measured unit, using this
representation.

## Coverage and owner boundaries

Vary names, values, object/array sizes, callers and loop arrangements. Check zero/one/many trips,
current values on repeated activation, multiple variants, ordinary/no-bank wide fallback;
getters/proxies/escapes/ambiguous aliases; non-number values, coercion callbacks and throws; related
writes/reinit/discard; early break/return, deopt/interrupt/residency and exact continuation stack/IP.
All T043 cell/owner and ordinary prototype guards remain regression coverage. Existing diagnostic
counters must prove typed entry, not only equal output.

Owner verification passed 2277 runtime tests, 25 diagnostic tests, agents, 408 tool and 35 script tests,
hooks/file sizes and 5170 Test262 subset cases; 238 NG fixtures also pass. Focused tests cover
separate/shared-code variant caches, clone cache reset, refresh and wide bitmap ownership, current bank
values, residency and fused-edge coordinates. The selected fused increment's edge is `ip + 1 + skip`;
retained Jump discovery is deduplicated, and an unmapped incoming edge rejects the plan. Ordinary VM
handling was correct.

The final debug CLI hash is `7d965c899f353529dcf175c0b5bbea3285191748b874cd40dd3fb56309c82082`;
production tree hash `9169fcaa20827eab5739bc01aea090fee59f16cf8cdea600828be71deab8dbdf`. Independent
checks preserve all 15 production files and five build inputs before/after: 27 scope comparisons match
baseline, 26 match NG (the existing eval gap below), and 17 T043 fixtures match NG. Fresh object/array
inputs enter the bank tier four times per route; early return enters once.
Getter/proxy/string-continuation probes have zero bank entries and establish fallback behavior only. A
final-forwarded Binary test forces a genuine stoppable site, restores its physical prefix/IP and resumes
the VM. Discard2 checks packed, unforwarded input mapping: Move cannot exit in production and copy
forwarding may delete the discarded key. The existing residency bound is tested; this engine has no
interrupt polling API to exercise.

Lead checks at `c9d2f49c` pass the complete local gate (410 tool tests, including the native layout-scan
fix) and 247 NG fixtures. Production files match the final reviewed manifest. CI `38384956641` was
superseded by a documentation update; its replacement `38385253266` at `af86ec70` passed. Queued iMac
jobs `20261013T104052Z-t044-native-layout-scan` and `20261013T104134Z-t044-selected-stream-route` use
that exact source; neither is a frozen-unit screen or a formal performance claim.

The queued layout diagnostic completed with all placements retained, no DISTURBED flag and 18 light
background samples. VM offsets are all `0xc40`; wide offsets are exactly `0/0x200/0x400`. Two-pair wall
medians versus the initial `0x400` candidate (identical instructions, diagnostic only):

| Canary | 0 | 200 | 400 |
| --- | ---: | ---: | ---: |
| recursive_call_tree | .9927 | .9755 | 1.0886 |
| prototype_method_call | 1.0842 | .9571 | .9949 |
| polymorphic_call_site | 1.0117 | .9839 | .9898 |
| capturing_closure_call | 1.0098 | .9704 | 1.0039 |
| heterogeneous_property_read | 1.0042 | .9928 | .9981 |
| string_key_map_churn | 1.0142 | 1.0034 | .9777 |
| ai-astar | 1.0111 | 1.0014 | .9938 |
| imaging-desaturate | 1.0016 | 1.0106 | .9913 |
| access-nsieve | .9859 | .9893 | .9919 |
| access-nbody | .9980 | 1.0139 | .9954 |

All-case geomeans are `1.0110/.9897/1.0021`. Initial base and the 400 executable are byte-identical
(`feb4d88c...`); the recursive range `1.0131..1.1640` there is A/A noise, not a reason to discard a
pair. Sequence, load decline and two pairs limit inference. Adopt the central 200 placement for x86
qualification, retain ARM's unmeasured 400, and perform no further layout search for this unit. Four
call canaries favor 200 in both pairs; the other six have geomean `1.0019`, including the two small
regressions in the table. This fixes an engineering configuration, not a speed verdict.

Supplemental route binary `693d61e1...` at `c9d2f49c` executes property_read 14..39 and array_read
15..44 (`TLRUN Ran`, `TLCLAIM typed`, 13/16 ops). Both have one entry, zero declined edges and no deopt.
Existing constant forwarding makes their bank-entry count zero; the general selected scalar path is what
executes. Property_write rejects its region at compile time because it contains bank stores: zero
entries and 10000 ordinary backedges. All three exit successfully; this is instrumented routing evidence
only.

Final relink `20261013T110043Z-t044-final-layout-and-ng247` has unchanged order, one stable build, VM
c40/wide200 and binary `dd8c5db69db16e55089d4f1d8bda78cf2947e17c811874d0071ff73b235050c2`. Native NG247
initially fails only date-basic's local constructor upper bound. Date and its fixture are unchanged:
Rust's local Date policy is UTC, while NG uses host time. Baseline 27 and candidate agree in the same
native zone; explicit UTC agrees with NG for all 247 in
`20261013T110757Z-t044-default-zone-baseline-and-utc247-fixed-copy`. The native failure remains; this
does not establish host time-zone support. The earlier UTC diagnosis job 110516 did not execute the
candidate because its copied artifact lacked an executable bit; the corrected job copies it into its own
output and verifies unchanged bytes.

Configured full Test262 run `38388016940` binds exact 292 source and CI artifact: 42672 configured,
42672 pass, zero fail/timeout/not_run/actionable gap, 10900 NG exclusions. Raw 53572 results still
contain 5660 failures and two timeouts in that excluded cohort; no full-ES2025 claim is made.

| Attempt | Source / job | Targets wall / instructions | Control verdict |
| --- | --- | --- | --- |
| 1 | 292e75ff / 20261013T111032Z-t044-screen-attempt1 | property .1499/.1910; array .1534/.1908, every target pair <1 | FAIL: captured_write wall1.1734>1.03 |

Captured_write wall pairs are `1.1734/.9975/1.2150/1.2705/.7896`; instructions `1.0154` are stable. All
pairs and launch preflights remain in the screen JSON. Other controls/sentinels pass their median gate;
no DISTURBED flag. This consumes attempt 1 despite variance and the large target gain. No formal run is
authorized yet. Paired profile job `20261013T112024Z-t044-captured-write-paired-profiles` preserves
three samples per exact base/candidate byte set, original workload and bound 16x diagnostic arguments in
AB/BA/AB order; it has no timing verdict.

Paired profiles place all main-thread samples in one outer VM-to-Wide cluster, with Wide self fractions
base `82.36/87.65/87.89%` and candidate `88.80/88.56/86.27%`. VM is an inclusive ancestor, with no
observed exclusive dispatch; removing the view cache pointer alone has no demonstrated causal link to
the captured-write wall failure. Static exact-byte comparisons do show Bytecode payload `1200 -> 1232`,
Wide activation stack `1016 -> 1048`, and +32 offsets for its authority mask and received-upvalue
length. Its hot activation symbol moves page offset `1344 -> 2000`; it is not the pinned typed Wide
executor. Wide source is unchanged, normalized machine code is not. These facts support shrinking inline
cold metadata, not a 17% speed prediction.

Attempt 2 keeps the frozen mechanism/gates: variant tables become lazily boxed on their first real
backward edge; the activation view keeps only owner and actual code again. Original/full/data caches
stay independent, shared code is disambiguated by the stable activation flag, and an unknown code owner
rejects acceleration. No eligibility, opcode, AST, dependency, ordinary heap-store or layout-search
changes are included. Test late authority loss, forward jumps, cache lifetime, clone/reset and existing
handback/route behavior before measuring; preserve cold wide-handback paths.

Attempt 2 owner full gate passes 2283 runtime, 25 diagnostics, agents, 410 tools, 35 scripts and 5170
Test262 subset; UTC NG247 also passes. Independent CLI `22be5cd9...` matches all 44 prior semantic
checks with five production/build inputs unchanged. Local debug-info confirms Bytecode1200, view24 and
variant48; x86 release layout still needs verification. Six new cold/lazy/foreign-code tests pass,
including a genuinely nonempty numeric table rejecting foreign code. No performance verdict is implied.
Owner CI `38394872992` is pending; the original failure is retained.

Nine finite semantic fixtures are retained in `tests/fixtures/compare-qjs/`: fresh values, zero trips,
coercion, getter/proxy effects, writes/reinitialization, early exits, string accumulation and derived
`this`. At `7ad94942` (runtime identical to `27ca5140`) all 247 script-goal NG fixtures pass. These
output comparisons do not prove accelerator entry or a forced exit; the owner's counter and
physical-site tests must establish those separately.

One baseline gap is retained separately as [raw
source](../../tests/fixtures/known-gaps/eval-zero-trip-getter-proxy-tdz.js). At the same revision,
running this source directly returns `0|12|0|15|0|ReferenceError|7` in both engines;
`eval(JSON.stringify(source))` returns `0|12|0|15|0|ReferenceError|NaN` in Rust and the former result in
NG. An arrow IIFE with an explicit return also agrees at `7`; the live zero-trip fixture covers that
local shared-cell scope. The global direct-eval gap is not repaired by this unit and must be compared
separately on the candidate.

The coding owner changes bytecode, its existing tests and necessary counters; the lead owns
docs/tools/order/Cargo. Independent review covers ownership and handback. Correctness is local; every
measurement/profile uses the iMac queue.

Follow [the performance workflow](../../docs/performance-workflow.md). The frozen plan owns
target/control thresholds and its two attempts. Only a passing screen allows formal measurement; full
portfolio and exact-candidate zero-gap Test262 are required before retention. No source/checksum special
cases are allowed.

Attempt 2 (`102cb2d2`, binary `8f125395...`) screen `20261013T124130Z-t044-screen-attempt2-wait-quiet`
fails: property_read wall `.1490`, array_read `.1557`, all target pairs below 1; dynamic_method_call
`1.0545` and string_key_map_churn `1.0776` exceed 1.03. Captured_write is `1.0229`. No DISTURBED flag;
all five pairs are retained. Both attempts count and neither authorizes formal measurement. Prior
dispatch 122614 was cancelled while waiting for another project's CPU load: empty out/log and no
measured cases, so it is not an additional implementation attempt. The workspace queue now waits for a
continuous quiet minute without a forced busy-host start; benchmark thresholds are unchanged.

Final x86 code confirms Bytecode1200 and Wide stack1016, and Wide admission normalized identical to
base; Wide whole-body code and placement remain different. The concrete structure correction succeeded;
its performance gate did not. Exact102 coverage38398145094 again passes configured42672 with zero
fail/timeout/not_run/actionable gap; all53572 raw rows are byte-identical to the first candidate,
including NG-excluded failures. Neither correctness nor target gain overrides the two frozen control
failures.
