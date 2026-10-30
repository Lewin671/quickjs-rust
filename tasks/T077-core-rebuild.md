# T077: In-place core rebuild

- Status: active; S2 implementation in progress on `agent/t077-s2-named-stores/codex`, base `b2daec46`; explicitly authorized by the lead's S2 brief while S1 formal measurement/review runs.
- Implementation: function/array named tables and function order index replaced by NamedStore; installers/array constructors ported. Shape IC cutover not yet present.
- Verified at: S2 family cutover working tree, 2026-10-30 UTC: staged fast gate passes (2,414 runtime tests, 117 selected Test262 cases, clippy/context/size); complete stage gates remain pending.
- Evidence: [S2 boundary](proposals/T077-s2-named-stores.md), [S1 receipt](archive/T077-s1-verification.md).
- Unresolved: S2 implementation, correctness/counters/triage; lead-owned S1 and whole-portfolio formal acceptance.
- Next action: replace function/array named authorities and port consumers, then implement weak shape ICs and structural chain dependencies; never push or merge.

## Authority and scope

The lead supersedes the 2026-10-23 no-rewrite decision on 2026-10-29 after the
user authorizes large refactoring: correct, fast, advanced/reasonable/clear.
Route: in-place staged replacement inside qjs-runtime on main; no second engine,
tracing GC, FFI, Rust async, benchmark-specific recognizers or old-path fallback.
Keep slot/shared-cell identity. Delete replaced code within its replacement stage.

The approved target originated on `agent/core-rebuild-design/codex` from
`6663738f`. S1 implementation uses the isolated owner branch/base in the resume
block; its single owner has all paths assigned by the self-contained brief.
The lead owns integration and formal timing; this branch never pushes or merges.
Global/shared API changes have one owner.

## Stage acceptance (overrides ordinary unit gates for T077 only)

Every S1–S7 landing requires focused semantic tests, full `check.sh`, all NG fixtures and a complete configured `--exact --all` Test262 comparison with zero
gaps/no lost passes, no new exclusions; append the complete scan's burndown.
Curated slices alone cannot authorize a landing. Use debug stress modes below,
default/agents/perf-counters configurations and both production targets where
representation changes. No correctness waiver or silent fallback.

Freeze each stage's exact source base, deletions, APIs, case inventories and aggregate comparison rule **before code** in its implementation brief.
This is not a schema-3 leaf plan: T022's 5% payoff, finite attempt budgets and
instruction pre-gates do not govern rebuild stages. Existing collectors are
reused; their strict local rejection label is retained alongside lead decisions.
They do not yet encode this complete contract; never label it automated retained.

Measure development geomean, external aggregate, equal-case selection portfolio and held-out geomean against the preceding accepted milestone and fixed rebuild
baseline. A payoff stage improves **all four** (<1 with the existing formal
precision/health protocol); report cumulative NG ratios too. Local lane/case/
microbenchmark and RSS limits (per-case 1.10, geomean 1.05) are reports, not vetoes
when all overall aggregates improve. Missing/unhealthy evidence is not a waiver.
Held-out remains acceptance-only: no reading/profiling or scope selection; freeze
the candidate before collection, and report a failure to the lead without tuning
against individual held-out inputs.

An enabling stage may be neutral (no unresolved overall aggregate loss) only with its named payer. Development must improve at least every second
stage: S2 vs baseline, S4 vs S2, S6 vs S4, S7 vs S5. For these pairs compare the
unchanged accepted pair endpoints, not multiplied ratios from unrelated runs.
If a payer misses, stop the dependent sequence, diagnose on development and
repair/re-scope the stage for lead review; do not close on promised later gains.
Do not change the predeclared contract to fit results. A stage fallback is a
source revert to the last verified main milestone, never retained dual execution.

Job `20261030T045926Z-core-rebuild-baseline`: **pending baseline**, main vs main vs NG, 30 blocks; no new standing.
Bind its exact source/binary/corpus/recipe hashes when the lead records it.
T018's 2×-faster-than-NG per-case target stays open; architectural completion
plus aggregate improvement is progress, not proof of that campaign target.

