# S2 round4 structural checkpoints

The [brief](../proposals/T077-s2-prototype-constants.md) freezes the mechanism
and stop rule. Baseline dcee3c56; no held-out. Runtime/order before this round:
2cccf17d. Round3 full/NG/verified/fuzz/RSS records are complete; its cached/off
exact scans were deliberately stopped by the lead, with no complete rows.
Round3 final public86 job 20261031T020157Z-t077-s2-r3-final-public86-r2 completed:
wall/instructions development .946350/.945667, external 1.025969/1.009854,
broad .994925/.983102, sentinel .970819/.944328, all86 1.003464/.989737.
Its first launch failed before collection on Python 3.9 selected by a login
shell; the successful retry preserved the configured runner Python.

## Step 1: 7ef4e68f

Stable prototype-only constants own no RC payload; all heap values and accessor
halves are weak. One NamedStore barrier covers descriptor and value writes,
numeric writes and mutable string access. The fourth value overwrite invalidates
once and switches that holder permanently to live slots; structural edits always
invalidate. Additions already invalidate structurally and do not spend that budget.
Own values stay live. No dependency, unsafe or binding-model change.

2,539 agents+perf-counters runtime tests pass with debug IC verification;
strict clippy passes. Fast gate passes 2,492 default runtime tests and its selected
65 Test262 cases. Seven new tests prove write routes, replacement, instability,
foreign-domain and marked-realm updates, key recycling and dead weak payloads.
Fuzz template adds warmed constant writes and accessor replacement.

Public86 job 20261031T021404Z-t077-s2-r4-step1-public86 encounters disturbance
caps in shards 0/3; runner intruders records searchpartyuseragent at 203.9% CPU.
No performance conclusion from that failed collection. Identical-revision retry
20261031T022044Z-t077-s2-r4-step1-public86-r2 completed with 430 valid pairs, zero invalid/heavy pairs. Wall/instructions:
development .963939/.946158, external 1.023469/1.008991, broad .982468/.979637,
sentinel .917969/.935775, all86 .996760/.987712. Index-of .891615/.913808,
method_call 1.206269/.996196, hash-map 1.103236/.953539; external still loses.
[Report](T077-s2-r4-step1-public86.json). No workload/address tuning.

## Step 2: 1352178a

Miss resolution/admission has a separate module. Cache miss, poly/mega, numeric
fallback, update and debug verification helpers are cold and never inlined.
Typed generic getter/setter misses and interpreter misses are explicitly outlined;
wide/compact property misses already use the shared cold handler. Own store hits
now keep the guard/write inline while add/poly paths stay cold.
2,539 verified runtime tests and fast gate pass (2,492 default, 65 selected
Test262 cases). Exact-image static counts ([base](T077-s2-r4-static-base.json),
[step1](T077-s2-r4-static-step1.json), [step2](T077-s2-r4-static-step2.json))
show typed VM 3,024 → 3,473 → 3,556 and typed Wide 3,006 → 3,474 → 3,566;
wide dispatcher 5,372 → 5,017 → 5,139. Step2 does not yet meet the typed size
target. The final structural unit moves unstable inherited live-slot handling
out of the hot helper and separates ordinary-object specialization from other
receiver families, preserving the same guards/hits in a shared family helper.
Public86 is collecting.

The [static collector](T077-s2-r4-static.py) attributes decoded addresses to
unique nm extents, not printed disassembly labels. The [counter probe](T077-s2-r4-counter-probe.py)
patches archived diagnostic source copies identically by semantic operation,
records hashes/unsupported mechanisms and never times those binaries.


## Step 3: attribution and structural correction

Same-input preliminary arm64 counter builds of dcee3c56 and 1352178a agree on
outputs and tier operations. Raytrace has 602,630 wide named reads, 357,389
computed reads and 83,181 index reads at both endpoints. Its typed dense writes
fail 18/18 and boxed dense writes 870/870 at both endpoints; the own-dense-write
proposal is not supported as the cause. Candidate sends 39,139 array-length
reads through the ordinary miss handler; those unconditional exotic hooks can
be answered before IC lookup. Gaussian has 720,012 generic named reads and
identical tier operations; candidate hot IC attempts rise from 21 to 720,022,
all misses, before the existing typed-array accessor hook. The x86 paired
probe is queued; these preliminary counts are not timing evidence.

The correction puts unconditional array/string length hooks before ICs and lets
the interpreter evaluate the cell-proven native typed-array length accessor
without repeating a descriptor-chain walk. Own shadowing and replaced/custom
getters stay observable; custom getter tests assert exactly one execution.
Typed-loop hoisting still declines accessors. Stable accessor constants weakly
retain their existing pair instead of allocating an Rc pair on every hit;
a pointer-identity test proves reuse, a death test proves no getter pinning.
2,542 debug IC-verified tests pass. No numeric-store or dense-write change is
made without counter evidence.

The step1/2 strict order audit can report stale mangled symbols after new methods
and outlining. After step3 source freezes, regenerate x86 with the same default
external/sentinel/broad set and existing pins, leaving arm64 untouched and doing
no offset/address search; final triage uses that committed order.


Step2 public86 job 20261031T022131Z-t077-s2-r4-step2-public86 completes healthy
(430 valid, zero invalid/heavy): wall/instructions development .971709/.959209,
external 1.031199/1.014235, broad .989974/.981720, sentinel .935974/.934362,
all86 1.005206/.992482. [Report](T077-s2-r4-step2-public86.json).

X86 paired counts complete at 1352178a and 2c47383a: [before](T077-s2-r4-counters-before.json),
[after](T077-s2-r4-counters-after-2c47383a.json). Gaussian instructions drop
7,500,871,478 → 6,602,457,928 (base 6,744,862,640); hot IC attempts
720,022 → 20, typed-array indexed-name tests 360,001 → 0, ID probes
1,081,256 → 1,253. X86 allocations base/after are 41,615/43,294; same-input
arm64 before/after allocations 403,293/43,293 prove 360,000 pair allocations
removed. Raytrace instructions 502,619,366 → 469,317,059 (base 446,837,039);
39,139 array-length misses disappear. Residual +5% instructions remain; dense
write failure counts match base, so the typed dense-write proposal stays deferred.
No per-function dynamic instruction attribution or qualified hardware CPI/cache
miss claim is available.

[2c47383a static](T077-s2-r4-static-2c47383a.json) exposes the inlining cost:
typed VM 3,923, typed Wide 3,945, wide dispatch 6,753. Shared constant-payload
loads are now explicitly non-inlined, and a numeric-store miss calls a cold
boxed-store helper. Guards and ordinary hit loads stay inline. This is the final
cold-boundary correction; no new runtime change follows final public86.
2c47383a passes the fresh full gate/5,170 curated cases, all 268 NG fixtures in
three modes, 300+50 differential seeds and 2,542 verified tests on arm64/x86.
Those are revision-bound results; final corrected source needs its own checks.

Order job 20261031T024025Z-t077-s2-r4-final-x86-order completed for 2c47383a,
463 ranked symbols, SHA 121c0b31b27164cbbfa6948336507a53e6bb7ab1d18eb518e4be15e98c37d86c.
It is superseded before final timing by the last cold-boundary correction; no
performance result used it and no offset/address search occurred.


Final runtime source is dc710e4b. Last correction only outlines shared constant
payload loads and the numeric-store miss frame; source is frozen regardless of
final payoff. Fresh source counters/statics/verified tests run in
20261031T025457Z-t077-s2-r4-frozen-source-counters; default-set x86 order generation
follows in the serialized queue. All post-freeze commits change evidence or the
order file only. A final external loss ends this owner run without another fix.
