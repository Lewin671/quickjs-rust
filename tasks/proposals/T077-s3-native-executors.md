# T077 S3: native ABI and specialized-executor inventory

Source audit at `dd0905b2`, 2026-10-29; paths are runtime `src/` relative.
[Stage plan](T077-s3-plan.md), [activation inventory](T077-s3-activation-inventory.md),
[deterministic counts](../diagnostics/T077-s3-counts.md).

## Native dispatch, every family

`function/call.rs` dispatches native after Proxy/bound/class-call validation.
Wide's `call_from_activation` also enters `native::call_native_function` directly.
`native.rs` first asks cached `FunctionData::native_family` if nonzero, then
families 1–28 excluding that cached ID. `Ok(None)` declines; `Some` returns and
caches; an error exits immediately. If all decline, `native/core.rs` is called
without caching a core-family ID. Family 7's implementation is agents-gated,
but the numeric family slot is probed even in a non-agents build.

All family dispatchers borrow `&[Value]`; most clone the receiver on each probe
and receive function/native/is_construct/CallEnv. Below: F=function capture and
creation realm, T=receiver, C=construct/new.target, E=realm/error/coercion,
P=property/iterator operations, J=callback/call/construct, Q=job/suspension,
A=agent. Capabilities describe the family as a whole, not every member.

| ID | Dispatcher / implementation | ABI touchpoints to preserve |
| ---: | --- | --- |
| 1 | `native/arrays.rs::call_array_native` → `array/` | F T C E P J Q; species, callbacks, sort, async-from, sparse/prototype semantics. |
| 2 | `native/array_buffers.rs::call_array_buffer_native` → `array_buffer` | F T C E P J; species, detach/resize and coercion order. |
| 3 | `native/typed_arrays.rs::call_typed_array_native` → `typed_array/` | F T C E P J; live buffer bounds after callbacks/species. |
| 4 | `native/data_views.rs::call_data_view_native` → `data_view` | F T C E P; offsets/endian/value coercion and detached checks. |
| 5 | `native/disposable_stacks.rs::call_disposable_stack_native` | F T C E P J Q; dispose/move/adopt/defer, async disposal and suppression. |
| 6 | `native/atomics.rs::call_atomics_native` | E A; typed backing, wait policy/coercion; no callee/receiver argument. |
| 7 | `native/agent.rs::call_agent_native` | E A; feature gate, can-block, broadcast/report; no callee/receiver. |
| 8 | `native/date.rs::call_date_native` → `date` | F T C E P J; primitive coercion and construction realm. |
| 9 | `native/math.rs::call_math_native` → `math` | E; coercion may call JS, randomness belongs to host/runtime; no receiver. |
| 10 | `native/errors.rs::call_error_native` → `error` | F T C E P J; cause/message, Aggregate/SuppressedError and realm. |
| 11 | `native/json.rs::call_json_native` → `json/` | E P J; replacer/reviver/toJSON and key order; no receiver. |
| 12 | `native/maps.rs::call_map_native` | F T C E P J; iterable construction, mutation during callbacks. |
| 13 | `native/sets.rs::call_set_native` | F T C E P J; iterators and set-like methods/traps. |
| 14 | `native/weak_maps.rs::call_weak_map_native` | F T C E P J; current owning stand-ins survive until S6. |
| 15 | `native/finalization_registries.rs::call_finalization_registry_native` | F T C E Q; holdings/token ownership, S6 supplies actual weak collection. |
| 16 | `native/weak_refs.rs::call_weak_ref_native` | F T C E; preserve current lifecycle pending S6. |
| 17 | `native/weak_sets.rs::call_weak_set_native` | F T C E P J; iterable and weak-key validation. |
| 18 | `native/numbers.rs::call_number_native` → `number` | F T C E P J; wrapper validation, radix/coercion and realm. |
| 19 | `native/promises.rs::call_promise_native` → `promise/` | F T C E P J Q; executor/thenable calls, species, FIFO and rejection. |
| 20 | `native/objects.rs::call_object_native` → `object/` | F T C E P J; descriptors, getters/setters, exotic/prototype hooks. |
| 21 | `native/reflect.rs::call_reflect_native` | E P J; call/construct forwarding incl new.target, traps; no receiver. |
| 22 | `native/regexp.rs::call_regexp_native` → `regexp*` | F T C E P J; overridden exec/species, lastIndex and string callbacks. |
| 23 | `native/strings.rs::call_string_native` → `string/` | F T C E P J; symbols, replacement callbacks, iterators and code units. |
| 24 | `iterator::call_iterator_native` | T C E P J Q; iterator/helper state and close; no function capture. |
| 25 | `generator::call_generator_native` | T E Q; next/throw/return, owned frame state; no construct flag. |
| 26 | `async_function::call_async_await_native` | F E Q; reaction captures, promise adoption and resume. |
| 27 | `async_generator::call_async_generator_native` | T E Q; request queues and resume; no function capture. |
| 28 | `async_generator::call_async_generator_reaction` | F E Q; pending request/reaction capture and settlement. |
| core | `native/core.rs::call_core_native` | F T C E P J Q; Boolean/BigInt/Symbol, URI/parse/global/eval/Function, Proxy, print, harness/host intrinsics. Exhaustive NativeFunction binding required, not a default fallback. |

Other dispatch/probe paths are real and must not disappear from the inventory:
`vm_call::try_fast_global_native_call` is used by VM, wide and typed-loop helpers
(URI, parseInt/Float, Math, String.fromCharCode, dense indexOf, trivial eval,
Number formatting and other explicitly matched variants). Its miss falls through
to ordinary native dispatch. `call_resolved_guarded_math_unary`, typed/helper
numeric lowering and number-leaf evaluators also execute primitive native math.
These are guards around semantic primitive operations, not evidence of zero
native work. S3a binds ordinary native entries; S3e consolidates call-site guards
into the common Code/native ABI; S4 can inline proved scalar primitive operations.