## S1 — Atom/shape authority for ordinary objects

- Scope: `property/{atom,key}.rs`, `value/object*`, new `value/layout/`, linked code and ordinary-object constructor sites.
- Delete: all four PropertyStorage forms, Dynamic/literal text indexes and ordinary cold symbol table; preserve literal input-slot metadata.
- Preconditions: owning Value unchanged; explicit domain API agreed before family
  ports. [Implementation brief](proposals/T077-s1-property-authority.md) is ready.
- Correctness: descriptor/order/prototype/exotic NG comparisons, debug layout/slot consistency and key/transition churn.
- Payoff: one keyless authority; T056 M1.2 failed on overlays/construction costs. S1 may be neutral; **S2 pays**.
- Exit: all ordinary creation paths share domain/edges; one ID probe; append-hit
  field-copy/intern counts zero; old stores absent; common stage gates pass.
- Size: **5–8 engineer-weeks**, high construction/API breadth.
- Parallel: owner A `property/{atom,key}.rs` + `value/layout/`; owner B
  `value/object/` storage/tests after API freeze; lead owns `value/object.rs`,
  linked code and family constructor ports. No two owners edit shared headers.
- Risk/fallback: layout growth, atom retention, compiler layout drift; bound caches,
  measure full portfolio; revert this stage if authority/neutrality cannot hold.

Foundation inventory, sizes and verification: [milestone evidence](archive/T077-s1-foundation.md).

### S1 design corrections

The [lead decision](proposals/T077-s1-fallibility.md) replaces monotonic keys
with lease-safe recycling and allocation-error abort. [Object invariants](../docs/design/core-rebuild-objects.md)
record paired identity/epoch guards, cold legacy value revisions and incremental
data-write checks. [Cutover evidence](archive/T077-s1-cutover.md) records reasons;
no error ABI, fallback store or correctness waiver was introduced.
Array.prototype needs array length/index [[DefineOwnProperty]] before NamedStore: the previous raw length/index setter and cache writes could
bypass validation/shrink semantics. The correction routes interpreter, compact,
Reflect and descriptor writes through one hook; creation/slot-write caches decline
this receiver. No second index store or array snapshot.

## S2 — All named stores and shape ICs

- Scope: `function/value*`, `value/array*`, `property/`, property builtin call
  sites; `bytecode/{named_property_cache,enumerate_keys_cache,vm_props,vm_set}`
  and compact/typed property consumers. Use [object contract](../docs/design/core-rebuild-objects.md).
- Delete: function/array text/symbol named tables, payload indexes and old
  exact-value/receiver cache forms; wrappers/exotics use the same NamedStore.
- Preconditions: S1 accepted; layout/key API fixed. Exotics keep index/length hooks.
- Correctness: IC-off complete run, debug cached-vs-generic data probes; inherited
  invalidation, setters/Proxy/super, lazy properties, namespaces, detach/resize.
- Payoff: shape-ID/live-slot hits remove repeated storage/descriptor routing over
  all receiver families. T065 non-Object frequency and T068 14.19% envelope
  motivate reach, not a forecast; text-elision oracle alone was nearly flat.
- Exit: no named-store bypass, stale-value cache or strong receiver pin; bounded
  mono/four-way ICs/prototype dependencies; all four aggregates improve.
- Size: **4–7 weeks**. Parallel: array owner `value/array*` + `property/array.rs`;
  function owner `function/value*` + `property/function.rs`; lead cache/property
  integration and shared APIs. Each consumes S1 NamedStore, returns semantic hooks.
- Risk/fallback: lazy/exotic order and invalidation omissions; source-revert S2,
  then repair with uncached shape lookup in the same core, never old storage.

## S3 — Universal bytecode, frames, calls and native ABI

- Scope: bytecode compiler/IR/VM, function call/context/install, native and builtin
  capability consumers, modules, generators/async. [Execution contract](../docs/design/core-rebuild-execution.md).
