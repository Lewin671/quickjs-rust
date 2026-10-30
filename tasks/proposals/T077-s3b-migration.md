# S3b buildable migration and deletion contract

Part of the [frozen brief](T077-s3b-brief.md). These are reviewable commits on
one implementation branch, **not** separately accepted T077 landings. Accepted
S2 must precede all runtime edits. No universal Code, activation pool rewrite,
explicit JS driver or new resource limit is part of this stage.

## Commit series

Every row builds and passes its new lowest-layer tests plus existing affected
runtime tests, default/agents/perf-counters compile checks and staged fast gate.
Add a previously failing semantic fixture with the commit that fixes it; never
commit failing tests or encode a baseline wrong answer. Cross-cutting ABI edits
are atomic. Final acceptance runs the [complete matrix](T077-s3b-verification.md).

| Commit | Production replacement and deletion | Focused proof before advancing |
| --- | --- | --- |
| B1 Compile binding authority | Build BindingPlan/ScopeDescriptor/EvalExposurePlan in existing compiler/upvalue analysis; consumers use its positional parameter/capture routes instead of repeated name searches. Preserve current executable IR. | Received-slot order, shadow identities, parameter/body/catch/loop descriptors, eval propagation stopping/continuing at the specified boundaries; existing parameters/closure tests. |
| B2 Enforce one cell/state authority | Replace Upvalue binding payload with BindingCell+metadata; migrate realm/module/import/mapped arguments routes and all writes to Init/Assign. Remove copied global lexical values and mutability side sets. Existing CallEnv remains the old service entry temporarily, forwarding to canonical cells only. | Const/TDZ/self/import checks, mapped index severance and actual argc, realm deletion/accessor invalidation, module live exports; T074 3/7/9. |
| B3 Install live scope leases | Wire promotion, ordered scope cursor, own var holders, strict/indirect eval and late crossing routes. Delete flattened deopt overlays, parent forks, scope-bypass caches and snapshot/write-back for migrated bindings. CallEnv may retain typed handles until B5, never a second live-value authority. | T074 1/2/5/6/8/11/12/13 plus lexical conflict/with order/loop lifetime, callbacks, parameter eval and early/late exposure. |
| B4 Replace lexical-call metadata | Replace internal string bindings/markers with CreationContext/ActivationContext lexical owners; arguments stays in B1–3 binding authority. Lower deferred keys as resumable enclosing regions with class-private overlay; snapshot saves typed state. Root setup/pure intrinsic queries use Context directly; old CallEnv wraps typed state until B5, without mirroring it. | T074 4/10, every T071 fixture and derived-this tests; module/global receiver/absence, private/home/new.target, field restrictions and pending finally across key suspension. |
| B5 Switch every consumer atomically | NativeEntry and every runtime helper consume Context; Vm/tier/loop frames borrow it directly. Delete CallEnv, FrameBindings and empty/native env construction, VmCallEnv provenance, remaining apply-back and named-self caller restoration. Replace caller-env tier entry with callee typed entry, same admission ordering. | All source consumer families compile; exhaustive native bind test; S3a borrowed owner/recycler/construct-throw tests and debug/release depth parity; all runtime tests. No Context adapter. |
| B6 Stress, audit and freeze candidate | Add forced-promotion/lease-poison modes and retained counters; remove obsolete env tests/counters/compatibility APIs, update env/architecture/suspension module headers and owning docs to actual behavior. Commit all permanent fixtures and complete-scan burndown at the verified runtime SHA. | Entire verification matrix; deterministic ten-program counts twice, deletion audit, clean full/NG/exact passes; source/binary/recipe pins before lead formal acceptance. |

B1–B4 are scaffolding plus real routed behavior on the existing ABI; no Context
consumer reconstructs CallEnv, even temporarily. B5 is the one signature switch,
so there is no dual native ABI. If a cut cannot compile without duplicate value
authority, combine it with its consumer change rather than retain a mirror.
Each row's tests stay enabled after the next row. Do not cherry-pick T071 code:
recover fixture sources and implement the approved contract against S2.

## Exact deletion audit at B6

Paths are relative to `crates/qjs-runtime/src/`. Search identifiers and inspect
surviving aliases/wrappers, not merely the old file name.

