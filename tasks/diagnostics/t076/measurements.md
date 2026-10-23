# T076 unit and bundle measurements

Contract and frozen selection: [T076](../../T076-executor-bundle.md),
[plan](../../performance-units/executor-bundle.json). Plan freeze `c0652810`
precedes every runtime change. [Preparation](preparation.md) owns baseline,
A/A qualification, profile/queue provenance and failed preliminary analysis.
All candidate instruction jobs are non-timing, five alternating pairs, all
86 selection cases, internal fixed N=2^20 / 2N. No held-out case selects work.

## U1: destination-aware immediate publication

Source changes only compact `execute.rs` and wide `activation.rs`. Constants,
Move/Dup and checked copies borrow the destination; homogeneous number/boolean
writes update its payload. Numeric fast results reuse that publication path.
Clearing known non-owning values writes Undefined directly; owning replacements
retain publish-before-release via the original out-of-line store. No last-use
ownership proof, Value representation, numeric-tier driver or binding change.

Two new tests prove floating bit patterns (including signed zero/custom NaN),
primitive transitions, self-copy, alias survival/release, and opaque TDZ marker
copying. All three publication tests and 78 existing wide tests pass. Full gate
on the staged runtime tree over `c0652810` passes clippy/features/workspace,
753 benchmark-tool tests, 35 script tests and 5,170 curated Test262 cases.
Full-log SHA-256 `2dbc73e8a21668b2cf4eef4f3c84f6fa48ab957b634bec265e367ed52fe3be77`;
focused-log SHA-256 `c41c8d48713b9baf2956c60962d3439686c1a416221101d4e843d9bf2638c896`.
The subsequent unit commit contains that identical runtime tree. Reference,
codegen and instruction results will be recorded at its exact revision.

U1 candidate is `e5dcaef042ab0d23dea0c522a1f00b8d34b74adc`; its final staged
fast gate also passes all 65 mapped Test262 cases. Non-timing job
`20261023T215026Z-t076-u1-reference-instructions`: exit 0 / 897 s; all 257
UTC QuickJS-NG fixtures and all 1,170 candidate/base process checks pass.
Reference-log SHA-256 `469fcd8ad0e7c2538a0aa4a4574e6ca848bb6b16e42eb9f5e5c4e449eec6a1c7`.
[Raw-count replay and every ratio](u1-instructions.json) are retained.

**Gate miss; drop U1, without wall tuning.** Target geomean 1.008294424 >
0.995; 14 controls exceed 1.003, worst `external/sunspider-1.0/3d-raytrace`
1.028745575. Targets: graphlib 0.999954423, immutable 1.013377656,
lodash 1.013638013, matter-js 1.006271201. No timing/triage/screen is run for
this unit. Source and tests are preserved on `agent/t076-u1/codex`; its two
runtime files are restored exactly to `c0652810` (the base engine source).
The instruction budget is binding; no repair iteration is attempted.

[Codegen census](u1-codegen.json) uses the next distinct text symbol as the
wide `activation::run` boundary in each nm listing. Span grows 24,928 ->
26,416 bytes; otool listing rows 5,482 -> 5,817 and stack-memory operand rows
1,209 -> 1,248. Rows include embedded data/padding and cold paths; stack
operands are not a count of executed spills. This documents larger generated
code/stack traffic context, not a causal stall price. Actual retired work,
not this static proxy, rejects the unit. Image/assembly hashes are retained.

## U2: retain proven scalar index operands

The final compiler pass tracks Box/Unbox equivalence through boxed copies in
one basic block. Register writes, branches and join targets kill facts. It
rewrites only ComputedRead -> existing ElementRead, keeping the original Box,
other consumers, local writeback, sites, recovery stack and operation indices.
Its exhaustive destination effects come from the existing forward pass. No
new opcode, register, seed rule, tier, environment or T045 join inference.

Four adjacent proof tests cover namespace separation, overwritten inputs,
copy/write consumers, successful guards and multi-predecessor/backedge joins.
Three behavior tests cover actual lowering, zero trips, postincrement operand
recovery, getter/setter and Proxy ordering, inherited holes, and coercion that
mutates a later element. The lowering regression fails with only this pass
disabled, then passes restored (ablation log SHA-256
`f1e0a83df954ff498de4936b5d3d8719e5dfb505b67fbefcb55dce1995cb8479`).

[Supplemental IR census](u2-ir.json): big.js rewrites exactly six of 23
ComputedRead to ElementRead; 29 regions / 557 operations / register widths are
unchanged. Entry/deopt diagnostic counts stay 273,022 / 51. Matter-js retains
39 regions / 1,344 operations, zero ComputedRead and 45 ElementRead, entry/
deopt counts 335,895 / 17. These arm64 feature traces were collected with the
U1+U2 working tree; U1 is subsequently removed. They price no wall/instruction
saving and do not claim exact final-image performance.

On the U2-only tree over drop commit `45cd5c8c`, the full gate passes 2,389
runtime tests, feature/clippy gates, 753 tool tests, 35 script tests and 5,170
curated cases. Full-log SHA-256
`e194365749b6774f48538e3f7815e31fec903c318a661848b15ad887e16de3e5`.
The focused feature test also passes on that same tree; existing counters
prove acceleration, and `QJS_TL_TRACE=4` explicitly reports `TLRUN region
11..62 Ran` with ElementRead. Trace-log SHA-256
`80e63be10691bc71ca8fedb4af7a39fe20891b97a0faf3bd422d8e5bb8919c5b`.
The next unit commit contains that identical runtime tree; exact-reference
and non-timing instruction results remain pending.

