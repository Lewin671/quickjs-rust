# T029: Compilation-graph static property-name identity

## Status: local fast gate passed; exact promotion pending

This T018 leaf unit is bound to the completed exact Performance Preview for
`13e5d229`. The current `91c23d10` revision adds only the T028 rejection
record, so its runtime source and preserved standard executable are identical.
The frozen plan SHA-256 is
`850be7112ee9e25a665ffe3c9458dab5eba46cd97a1ddfaf70fcf0deef300f53`.

## Goal

Give equal static property names one immutable `Rc<str>` identity across a
root compilation and all nested functions compiled from it. Named operations
against compact Small property storage may then test pointer identity before
textual equality; separately compiled and dynamic names retain the current
content fallback.

## Why this unit is current

Performance Preview run `30861546566` produced the completed exact queue for
`13e5d229`. Its SHA-256 is
`9b54feb69fd148e7291ab73aacc7475ee1c46c1698e8942b4f94165debc659e6`.
HashMap remains external rank 1 at 6.9774x QuickJS-NG, public-field Raytrace
rank 3 at 6.0456x, and CDJS rank 6 at 5.1536x, but their latest profiles split
the remaining cost among generic dispatch, direct-call/frame setup, `Value`
lifetime, and named properties. T026-T028 each proved a high-coverage
dispatch/frame mechanism and still returned only roughly 1.6%; retrying that
family is closed.

The first lower-ranked case with a distinct, stable representation boundary is
rank-15 `access-nbody` at 3.6063x QuickJS-NG. A fresh current-runtime sample
contains 3,660 main-thread samples. `_platform_memcmp` contributes 276 flat
samples and its executable stub another 34: **8.47%** directly comparing
property-name bytes. Named Small-storage resolution, shared-slot reads and
writes, typed-loop writeback, and cache work add independent surrounding cost.
The profile SHA-256 is
`595541f6b8d350d18f10e0c71c446226a6b5a50d9274172a09d7b98bf5cd2729`;
stdout and empty-stderr SHA-256 values are
`50fbe849aa61688a0dde78393afa32aba45d9f4a52109662bea06fa4c45715d5`
and
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

T024 already falsified two weaker forms: pointer comparison alone was 0.9993x
because every bytecode site owned a different allocation, while inline byte
comparison improved N-body about 5% but charged the corpus 0.5%. This unit
changes the missing premise -- equal names in a compilation graph share the
same allocation -- rather than retuning either rejected comparison.

## Mechanism and proof obligations

One compilation-local property-name table is shared by the root compiler, all
nested-function compilers, and the second compilation pass used after lexical
capture discovery. It interns only static ordinary string property names used
by named get/set operations and static object shapes. It is neither global nor
realm mutable state and does not change AST, parser, public `Value`, Symbol,
private-name, or computed-key representations.

Named Small-storage lookup compares keys by `Rc::ptr_eq` first and falls back
to byte equality. The fallback is mandatory for separately compiled scripts,
modules, direct eval, `Function`, cross-realm objects, dynamic computed strings,
and host-created objects.

The mechanism gate requires all of:

1. equal named get/set and static-literal keys in sibling and nested functions
   of one compilation graph are pointer-equal;
2. a lexical-capture recompilation keeps that same identity domain;
3. separately compiled equal names remain pointer-distinct yet read and write
   the same property correctly through textual fallback;
4. focused semantics cover accessors, prototype shadowing, frozen/read-only
   receivers, strict errors, and dynamic computed strings;
5. source bytecode, opcode count/discriminants, `size_of::<Op>()`, parser, AST,
   and public runtime representations remain unchanged; and
6. a diagnostic candidate routes at least 90% of N-body's eligible named
   Small-storage comparisons through pointer identity and reduces flat
   `memcmp` plus its stub by at least 70% from the frozen 310 samples.

A timing win without these checks is not attributable and cannot be retained.

## Frozen timing gate

`access-nbody` is the sole payoff target and must be at most 0.95
candidate/base. HashMap, public-field Raytrace, CDJS, A*, date-format-xparb,
Tagcloud, binary trees, recursive control flow, broad property reads, dynamic
method calls, object allocation, and local reads are controls capped at 1.03.
The attempt budget is one. Complete promotion additionally requires the full
broad and external portfolios plus zero exact Test262 gaps.

## Scope

Allowed implementation paths:

