# T077: In-place core rebuild

- Status: active; S3a and S1+S2 accepted and integrated (lead 2026-10-31); S2 follow-up and S3b in progress.
- Implementation: main has S3a direct native entries/slices and S1+S2's one named-property authority (recycled KeyIds, prototype-sensitive shapes, validity cells, cell-protected weak prototype constants, shape ICs, construction recipes). Owner branches: `agent/t077-s2-followup/codex`, `agent/t077-s3b-context/codex`.
- Verified at: S1+S2 `fc3deef7` (runtime `dc710e4b`): full check, 268 NG fixtures, 42,672/42,672 exact cached and IC-off; formal/held-out/RSS in [S1+S2 acceptance](diagnostics/T077-s1s2-acceptance.md); S3a in [S3a acceptance](diagnostics/T077-s3a-acceptance.md).
- Evidence: [S2 follow-up round 2](diagnostics/T077-s2fu-round2.md), [S2 follow-up](diagnostics/T077-s2-followup.md), [target](../docs/design/core-rebuild.md), [baseline](diagnostics/T077-baseline.md), [S2 freeze](diagnostics/T077-s2-freeze-qualification.md), [S2 round 4](diagnostics/T077-s2-r4-handoff.md), [S3 plan](proposals/T077-s3-plan.md), [S3b brief](proposals/T077-s3b-brief.md).
- Unresolved: accepted local regressions (external +0.25% vs S3a, typed-loop/wide static size) are S3e/S4 obligations; S2 follow-up round 2 must remove hot work, reduce owned-child layout memory and finish iterative cycle walks; the global accessor stale-read bug is in S3b; deep Rust-stack exhaustion is S3d-owned.
- Next action: S2 follow-up owner executes lead round-2 feedback A–D with runner-ref transport authorized; S3b continues on its branch; lead reviews, measures and integrates each. Cumulative vs baseline: development 0.893, external 0.996, selection 0.978, held-out 0.929.

## Authority and scope

The lead supersedes the 2026-10-23 no-rewrite decision on 2026-10-29 after the
user authorizes large refactoring: correct, fast, advanced/reasonable/clear.
Route: in-place staged replacement inside qjs-runtime on main; no second engine,
tracing GC, FFI, Rust async, benchmark-specific recognizers or old-path fallback.
Keep slot/shared-cell identity. Delete replaced code within its replacement stage.

The [target design](../docs/design/core-rebuild.md) owns architecture. The lead owns integration/formal timing; this owner may merge main into its branch only when directed. Global/shared API changes have one owner.

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

Rebuild baseline ([receipt](diagnostics/T077-baseline.md)): job `20261030T045926Z-core-rebuild-baseline`, source `6663738f`, 30 blocks: Rust/NG development **2.632**, external 0.911, selection 0.971, sentinel 1.046; A/A 1.000.
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
record prototype-sensitive identities, dictionary versions and incremental data-write checks. [Cutover evidence](archive/T077-s1-cutover.md) records reasons;
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
- Correctness: IC-off complete run; debug cached-vs-generic data probes; inherited invalidation, setters/Proxy/super, lazy properties, namespaces, detach/resize.
- Payoff: shape-ID/live-slot hits remove repeated storage/descriptor routing over
  all receiver families. T065 non-Object frequency and T068 14.19% envelope
  motivate reach, not a forecast; text-elision oracle alone was nearly flat.
- Exit: no named-store bypass, stale-value cache or strong receiver pin; bounded mono/four-way ICs/prototype dependencies; all four aggregates improve.
- Size: **4–7 weeks**. Parallel: array owner `value/array*` + `property/array.rs`;
  function owner `function/value*` + `property/function.rs`; lead cache/property
  integration and shared APIs. Each consumes S1 NamedStore, returns semantic hooks.
- Risk/fallback: lazy/exotic order and invalidation omissions; source-revert S2,
  then repair with uncached shape lookup in the same core, never old storage.
### S2 design corrections

The [round-1 correction](proposals/T077-s2-prototype-layout.md) replaces shared-layout subscriptions and implicit prototypes: layouts include prototype identity, shared shapes have no epochs, and private prototype states own uncapped weak user graphs with cascading validity generations. Arrays/functions retain explicit realm intrinsics and lazy storage; missing/accessor outcomes cache pure proofs. [Object contract](../docs/design/core-rebuild-objects.md#inline-caches) owns implemented invariants.

## S3 — Universal bytecode, frames, calls and native ABI

- Contract: [execution](../docs/design/core-rebuild-execution.md); [approved five-stage plan](proposals/T077-s3-plan.md); [frozen S3b implementation brief](proposals/T077-s3b-brief.md) with typed Context, complete ports, binding structures, B1–B6 commits and verification matrix. S3a is integrated; S3b remains design only.
- Landings: S3a direct native entries/slices; S3b typed Context and T074
  binding authority; S3c owning activations/suspension; S3d explicit JS stack and
  callee argument writes; S3e universal register Code and executor deletion.
- Every landing takes the common full/NG/complete exact zero-gap gates above;
  no partial-language backend or old-path fallback. Payers/endpoints as in the
  plan (approved); the S4-versus-S2 payer obligation remains.
- Lead decision 2026-10-29: split approved. S3a may land ahead of S1/S2 on its
  disjoint paths (whichever lands second rebases). S3b–e need accepted S2, T074
  approval before S3b and frozen per-stage APIs/deletions/test maps. The S3e brief
  must price growing the existing wide register tier (62.5% of routed entries,
  most executed ops) into universal Code against a fresh lowering.
- Exit remains: universal Code; no CallEnv/nested JS Vm/ordinary argument Vec,
  stack/leaf/compact/numeric-plan/wide/caller-env/virtual executable streams or
  cached admissions. Typed/mutation loops consume common frames/Code exit maps
  until S4 removes them. [Retirement](../docs/design/core-rebuild-retirement.md).
- Evidence: [ten-program deterministic path counts](diagnostics/T077-s3-counts.md)
  price reach, not savings: 78,693,537 native-family declines, 24,987,337 CallEnv
  constructions/clones and 2,137,517 frame/VM constructions. No timing claim.
- Proposed size: 13–21 engineer-weeks plus 1–2 weeks for review and unpriced repairs;
  original 12–20-week estimate needs lead reconciliation. Largest landing is
  S3e (4–6 weeks); all detailed boundaries, tests and risks live in the plan.
- Risk/fallback: cell/context/re-entry/suspension correctness and wide/scalar
  throughput; revert a failed stage/payer pair to the last verified milestone,
  never retain a compatibility interpreter. S4 still pays for neutral S3e.

S3d owns the pre-existing deep Rust-stack crash (including native callbacks): one runtime-wide stack anchor with a host-configured budget,
a CLI engine thread with an explicit stack size, and frame-counted JS depth after JS stops recursing in Rust.
S3a preserves base depth behavior and adds no limit. Its rejected per-realm guard missed JS frames before the anchor and did not establish overflow protection.

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

Stage estimates remain planning ranges, not measured throughput; re-estimate after S2/S3. No T022 attempt budget governs these rebuild stages.

Lead serializes iMac timing (~9 min triage/~25 min 30-block comparison); retain
K=6, complete formal precision/health evidence, held-out 30 pairs and RSS >=15
alternating pairs using qualified host manifests. While an exact candidate is
measuring, implementers run correctness and design the next stage on isolated
branches; no source/result drift, host timing overlap or cancellation of others.
Every stage receipt records hashes, both relative baselines, deletions, counters,
all local regressions, aggregate results and the lead decision in the repository.
