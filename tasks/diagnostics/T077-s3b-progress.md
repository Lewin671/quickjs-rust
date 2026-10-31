# T077 S3b implementation record

Implementation base: `1e6e2a2b8ccdb590d735e5b6e969105cb1344c72`, accepted
S1+S2 integration (runtime `dc710e4b`, acceptance `fc3deef7`). Frozen brief:
`71294a20864662808216f874635dc8fb0473693e`. Owner branch:
`agent/t077-s3b-context/codex`. User authorizes local commits only, no push/merge.
The integrated S2 semantic API is the input; Context does not access NamedStore
internals. Prototype/layout follow-up paths remain outside this owner boundary.

Predecessor performance endpoint is this accepted S1+S2 base; cumulative endpoint
is `6663738f` per T077. Lead owns exact timing binaries/corpus/recipe pinning and
formal/held-out/RSS acceptance; owner collects public86 quick triage via the iMac
queue only. No performance standing is claimed by this record.

## Pre-code inventory recount

The frozen regex over runtime Rust sources counts **198 files / 2,122 tokens**
for CallEnv at the base, versus 196 / 2,112 at design. Delta by family:
array 18/179 (+1 token), bytecode 32/208 (+1 file/+6 tokens), function 10/92
(-1 token), global.rs 1/46 (+1 token), object 11/94 (+1 file/+3 tokens).
All other families are unchanged. The 98 inherent methods remain (including
one test-only cache query); Clone/Debug remain separate. S3a native bind arm
surface is unchanged. Migration/deletion authority remains the frozen brief.

## Global accessor binding repair

Base reproducer `/tmp/qjs-s2r/t/glob6.js`: `a 1 1`, `b 1 3`, `c 1/1 4`,
`d 1 5`, `e 1 6`, `top 1 7`. The setter ran, but apply-back incorrectly
published its assigned value as a realm data binding. The sync helper now
checks the current own descriptor and invalidates the realm data route for an
accessor without invoking the getter/setter again. Audit also found interpreter
`set_property_value` and compact `property::set_named` publishing the assigned
value after successful [[Set]], including setter success. Both now inspect the
resulting own descriptor (setter may redefine/delete it), publish only its data
value and invalidate otherwise. This is an additional repair site within the
brief's binding-authority bug, not an architecture deviation.
Permanent fixture checks alias/globalThis/identifier writes and exact getter
count. Lowest-layer runtime regression and all **269 NG fixtures** pass on the
working tree. Staged fast gate passes (including 65 selected Test262 cases),
combined agents/perf-counters CLI compile passes; exact variable-statement slice
passes 178/178 with zero gaps. This filtered scan is not a complete-suite scan
and does not create a burndown entry. No complete correctness or timing standing
is claimed for this preliminary repair.

## B1 binding-plan migration

BindingPlan replaces FrameInitPlan. The existing compiler records typed scope
IDs and declarations (parameter/body/block/catch/loop/class-name/with), including
future TDZ declarations in active scope descriptors. Exposure sites retain only
their active scope IDs. Immutable metadata contains declaration kind, write/init
policy and eval deletability; B2/B3 connect these to live cells and leases.
Capture routes are linked at body compilation for functions and class families,
with received order preserved and parent locals/upvalues addressed positionally.
Mapped-arguments duplicate selection is compiled once instead of scanning the
parameter AST on every arguments-object creation. Raw IR constructors are now
test-only; production bodies consume compiler scope metadata in one preparation.
No runtime ABI switch, CallEnv deletion or repaired T074 semantics is claimed
at B1. Six plan/descriptor tests pass; default/agents/perf-counters compilation and
269 NG fixtures pass. The fast gate exposed an old raw-IR test whose explicit
parameter slots were out of bounds of its empty locals; the test now supplies
valid parameter slots while preserving its duplicate-position assertion.
Final fast gate is in progress.
