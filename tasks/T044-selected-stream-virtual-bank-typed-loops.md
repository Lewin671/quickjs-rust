# T044: Typed loops on selected virtual scalar banks

- Status: active; plan frozen at `e778be0d`, candidate integrated for checking.
- Implementation: owner `0a95c1cc`, integrated as `c5f35035` on the lead branch;
  not promoted to main or screened. Runtime work started after freeze.
- Verified at: owner `0a95c1cc`, full local gate/NG238 and CI `38384175694`;
  lead `c9d2f49c`, full local gate and NG247; final debug CLI `7d965c89`.
- Evidence: [plan](performance-units/selected-stream-readonly-virtual-bank-typed-loops.json);
  workspace queue/profile jobs and numbers below. Raw files are not retained.
- Unresolved: lead CI, queued layout qualification, screens and complete
  promotion. The discovery run has three imprecise controls.
- Next action: collect queued native layout scan/route diagnostics, re-pin
  and use the frozen screen after CI. No screen attempts are used yet.

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

Owner verification passed 2277 runtime tests, 25 diagnostic tests, agents,
408 tool and 35 script tests, hooks/file sizes and 5170 Test262 subset cases;
238 NG fixtures also pass. Focused tests cover separate/shared-code variant
caches, clone cache reset, refresh and wide bitmap ownership, current bank
values, residency and fused-edge coordinates. The selected fused increment's
edge is `ip + 1 + skip`; retained Jump discovery is deduplicated, and an
unmapped incoming edge rejects the plan. Ordinary VM handling was correct.

The final debug CLI hash is
`7d965c899f353529dcf175c0b5bbea3285191748b874cd40dd3fb56309c82082`;
production tree hash
`9169fcaa20827eab5739bc01aea090fee59f16cf8cdea600828be71deab8dbdf`.
Independent checks preserve all 15 production files and five build inputs
before/after: 27 scope comparisons match baseline, 26 match NG (the existing
eval gap below), and 17 T043 fixtures match NG. Fresh object/array inputs
enter the bank tier four times per route; early return enters once.
Getter/proxy/string-continuation probes have zero bank entries and establish
fallback behavior only. A final-forwarded Binary test forces a genuine
stoppable site, restores its physical prefix/IP and resumes the VM. Discard2
checks packed, unforwarded input mapping: Move cannot exit in production and
copy forwarding may delete the discarded key. The existing residency bound
is tested; this engine has no interrupt polling API to exercise.

Lead checks at `c9d2f49c` pass the complete local gate (410 tool tests,
including the native layout-scan fix) and 247 NG fixtures. Production files
match the final reviewed manifest. CI `38384956641` is pending. Queued iMac
jobs `20261013T104052Z-t044-native-layout-scan` and
`20261013T104134Z-t044-selected-stream-route` use that exact source; neither
is a frozen-unit screen or a formal performance claim.

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