- Delete: stack interpreter, nested general JS VMs, CallEnv, per-call argument
  vectors, numeric/this leaves, compact/numeric-plan/wide/caller-env executors,
  native family probe dispatch and their cached admissions/pools/handback.
- Preconditions: S2 accepted; settle T074; freeze Context/BindingPlan/NativeEntry and exhaustive Op-to-Code map.
- Correctness: verifier malformed/CFG tests; deep recursion and JS/native depth;
  zero-based disassembly audit; T016 cell/TDZ/mapped-arguments/global/module tests;
  transitive eval/with leases; callback/private/referrer/derived-this tests;
  exception/finally/disposal and all suspension/job discriminators.
- Payoff: eliminate entry/argument/env work and duplicate dispatch ownership,
  not compression alone (T065 0.998); T068 calls+env deletion envelope 15.63%.
  S3 may be neutral during scalar-tier loss; **S4 pays**.
- Exit: universal register Code, no legacy opcode/fallback; ordinary calls no
  Vec/CallEnv/Rust recursion; virtual streams gone, existing typed/mutation
  executors consume common frames directly, with no retained scaffolding.
- Size: **12–20 weeks**, largest integration risk. Lead amendment 2026-10-29: the Op/capability inventory runs in parallel with S1 and must split S3 into independently landable sub-stages (each with the common gates); no multi-month unlandable branch.
- Parallel: lead owns `ir*`, compiler shared builder, `vm*`, `function/call*` and
  Context; one owner ports `native/arrays.rs` + array builtin family, another
  `native/strings.rs` + string/regexp families after NativeEntry freeze; serialize
  other families/shared headers. Native ports use owned values/argument slices.
- Risk/fallback: borrow/re-entry, cell authority, suspensions and suite losses;
  source-revert S3 if gates fail. No partially supported engine can land as S3.

## S4 — One scalar/escape optimizer; finish executor deletion

- Scope: Code optimizer, existing `typed_loop/`, `vm_numeric_mutation_loop/`,
  `virtual_object/`, frame scalar/materialization maps; [retirement matrix](../docs/design/core-rebuild-retirement.md).
- Delete: region executors, backedge admission/decline masks, private override
  plans and every remaining tier driver; consolidate virtual-object analysis.
- Preconditions: S3 universal semantics and frame/state-map contract verified.
- Correctness: optimizer-off full run; guard failures after effects, scalar/boxed
  alias/CFG joins, holes/prototypes/detachment, callback mutation, capture and
  exact materialization before throw/suspend. Compare optimized/unoptimized/NG.
- Payoff: scalar instruction chains reduce transport and preserve useful suite
  work with one loop; T075 transport envelope 7.9273%, not predicted savings.
  **Pays for S3**, all four aggregates improve and development beats S2.
- Exit: one PC/dispatcher/activation ABI, source/counter inventory finds no old
  executor or ordinary-call admission ladder; full common gates pass.
- Size: **6–10 weeks**. Parallel: analysis owner `bytecode/optimizer/scalar/`,
  escape owner `bytecode/optimizer/escape/`; lead Code op definitions, dispatcher,
  materialization and deletion. New directories are proposed path boundaries.
- Risk/fallback: lost kernel throughput/recovery errors; use boxed operations
  where proof is absent, repair general optimization or revert S4/S3 pair.

## S5 — Opaque owning value and custom counted header

- Scope: `value/access.rs`, `value/repr/` and all Value/typed-handle consumers;
  [memory contract](../docs/design/core-rebuild-memory.md). No new collector yet.
- Delete: public enum-pattern dependency, std Rc JS payload wrappers and their
  variant drop glue; keep non-Copy ownership, weak identity and iterative release.
- Preconditions: S4; complete facade/owner inventory, public API migration and packed-target/provenance qualification.
- Correctness: enum during mechanical port, then one representation per build;
  Miri/sanitizers/count/weak/COW/NaN/bit-pattern/deep-drop tests, both targets and
  host escapes; no raw word is a safe persistent handle.