| Delete | Retain / replace |
| --- | --- |
| CallEnv definition/re-exports/Clone/Debug, function/env.rs frame portion, FrameBindings/FrameBindingValue | Move realm, host and cell services to context/binding modules. `env.rs` may disappear after its remaining responsibilities move; no renamed universal env object. |
| binding_snapshot, snapshot_locals, visible_local_entries, fork_current_frame_values, fork_deopt_bindings/fork_cells, DynamicBindingsMut snapshot-on-borrow/apply-on-drop on execution paths | Descriptor/cell iteration only for diagnostics; canonical realm/module cell tables remain. No execution name/value snapshot under any spelling. |
| vm_frame_init::frame_call_env/frame_deopt_bindings overlays/memos; vm_call_env::{realm_env,current_env,call_env,callee_env,apply_call_env}; VmCallEnv/Origin | Typed Context borrow and ScopeLease; no marker provenance/scrub/realm snapshot around calls. |
| vm_bindings::apply_env, refresh/writeback passes, conversion::to_primitive_method_env/sync_to_primitive_method_env, function/call_frame::apply_completion_to_caller | Direct canonical cell/property effects; completion returns semantic result/state only. |
| empty_frame, new_function_frame*, new_direct_leaf_function_frame, detached/from_map/with_locals execution factories; function_env/direct_leaf_function_env/callee_this_realm_env and primitive_prototype_env shells | Typed activation/root/creation builders, explicit no-context host root; no replacement empty-env factory called per native/coercion. |
| compact_fn::shares_caller_environment, environment_is_slot_only env proof, try_run_in_caller_env wrappers/proof cache and their wide/direct wrappers | Each admitted callee selects its own typed services and positional binding routes without a compatibility env. Compact/wide executors still exist until S3e; do not disable all tiers to make deletion easy. |
| Hidden string metadata for this/new.target/home/super/active constructor/private/referrer/eval flags/marked globals; named-self mutation/restoration of caller | Typed lexical owner/creation context; arguments remains a real lexical binding; self name owns a separate immutable cell. |
| DynamicBindings bypass registry/closure_scope_use stale-route shortcuts, flattened with stacks and old deopt shadows | Live chain walking for crossing names; ordered scoped ownership. Existing unrelated static capture fast paths remain. |

S3c still owns GeneratorSnapshot/by-value frame reconstruction; S3d owns nested
Vm, ordinary JS transport and stack policy; S3e owns Op/tier/virtual retirement.
Do not expand this deletion list to those stages or claim their savings here.

## Owner boundaries

Shared binding/context/ABI/IR changes serialize on **one coding branch** through
B5, as AGENTS requires. No independently compiled partial ABI branches. Parallel
assistance is tests/fixture preparation, integrated only with its passing fix;
after B5, disjoint family refinements can use isolated worktrees from one recorded
base and a frozen typed API. This table assigns future ownership, not agents now.

| Owner | Exclusive paths under runtime src/ unless otherwise shown |
| --- | --- |
| Core/integrator | `context/`, `binding/` (new), `function*`, `bytecode/`, `native.rs`, `native/entry.rs`, `lib.rs`, `global.rs`, `conversion.rs`, `operations.rs`, `error.rs`, `property/`, `value/`, `module/`, every global doc/script/config and remaining unassigned runtime file. Owns all production paths through B5. |
| Array tests, then family owner after B5 | `array/`, `array_buffer*`, `typed_array*`, `data_view.rs`, `native/{arrays,array_buffers,typed_arrays,data_views}.rs`; test files `tests/{arrays,parameters}.rs`; fixtures with `s3b-array-` prefix. |
| Text tests, then family owner after B5 | `string/`, `regexp*`, `number/`, `native/{strings,regexp,numbers}.rs`; new test files `tests/s3b_text.rs`; fixtures `s3b-text-`. No shared conversion.rs changes. |
| Scope/context tests | New `tests/direct_eval/s3b_*.rs`, new comparison fixtures `s3b-scope-*`, module fixtures `s3b-context-*`; core owner registers modules/fixtures and owns compiler/shared test files. |

Other builtin families remain with core or receive explicit nonoverlapping paths
after API freeze. `function/value.rs`/FunctionData and S2 property files never
have a second simultaneous owner. Core integrates sequentially, validates scope
and runs full/NG gates per runbook. Performance timing is lead-serialized.

## Risks and pre-code lead decisions

Approved exposure/holder/routes/metadata/context/landing choices are **settled**;
do not request the six decisions again. The implementer adopts uncached crossing
routes, lazy own holders and metadata-bearing cells specified here. Residual gates:

| Lead must record before coding | Why it is still open |
| --- | --- |
| Accepted S2 integration SHA, final property API delta, brief SHA and owner/base assignment | S2 is under repair on another branch; `71294a20` is only the design audit base. No invented future SHA. |
| Formal endpoint pins: preceding accepted S2 milestone and fixed `6663738f` rebuild baseline, exact binaries/corpus/recipe/health protocol | S3b is payoff: all four aggregates improve against predecessor; report cumulative baseline/NG. Later S3c/d and S4/S2 payer endpoints remain unchanged. |
| Accept these private type/cut choices and the 3–5 engineer-week scope including T074 suspension repair, or revise this brief before code | BindingCell migration touches every writer; deferred-key lowering is genuine compiler/VM work, not a signature-only port. No public embedding change is authorized. |

Main risks: missed binding writers/import policy, parameter/eval environment
selection, callbacks with borrowed stores, premature TDZ exposure, key await/yield
continuations, and loss of tier throughput when replacing caller equality proof.
Use the frozen tests/counters and development profiles; measurements determine
payoff. No promised later saving, new fallback, correctness waiver or retry gate
borrowed from T022. A failed stage returns to the last verified source milestone.
