# T044: Typed loops on selected virtual scalar banks

- Status: active; plan frozen at `e778be0d`, owner implementation in progress.
- Implementation: in progress on `agent/selected-stream-virtual-bank/owner`;
  no candidate is integrated or screened. Runtime work starts after freeze.
- Verified at: `27ca5140`, 2026-10-13, complete 30-block four-worker comparison,
  exact-binary profiles and supplemental instrumented entry trace.
- Evidence: [plan](performance-units/selected-stream-readonly-virtual-bank-typed-loops.json);
  workspace queue/profile jobs and numbers below. Raw files are not retained.
- Unresolved: candidate implementation, screens
  and complete promotion. The discovery run has three imprecise controls.
- Next action: complete the two structural stages on the owner branch,
  independently review variant/handback semantics, then use the frozen screen.

## Measured boundary

Fresh comparison `20261013T071551Z-campaign-derived-this-27ca5140` covers all
25 broad, six sentinel and 45 external cases. Source `27ca5140`, binary
`b97911efb7bfc31df8bd672922b64888aa16b8c5f2cc049d60f4720e2b94a1c2`.
Property_read is `6.0097x` NG and `0.9981x` old main; array_read is `3.8335x`
NG and `0.9981x` old main. Broad/sentinel geomeans versus NG are
`1.078263/1.084706`. External JetStream/Kraken/SunSpider subset geomeans are
`1.247017/0.829364/1.179602`. This is a current standing, not a speed claim.
The bundle is complete, no DISTURBED flag or measurement issues, but its
readiness is inconclusive: captured_write, property_dynamic_read and
string-fasta have insufficient precision. No favorable rerun is ordered.

Original broad profiles use unchanged workload bytes and bound CLI arguments,
with 16x calibrated iterations solely to collect stacks. Three property_read
samples have totals `1658/1658/1661`; largest virtual dispatch branches
`896/933/925`. Array_read totals are `1661/1654/1662`, branches `920/908/927`.
Parents and children are not added and this envelope is not wholly removable.
The first receipt collector retained a trailing newline in a bash argument;
validation rejected that command format. It remains recorded; a corrected
collector re-executes the same inputs to generate valid receipts, without
changing workload, candidate bytes, performance thresholds or attempt budget.
Validated receipts from `20261013T092227Z-campaign-broad3-valid-receipts` have
property_read totals/branches `1666:903,1665:932,1670:930` and array_read
`1670:918,1670:962,1668:945`. Queue SHA
`4d458d5657079cdb4e11478b743cb68788d1c1ad646fc55f0b85d5b9a949b712`;
plan SHA `1b1542803a0701442264af348653b9fd8f18d83d679417a7f563dd485e02a91d`.
The validator checks all six profile receipts against the exact queue binary.

Supplemental job `20261013T090655Z-virtual-bank-active-stream-trace` compiles
only the existing perf-counters feature at the same source. All three broad
regions compile (`TLOK`) and then report `TLRUN ... Declined`: property reads,
property writes and array reads. The trace is not the exact release binary's
performance evidence. Source inspection explains the missing physical input:
FrameProgramView selects lowered code, while LoopPlanView/typed compilation
uses original code. Lowering replaces object/array alias locals with Undefined
and stores actual fields in Vm.virtual_values. Ordinary invariant value
hoisting already exists; adding a duplicate heap-property hoist is not the fix.

## Target architecture and stages

A typed program and its handback belong to one selected executable variant,
not just one Bytecode owner. Original/full/data-only variants cache their own
plans; their code and stack/deopt coordinates cannot be mixed. The LoopFrame
boundary reads current virtual scalar slots without materializing a receiver.
Wide frames without a bank retain their original path. Existing numeric
mutation plan ownership and frame restart boundaries remain intact.

1. Establish a selected-stream compile/cache view and a read-only bank input
   interface. Preserve all original/wide behavior, stack effects and restart
   ownership; do not add normal dispatch opcodes or a new hot VM field.
2. Translate eligible lowered virtual loads to existing scalar register inputs.
   Seed actual current primitive values on every entry/resume, guarded before
   coercion. Reject related bank stores/init/discard/materialization and unknown
   effects across the region. Object fields and fixed-element arrays share this
   scalar-bank path. Preserve exact stack/locals and selected IP on every exit.

Known receiver/key operand discard on virtual-load instructions is a stack
effect to preserve; the rejected discard means bank lifetime/content clearing.
Variant changes must reset or map declined-program bits; array-index identity
from an original/wide cache cannot name a different lowered program.

Both stages complete before the bounded final screen. Heap-property store
promotion, cross-activation value caching, binding-name ABI, AST/dependencies,
unsafe and disabling virtual lowering around benchmark shapes are excluded.
Future bank writes are a separate measured unit, using this representation.

## Coverage and owner boundaries

Vary names, values, object/array sizes, callers and loop arrangements. Check
zero/one/many trips, current values on repeated activation, multiple variants,
ordinary/no-bank wide fallback; getters/proxies/escapes/ambiguous aliases;
non-number values, coercion callbacks and throws; related writes/reinit/discard;
early break/return, deopt/interrupt/residency and exact continuation stack/IP.
All T043 cell/owner and ordinary prototype guards remain regression coverage.
Existing diagnostic counters must prove typed entry, not only equal output.

Nine finite semantic fixtures are retained in `tests/fixtures/compare-qjs/`:
fresh values, zero trips, coercion, getter/proxy effects, writes/reinitialization,
early exits, string accumulation and derived `this`. At `7ad94942` (runtime
identical to `27ca5140`) all 247 script-goal NG fixtures pass. These output
comparisons do not prove accelerator entry or a forced exit; the owner's
counter and physical-site tests must establish those separately.

One baseline gap is retained separately as
[raw source](../tests/fixtures/known-gaps/eval-zero-trip-getter-proxy-tdz.js).
At the same revision, running this source directly returns
`0|12|0|15|0|ReferenceError|7` in both engines; `eval(JSON.stringify(source))`
returns `0|12|0|15|0|ReferenceError|NaN` in Rust and the former result in NG.
An arrow IIFE with an explicit return also agrees at `7`; the live zero-trip
fixture covers that local shared-cell scope. The global direct-eval gap is
not repaired by this unit and must be compared separately on the candidate.

One coding owner may change `crates/qjs-runtime/src/bytecode/` and its existing
runtime tests, plus necessary diagnostics counters only. The lead owns plans,
docs, scripts, order files and Cargo. A read-only reviewer checks lowering,
entry guard, variant ownership and every handback. Correctness runs are local;
all profiles, layout scans and measurements use the shared iMac queue.

Follow [the performance workflow](../docs/performance-workflow.md). The frozen
plan owns target/control thresholds and its two attempts. Only a passing screen
allows formal measurement; full portfolio and exact-candidate zero-gap Test262
are required before retention. No source/checksum special cases are allowed.