- Payoff: smaller values/count-tag paths; leaking 0.913 oracle is not its price; NG uses 16-byte values. All aggregates must improve.
- Exit: size/ownership proofs, no new unsafe outside audited repr, same common
  gates, measured allocation/transport reduction and real wall gain.
- Size: **8–12 weeks**. Lead owns representation/handle APIs; parallel family ports use existing builtin directories after facade freeze; no shared-type edits.
- Risk/fallback: pointer portability, provenance, weak teardown and broad API
  churn; revert representation source, not runtime fallback to a legacy engine.

## S6 — Backup cycle collection and real weak semantics

- Scope: heap edge visitors/registry, weak collections/refs/finalization, cells/contexts/code/jobs/suspensions; delete strong weak-target stand-ins.
- Preconditions: S5 common header and complete edge inventory; no engine-owned strong cycle mistaken for host ownership.
- Correctness: trial-count oracle, boundary stress, cycle/ephemeron/kept-alive/finalizer/host-escape suites and poisoned weak reuse.
- Payoff: cycles stop leaking; throughput improvement unpriced. May be neutral;
  **S7 pays**. Report live/retained bytes and pause p50/p99/max, not only RSS.
- Exit: all cycle families return to live-node baseline; no precise roots or JS
  during collection; common gates and neutrality, development S6 beats S4 via S5.
- Size: **4–7 weeks**. Lead owns collector/header contract; separate owner
  `weak_map*`/`weak_set*`; another `weak_ref*`/`finalization_registry*`, using typed
  strong/weak/ephemeron visitor operations. Edges in shared payloads serialized.
- Risk/fallback: missed edges, premature weak death, retention/pauses; leave last
  verified RC stage in main while repairing, never ship a disabled partial collector.

## S7 — Primary Latin-1/UTF-16 strings

- Scope: JsString, String/RegExp, atom backing and host text boundaries;
  [string contract](../docs/design/core-rebuild-strings.md). Delete runtime sentinel
  UTF-8 authority, mode-specific matcher views and byte-prefix coordinate scans.
- Preconditions: S6 accepted; S5 ownership/COW API fixed; T057 semantics preserved. No repeat of the abandoned lazy-view experiment.
- Correctness: both encodings, every surrogate/sentinel form, interior-pair RegExp,
  key/hash/order/COW/host round trips; work-count and full common gates.
- Payoff: direct code-unit operations, shared prepared subjects and less duplicate
  storage; T068 RegExp visible 4.24% but inlined/string/startup reach is not bounded.
  **Pays for S6**: all aggregates improve, development beats S5; report NG gap.
- Exit: single string authority, explicit host conversions, no stale Deref/Borrow
  dependency; final cumulative portfolio, correctness and memory evidence complete.
- Size: **5–8 weeks**. Lead owns JsString/atom/hash API; parallel String and RegExp
  owners in `string/` and `regexp/`, using borrowed Units and identical coordinates.
- Risk/fallback: coordinates/host source break; source-revert S7, retain correctness repairs, re-scope without runtime dual storage.

## Throughput and verification

Total **44–72 engineer-weeks**, roughly **55–90 with 25% contingency**, one
experienced owner; ranges are planning estimates, not measured throughput. Extra
owners shorten family ports after APIs freeze, not shared architecture. Re-estimate after S2 and S3 inventory; sizes imply no T022 attempt budget.

Lead serializes iMac timing (~9 min triage/~25 min 30-block comparison); retain
K=6, complete formal precision/health evidence, held-out 30 pairs and RSS >=15
alternating pairs using qualified host manifests. While an exact candidate is
measuring, implementers run correctness and design the next stage on isolated
branches; no source/result drift, host timing overlap or cancellation of others.
Every stage receipt records hashes, both relative baselines, deletions, counters,
all local regressions, aggregate results and the lead decision in the repository.