- `crates/qjs-runtime/src/bytecode/compiler*.rs`
- `crates/qjs-runtime/src/bytecode/ir.rs` tests only when needed to prove key
  identity or unchanged layout
- `crates/qjs-runtime/src/value/object.rs`
- `crates/qjs-runtime/src/value/object/slot_reads.rs`
- named-property VM/cache/typed-loop call sites needed to preserve an
  `&Rc<str>` through the hot path
- qjs-runtime diagnostics and focused tests
- `docs/benchmarking.md`
- this task, its frozen plan, and the performance task index

Forbidden within this unit:

- global mutable state or a process-wide/Realm-wide atom table;
- changing public property-key, `Value`, object ownership, GC, parser, or AST
  representations;
- removing textual fallback or assuming separately compiled equal names are
  pointer-equal;
- changing computed-key coercion, Symbol/private/index semantics, cache
  invalidation, descriptors, prototypes, or enumeration; and
- source-path, benchmark-name, function-name, property-spelling,
  iteration-count, checksum, or expected-result conditions.

## Verification sequence

1. Validate the frozen unit against the exact queue before code.
2. Preserve the exact standard release executable as the base.
3. Add focused identity, fallback, and observable-semantics tests; run the
   qjs-runtime suite and Clippy with warnings denied.
4. Build release and `perf-counters` candidates; prove the identity-hit and
   `memcmp` mechanism gates.
5. Run warmup-then-strictly-alternating same-host A/B for `access-nbody`, then
   the frozen controls only if the target passes.
6. Record `retained`, `rejected`, or `inconclusive` without changing cases,
   thresholds, or the one-attempt budget after timing.

## Local result

The one-attempt implementation gives every root compilation a private static
property-name table and shares it with ordinary nested functions, class
thunks, and the lexical-capture recompilation pass. Static named get/set keys
and static object-literal shapes retain the table's immutable `Rc<str>` keys.
Small property storage performs a complete identity scan first and only scans
text when no identity exists, so separately compiled scripts, dynamic keys,
host objects, and cross-realm values keep content-based interoperability.

Focused tests prove root/sibling/nested identity, capture recompilation,
pointer-distinct independent compilations, textual read/write fallback,
accessors, prototypes, frozen strict writes, and computed strings. The full
2,064-test `qjs-runtime --all-features` suite passed, as did runtime Clippy
with warnings denied. `Op`, AST, parser, and public value/property
representations were not changed.

The diagnostic release passed both mechanism thresholds. On the frozen N-body
profile wrapper it reported 50,660,897 identity hits and 449,307 textual
fallbacks: **99.12%** identity routing, above the required 90%. Candidate flat
`_platform_memcmp` plus its executable stub fell from the frozen 310 samples
to approximately 10, a **96.8%** reduction, above the required 70%. The
candidate sample and standard executable SHA-256 values are
`269c060159b6730c7363e42dac704ea754297d52a9f2d703635c1335f5348946`
and
`1a4f982bd3bf5caa738afdac839d829ddca861d26abdb12ee0ca03cfd04b9081`.

Eleven warmup-then-strictly-alternating amplified N-body pairs measured median
candidate/base **0.8613x**, with every pair in 0.8332-0.8903, clearing the
frozen <=0.95 target. All twelve frozen controls passed their <=1.03 ceiling:

| Control | Candidate/base median |
| --- | ---: |
| HashMap | 0.976943 |
| public-field Raytrace | 0.994393 |
| CDJS | 0.982043 |
| A* | 0.9839 |
| date-format-xparb | 0.9993 |
| Tagcloud | 1.0073 |
| binary trees | 0.9982 |
| recursive control flow | 1.0079 |
| broad property read | 1.000974 |
| broad dynamic method call | 0.987592 |
| broad object allocation | 0.997489 |
| broad local read | 1.002182 |

The seven JetStream/broad control record has SHA-256
`5f177ea7bde7a0fa3d139d008512cd159962e9daa9d6411dea6df1e5806390fe`.
A local three-role broad run physically completed all 25 cases, but its dirty,
receiptless candidate correctly made the strict analyzer refuse promotion.
The runtime is therefore locally retained, not yet promotion-complete. An
exact committed candidate still needs clean-receipt complete broad/external
reports and the zero-gap exact Test262 receipt before T022 can record the final
`retained` decision.