Installation must map **every NativeFunction variant**, including internal
reactions and agents, to an entry (or reject UninitializedLexical as a TDZ marker).
Use an exhaustive match without wildcard at creation; family ownership tests
must prove no overlap/missing entry. Bind an operation entry directly, not a
function that repeats the 28 Option probes. Keep NativeFunction identity only
where semantics/guards/harness need it. S3a removes native_family and Option
family dispatch; S3b changes the entry service parameter to typed Context.

Ordinary borrowed native arguments must come from a detached owning caller block
or a host slice. No Executor/RefCell mutable borrow spans callback re-entry.
NativeBoundary pins that block until return/throw; retained arguments are owning
clones. Restoring current context on all exits and checked native-depth limits
are obligations even though pure JS calls move to the explicit stack in S3d.

## Every specialized executor and admission surface

| Executor / cached state | Ordinary-call and backedge admission | Retirement / surviving analysis |
| --- | --- | --- |
| Numeric leaf: `vm_numeric_leaf{,_registers}.rs`; `numeric_leaf_plan` | Outer `is_direct_leaf_function`; lazy plan compile/negative OnceCell; parameter/upvalue shape and primitive tag/stack/register limits. Number-only helper/typed calls also probe/evaluate it. Delayed writes only commit on success. | S3e deletes evaluator, plans and all entry/helper probes; general Number ops/constant folding, S4 scalar data flow. |
| This-property leaf: `vm_this_property_leaf.rs`; `this_property_leaf_plan` | After numeric decline; plain object own-data/property guard and supported numeric/receiver/argument plan. Getter/constructor initializer paths also use helpers; property miss must decline before effects. | S3e deletes plan/admission/evaluator; ordinary GetNamed/IC + Return and cold field-init metadata. |
| Caller-env: `compact_fn/activation::{try_run_in_caller_env,shares_caller_environment}` plus wide | Caller host/agent agreement, no named bindings/imports/deopt/dynamic realm, no special lexical context; compact/wide authoritative-slot/cell admission. Wide caller entry rejects requires_this. Cached fixed facts on inline callees still need live host/realm checks. | S3b deletes equality-of-CallEnv proof, replacing it with exact typed context requirements; ordinary compiler/tier eligibility remains until S3e. No copied service env. |
| Compact numeric register tier: `compact_fn/{compile,execute,activation}.rs` | Whole-body supported Op set, no global/eval/with/handler/suspension/backedges, balanced stack CFG, initialized/authoritative slots, register limit and arity <=8; runtime cell owner/readonly routes. Inline facts cache positive and negative shape proofs; inline entry still checks live environment. | S3e deletes driver, TLS storage, compact_inline_facts, pools/program caches/admission. Reuse depth-to-register lowering, not a permanent subset. |
| Numeric compact plans: `compact_fn/numeric_plan/` | Compile-time Number/boolean/undefined/callee data flow and call graph; runtime numeric args/arity/cell helper targets, depth limit and side-effect-free replay. Standalone compact and wide inline paths ask separately; nested f64 calls use their own stack. | S3e deletes plan interpreter/cache/probes/stack; owning generic arithmetic initially, S4 absorbs scalar/helper analysis into common Code. |
| Wide: `compact_fn/wide/`; `wide_inline_facts` | Wider whitelist/CFG/TDZ/lexical slots and receiver checks, program activation admissibility, authoritative cell routes, class/new eligibility and live context. Inline calls may use numeric plan; exits reconstruct general frame and may hand loops back. | S3e deletes driver/windows, exit Op-depth translation, handback, facts/admission/cache/pools. Properties/calls go through common Context and driver. |
| Numeric mutation: `vm_numeric_mutation_loop/` | First at generic backedge; compile cached plans for dense/typed arrays, nested dense/hole append/predicate scan. Runtime guards, frame override/suppression and exact effect publication; current ten-program run has no successful entry. | Port once to common-frame/state-map API in S3e; S4 deletes executors, plan overrides and backedge probe. Keep generic data/type/alias analysis, not pattern-shaped code recognition. |
| Typed loops: `typed_loop/`, `vm_loop_dispatch.rs` | Second generic backedge probe, and wide loop path; supported region CFG/op/cell/array/helper graph, no direct-eval-with, decline bit, enclosing-special-region exclusions, state/argument/type guards; recovery PC and done effects. Empty lists and cached declines still incur entry probes. | S3b Context port; S3e common Code exit-PC maps without retained executable Op; S4 deletes driver/scalar banks/admission/decline masks. Scalar instructions use same PC loop. |
| Virtual objects/functions: `virtual_object/`, `vm_virtual_object.rs` | Compile alias/escape/CFG proof; per-activation `FrameProgramView::new` selects full/data-only/original using authoritative mask and function-context safety. No call ladder; separate executable streams/bank, materialization barred at suspension. | S3e deletes selected streams, bank, allocation_ip dependency and virtual handlers. S4 escape analysis emits one Code/materialization map. |

`vm_string_append.rs` is an owning COW semantic operation, not a second executor;
retain it through S3, and preserve uniqueness under S5/S7. Named/enumeration ICs
belong to S2. At every deletion stage search Bytecode/FunctionData plan fields,
negative caches, TLS scratch, decline masks, call sites and counters: deleting a
public entry but leaving an ordinary-call refusal ladder does not close it.
