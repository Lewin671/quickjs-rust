# T029: Compilation-graph static property-name identity

## Status: frozen before implementation

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