U2 candidate `12aa95b829a26a9f96c7e81750e07a6a21477ebe` passes the 65-case
staged semantic slice. Non-timing job
`20261023T221604Z-t076-u2-reference-instructions`: exit 0 / 901 s; 257 UTC NG
fixtures and all 1,170 process checks pass. [Every raw-count ratio](u2-instructions.json)
replays exactly. Target big.js is **0.997338770**, a 0.2661% instruction
reduction, short of the frozen <=0.995 requirement. No control exceeds 1.003;
maximum is hash-map 1.000585500 (math-spectral-norm 1.000257651 next).

**Gate miss; drop U2 without expansion or wall tuning.** Source/tests remain
on `agent/t076-u2/codex`. All five typed-loop runtime/test files are restored
to `45cd5c8c`, which has the original `19125d48` compiler. The six qualifying
reads were real, but their price does not meet the declared unit threshold.
U3 is tested again after this removal so its own full gate describes the
actual surviving candidate, not the discarded U2+U3 composition.

## U3: prelinked received-cell indices

WideProgram owns one immutable boxed byte table derived from the existing
received-slot mask. Entries carry typed source-vector indices; holes use an
unreachable sentinel, not a binding value. Wide reads/writes and typed-loop
seeding use it while retaining the original runtime owner, mask, vector and
TDZ checks. WideActivation's call-context layout is unchanged; no per-call
allocation, name view, cell snapshot, opcode/Value width or new dependency.

Two table tests cover absence, dense 128-cell proofs, sparse slots and the
63/64/127 boundaries. Four binding tests cover admitted body/source-order
agreement, distinct live owners sharing bytecode, write-through, a TDZ error
followed by initialization and typed-loop seeds after sibling writes. A
feature test confirms three callback entries into compact drivers; named
properties/mutable cells exclude the numeric/leaf paths. A first entry-test
pilot checked only `compact_direct_calls` (0); actual `compact_caller_env_calls`
was 3, so the assertion now includes both existing compact entry routes.
Production admission is unchanged.

The first full-gate pilot finds the retired rank method unused in production
(`error: method cell_received_upvalue_index is never used`). It is retained
under cfg(test) as the prelinked table's oracle; no dead-code allowance is
added. U3+U2 passes the full gate after that correction; U2 is then removed
and the whole gate runs again on U3 alone over `6e806ea4`.

Final U3-only gate: 2,388 runtime tests, agents/perf-counters/clippy, 753 tool
tests, 35 script tests and 5,170 curated cases all pass. Full-log SHA-256
`ac4bcee189a175831b89652a150541e7817d92dd6dea8fc1de75a63fc381adb4`.
Focused publication-independent metadata/binding tests pass; the U3-only
feature test log has SHA-256
`127854473356c45056321e9f56eab7536381281b7f654c90f7df7bad12ba8b3d`.
`QJS_TL_TRACE=4` confirms four `TLRUN region 11..57 Ran` events for the
sibling-write/seed case (log SHA-256
`d6119b8afcbc99a7e39baf507ffa9a2edcd2fec1d994f404c13c4005d940b6d6`).
The subsequent unit commit contains this identical runtime tree; exact
reference/instruction evidence is pending.

U3 candidate `08bb19a7181818c293d7c9cef159b9c6443aa227` passes the 65-case
staged semantic slice. Non-timing job
`20261023T224347Z-t076-u3-reference-instructions`: exit 0 / 892 s; all 257
UTC NG fixtures and 1,170 process checks pass. The reference log hash is the
same `469fcd8a...c6a1c7` as U1/U2 (identical fixture paths/results).
[All raw-count ratios](u3-instructions.json) replay exactly on both hosts.

Own-target geomean **0.987714706 passes** <=0.995: immutable 0.984939745,
js-yaml 0.992729673, lodash 0.982803460, papaparse 0.990418485. The sole
control breach is **audio-beat-detection 1.010866433 >1.003**. Therefore the
unit misses the filter and is dropped literally, without another collection
or a wall-based repair. Source/tests remain on `agent/t076-u3/codex`; all
seven runtime/test files are restored to `6e806ea4` (the original engine).

The audio control's complete five-pair counts are retained below. The base
itself spans 2.6711% here, versus the initial three-repeat baseline's 1.1583%
range and the qualification A/A median 1.000233851. This demonstrates finite
instruction variability, not that the candidate caused no regression. The
present evidence does not separate an implementation effect, randomized
runtime work, or other process-dependent work. No favorable retry is made.

| Pair | Base instructions | Candidate instructions | C/B |
| --- | ---: | ---: | ---: |
| 0 | 17293437898 | 17481355890 | 1.010866433 |
| 1 | 17057999067 | 17459187782 | 1.023519096 |
| 2 | 17513634038 | 17418766596 | 0.994583223 |
| 3 | 17341272281 | 17494507286 | 1.008836434 |
| 4 | 17124607571 | 17507431926 | 1.022355219 |

Conditional non-timing job `20261023T224727Z-t076-u3-conditional-conformance`
replays the same gate, exits 0 / 0 s and records `SKIP: no surviving unit; U3
instruction gate missed`. No complete exact scan was started, so no burndown
entry is appropriate. With zero survivors, there is no distinct bundle
candidate for quick triage. Screen/formal/held-out/RSS/resource prerequisites
are unmet, and 0/2 screens are spent. No decide/promotion command is run.
