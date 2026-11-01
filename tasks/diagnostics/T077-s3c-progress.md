# T077 S3c implementation progress

## Authority and checkpoint

Base runtime `8322fc98`; lead decision `6ae8a986` in the
[brief](../proposals/T077-s3c-brief.md#lead-decision-2026-11-01). C0–C2 are
authorized in order, followed by counts/correctness/public86 and a stop for
lead review. C3–C5 and the entry experiment are not started. No origin push or
main merge; temporary runner refs are authorized measurement transport only.
Hard final gate: all four aggregates <1 vs base, RSS 1.10 per case / 1.05
geomean and full correctness. External <=0.983 is a target; any remaining
fixed-baseline deficit carries to S3d.

## C0: fresh class constructor prototype links

Implementation: fresh compiled class constructors initialize NamedStorage with
their selected parent, before publication. The general mutation API keeps
all cycle/extensibility checks. Ordinary constructor/static/prototype identities
and attributes remain live. Focused tests include later rejected cycle mutation,
live inherited static replacement, and extends null; an NG fixture covers the
same observable behavior.

Deterministic perf-counters test at this C0 source tree: N=32/64/128 subclasses
produce 32/64/128 creation links and **0/0/0 extra cycle-walk node visits**
over the empty-realm initialization control (68 visits). The generic existing
function mutation test still visits >=3 additional nodes and rejects the cycle.
There is no local timing. Counters exist only under perf-counters.

Checks: runtime suite 2,729/2,729; focused perf-counters 2/2 (including the
N/2N/4N test). NG default / QJS_DISABLE_IC=1 / QJS_VERIFY_IC=1 each pass 320/320 fixtures.
The staged fast gate passes (format/lint, runtime and 117 selected Test262
cases). All checks refer to the C0 source in this standalone commit.
**C0 is ready for the lead to land separately.** Continue directly with C1.

## C1a: owning body and frame transfers

C0 standalone commit **86dbf2cc** is ready for separate lead landing.
C1a replaces the borrowed/shared body enum by an 8-byte Rc owner, using the
callee's existing Rc when positional/general call inputs identify it and
acquiring root ownership once at borrowing APIs. Generator start/prologue
retain their existing Rc body. `FrameState` loses its caller lifetime; current,
parked and pending-entry frames transfer stable boxes instead of 448-byte
aggregates. Every old dormant-driver semantic test remains; pointer-identity
assertions verify both boxes survive a switch. Root lifetime and function-body
identity have focused tests. Mechanical Vm signature changes remove the old
bytecode lifetime throughout its consumers.

This is the first reviewable C1 sub-cut, **not complete C1**: the fieldwise
header/directory/block pool and subsequent C2 tier consumers are still next.
Runtime 2,731 and perf-counters 2,781 pass at this source tree. Activation
poison + binding verification passes all 2,731 tests. NG passes 320/320; one
loaded-host run timed out on named-property-iterative-release at the unchanged
10-second limit, then the unloaded rerun passed. The fast gate passes with
65 focused Test262 cases. Generator resume also retains the existing Rc body. No performance standing yet.

## C1b: common header, directory and bounded general storage

C1a is committed at **59708713**. C1b installs the approved 64-byte header
(program/block/context/cold owners, three typed call refs and scalar continuation
fields), with compile-time size/alignment/offset checks on both production
targets. General frames initialize pooled records fieldwise. RuntimeServices
owns FrameStore; active records detach into driver locals and parked callers
retain generation-checked IDs. ID exhaustion is a structured entry error; max
generation retires the slot permanently. Views/leases/cells retain no record
address. Dropping a parked caller releases its record; re-entry borrows no
directory or pool across JavaScript.

One pool replaces Bytecode's operand/local/cold recyclers and their handles;
the obsolete ir_pools module is removed. The 64-pair / 256-KiB / 16-KiB bounds
account for record, context, register/local/operand/route/virtual capacities,
cleared cold banks and conservative free-entry bookkeeping. Directory peak
capacity is reported separately. Released bytes are cached for O(1) checkout;
clearing drops all program/realm/cell/lease owners before borrowing the pool.
Poison stamps both header and value banks and is checked before checkout.
No escaped cell/owner identity is pooled, even when its former record is reused.
Local and route vector capacities are reused without callee/name matching.

Every previous storage/native-borrow/dormant-driver test remains and now tests
the common owner. New tests cover same-box/block reattachment after directory
growth, stale IDs, abandoned parked frames, generation overflow/resource error,
poison corruption, count/byte bounds and escaped-cell identity.

At the C1b source in this commit: full check passes (2,736 runtime tests,
5,174 curated Test262, all tooling/feature checks); full perf-counters 2,786;
combined-all (binding verification, forced exposure, both poisons, IC verification)
2,736; x86_64 check including the layout assertions passes. NG default / IC-off / IC-verify each pass 320/320. The staged fast gate
passes, including 65 focused Test262 cases. C2's register TLS/ActivationStack and eager context
protocol still exist; C3/C4 and the measurement checkpoint have not started.

## C2: register consumers and lazy Context

C1b is committed at **52422927**. Both register drivers now use the same
ActiveFrame/FrameStore owners as general execution: each callee gets a zero-based
bank, calls park the caller ID, and native argument borrows pin its detached
block. Wide-to-general handoff transfers that same record, ID and block. General
initialization is recorded once, preserving cells, handlers and cold state across
later handbacks. Identity assertions extend the existing owner tests.

CallSeed owns receiver/new.target/home inputs and the callee prefix. Header refs
select inline, lexical, absent or Context authority. First real Context demand
moves the inputs into one owner; it never mirrors or applies back values. The
callee supplies ordinary creation metadata; dynamic creation selection freezes
at entry, and lexical calls retain the original shared owner. A new counter test
compares 3 and 3,000 captured-write calls and proves constant Context construction.
Three seed tests cover lazy metadata, lexical identity and dynamic selection.

Deleted: ActivationStack and its two owners, both register TLS owners, the
services activation reuse pool, four reuse helpers, eligibility methods,
class/method rebinding helpers and both depth-rebind macros. Context continuation
reconstruction is gone. The three former reuse/poison tests were ported to the
common storage owner rather than dropped. There is no new dependency, unsafe
code, CLI stack override or change to admission semantics.

Initial C2 code failed the callback-depth gate. Opcode preparation temporaries
were outlined into a segment that returns before callbacks; root setup and cold
error/fallback state were outlined too. Removing an unused 128-byte root Context
output slot recovered the last release-depth deficit. Final debug/release depth
meets or exceeds `8322fc98` in every probe; [depth receipt](T077-s3c-c2/depth.json)
records both binaries and all boundary trials. This does not close S3d's earlier
debug deficit versus `3cc12c6f`.

The first instruction screen was substantially worse, not a payer. Avoiding
primitive Value drop dispatch and unused general-state clearing reduced overhead,
but did not recover entry/return cost. The [checkpoint receipt](T077-s3c-c2/README.md) owns the final
109-program counts, N/2N call slopes, correctness and iMac public86 results.
C3/C4/C5 have not started. C0 remains ready for the lead to land separately.

At final C2 source: the staged fast gate passes with 117 focused Test262 cases;
the full gate passes with 2,739 runtime tests, all tooling/agents checks and
5,174 curated Test262. Full perf-counters passes 2,790 and x86_64 check passes.
The 109-program instruction census and 500-seed/profile fuzz complete; the
receipt records the regressions rather than claiming recovery. All seven stress/poison modes pass 2,739; combined IC-off passes 2,697 with
the unchanged 42 cache-mechanism exclusions. NG passes 320 scripts and 10 modules
in each of nine modes, with extended-budget combined runs recorded explicitly.
C2 is committed at **63b2347f**. The corrected optimized-verifier public sweep
passes 351/351; the first unoptimized timeout attempt is preserved separately.
The single iMac triage **20261101T210008Z-t077-s3c-c2** is complete: 430/430
valid, no invalid/heavy samples, likely loss. The [receipt](T077-s3c-c2/README.md#imac-public86-checkpoint)
owns the per-lane ratios and frozen request/result/log. Owner recommends
re-freezing combined S3c+S3d; no C3–C5 runtime work or further timing followed.
Stop for the lead decision. There is no formal/held-out/RSS acceptance claim.
