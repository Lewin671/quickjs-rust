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
20261031T022044Z-t077-s2-r4-step1-public86-r2 is queued; no workload/address tuning.

## Step 2: 1352178a

Miss resolution/admission has a separate module. Cache miss, poly/mega, numeric
fallback, update and debug verification helpers are cold and never inlined.
Typed generic getter/setter misses and interpreter misses are explicitly outlined;
wide/compact property misses already use the shared cold handler. Own store hits
now keep the guard/write inline while add/poly paths stay cold.
2,539 verified runtime tests and fast gate pass (2,492 default, 65 selected
Test262 cases). Exact-image static counts and public86 follow this checkpoint.

The [static collector](T077-s2-r4-static.py) attributes decoded addresses to
unique nm extents, not printed disassembly labels. The [counter probe](T077-s2-r4-counter-probe.py)
patches archived diagnostic source copies identically by semantic operation,
records hashes/unsupported mechanisms and never times those binaries.
