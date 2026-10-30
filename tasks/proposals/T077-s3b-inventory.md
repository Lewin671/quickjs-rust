# S3b counted CallEnv port inventory

Part of the [frozen brief](T077-s3b-brief.md). Static source pin `71294a20`;
recount at accepted S2 and account for every delta before implementation.

## Exhaustive methods

`function/env.rs`: **98 inherent methods** (97 production-capable, one
`direct_eval_cache_len` test-only), plus `Clone::clone` and `Debug::fmt`.
Each identifier occurs in exactly one row below; no inherited Deref API exists.
Clone becomes explicit retained typed-handle cloning, never Context cloning;
Debug prints typed metadata/route identities with short borrows, no execution snapshot.

| Capability (method count) | Every old method | Replacement |
| --- | --- | --- |
| Root/import factories (4) | `new`, `detached`, `from_map`, `with_locals` | Root/host service setup or canonical module cells; delete compatibility factories. |
| Frame factories (6) | `empty_frame`, `new_function_frame`, `new_function_frame_with_capacity`, `new_direct_leaf_function_frame`, `indirect_eval_frame`, `fork_current_frame_values` | Typed enter_function/enter_eval; delete empty/fork factories. |
| Host (3) | `module_host`, `module_host_ref`, `set_module_host` | CreationContext/RuntimeServices host capability. |
| Agent (2) | `agent_context`, `set_agent_context` | Feature-gated RuntimeServices.agent; retain job agent identity. |
| Module imports (7) | `set_module_import`, `module_import_value`, `module_import_cell`, `has_module_import`, `module_imports`, `has_module_imports`, `set_module_imports` | ModuleContext immutable indexed routes to live cells; reads enforce import policy. |
| Module exports (3) | `set_module_live_bindings`, `has_module_live_bindings`, `module_live_binding_cell` | ModuleScope canonical instantiation cells; no settled locals projection. |
| Realm owner (2) | `realm`, `realm_rc` | ctx.realm() borrow; explicit owner clone only when retained. |
| Global lexicals (5) | `mark_global_lexical_binding`, `is_global_lexical_binding`, `set_global_lexical_value`, `mark_immutable_lexical_binding`, `is_immutable_lexical_binding` | Realm declarative descriptors/cells; remove value/set side tables. |
| Catch (3) | `mark_catch_binding`, `unmark_catch_binding`, `is_catch_binding` | Ordered catch ScopeDescriptor/ScopeCursor and declaration checks. |
| Self name (3) | `set_immutable_function_name`, `is_immutable_function_name`, `immutable_function_name` | Function-name lexical descriptor/cell and WritePolicy. |
| Eval conflicts (3) | `clear_direct_eval_var_conflicts`, `mark_direct_eval_var_conflict`, `is_direct_eval_var_conflict` | EvalDeclarationInstantiation against active descriptors/parameter boundary. |
| Enumeration/snapshots (7) | `binding_snapshot`, `for_each_visible_local_binding`, `for_each_visible_local_value`, `visible_local_names`, `has_writable_frame_state`, `visible_local_entries`, `snapshot_locals` | Delete execution uses; cold diagnostics enumerate descriptors/cells, never apply back. |
| Private (2) | `private_environment`, `set_private_environment` | CreationContext.private and class-region lexical overlay. |
| With (2) | `set_direct_eval_with_stack`, `direct_eval_with_stack` | Ordered object ScopeLease; no cloned with-stack for each native. |
| Dynamic overlays (7) | `set_deopt_bindings`, `deopt_bindings`, `fork_deopt_bindings`, `remove_deopt_binding`, `remove_deopt_cell_if`, `remove_frame_binding`, `replace_local_value` | ScopeLease own holder/lexical lifetime; delete forks, flattening and value replacement. |
| Dynamic reads (7) | `get`, `contains_key`, `get_local`, `has_frame_binding`, `has_local_binding`, `frame_binding_cell`, `local_binding_cell` | resolve -> ResolvedReference; init/TDZ/access checks on selected location. |
| Global read routes (5) | `realm_read_layer`, `get_realm`, `realm_binding_cell`, `is_realm_binding_cell`, `realm_contains` | GlobalRoute/canonical realm cell, validated against global descriptors. |
| Marked globals (6) | `dynamic_function_realm_global`, `has_dynamic_function_realm_global`, `has_realm_override_context`, `object_prototype_intrinsic_override`, `array_prototype_intrinsic_override`, `global_this` | Typed GlobalSelection and creation-realm intrinsics; no hidden string markers. |
| Eval cache (3) | `cached_direct_eval_bytecode`, `cache_direct_eval_bytecode`, `direct_eval_cache_len` | Realm immutable compiled-plan cache keyed by parse context; len remains test-only. |
| New target (2) | `set_new_target`, `derived_this_cell` | Typed lexical owner/construct target; Derived cell identity, scoped native view. |
| Binding writes (6) | `insert`, `insert_deopt`, `insert_frame`, `insert_frame_cell`, `set_local`, `remove` | Init/Assign/Declare/Delete by BindingRef; own-holder insertion, never outer overwrite. |
| Global writes (5) | `insert_realm`, `replace_existing_realm`, `replace_existing_realm_with_cell`, `remove_realm`, `realm_entry_or_insert` | Global environment semantic API; preserve sole cell and object descriptor changes. |
| Global object sync (3) | `is_realm_global_object`, `sync_realm_global_object_property`, `forget_deleted_global_object_property` | GlobalRoute + S2 property notifications; no private store bypass. |
| Admission queries (2) | `slot_is_authoritative`, `supplies_no_named_binding` | BindingPlan route facts and current promotion mask; no caller-env equality proof. |

## Exhaustive source consumer families

Reproduction: for every `crates/qjs-runtime/src/**/*.rs`, count regex
`\bCallEnv\b`, group by first relative path component; report nonzero files
and token occurrences. This deliberately includes imports, signatures, comments,
cfg-disabled code and embedded/separate tests: **196 files / 2,112 tokens**.
It measures port surface, not call sites, calls, allocations or unique helpers.
`function/` includes authority `env.rs` (1 file / 7 tokens). No family is omitted.

| Family under runtime src/ | Files / tokens | Capability/port owner family |
| --- | ---: | --- |
| `agent.rs` | 1 / 9 | Agent/native context |
| `array` | 18 / 178 | Array algorithms/callbacks |
| `array_buffer` | 1 / 6 | Buffer construction |
| `array_buffer.rs` | 1 / 16 | Buffer/realm services |
| `async_function.rs` | 1 / 12 | Async reactions/retained context |
| `async_generator.rs` | 1 / 50 | Async generator methods/reactions |
| `atomics` | 1 / 1 | Atomic helpers |
| `atomics.rs` | 1 / 13 | Atomics/agent |
| `bigint.rs` | 1 / 11 | BigInt conversion |
| `boolean.rs` | 1 / 2 | Boolean boxing |
| `builtins.rs` | 1 / 2 | Intrinsic installation |
| `bytecode` | 31 / 202 | Every VM/tier/loop/frame/eval consumer; core owner |
| `conversion.rs` | 1 / 17 | Effectful ToPrimitive/coercion |
| `data_view.rs` | 1 / 8 | DataView realm/errors |
| `date` | 4 / 25 | Date conversion/intrinsics |
| `disposable_stack.rs` | 1 / 14 | Disposal callbacks/retained context |
| `error.rs` | 1 / 13 | Error realm/coercion |
| `finalization_registry.rs` | 1 / 6 | Finalization callbacks |
| `function` | 10 / 93 | Env authority, calls, arguments, captures, creation; core owner |
| `function.rs` | 1 / 1 | Function exports |
| `generator.rs` | 1 / 13 | Generator retained context/resume |
| `global.rs` | 1 / 45 | Direct/indirect eval, dynamic Function, globals |
| `iterator` | 6 / 93 | Iterator helpers/callbacks/reactions |
| `json` | 4 / 33 | JSON parse/revive/stringify |
| `lib.rs` | 1 / 1 | Internal re-export replacement |
| `map.rs` | 1 / 12 | Map callbacks/realm |
| `math.rs` | 1 / 15 | Math coercion |
| `module` | 4 / 10 | Module link/evaluation/jobs/live cells |
| `native` | 26 / 569 | Bound entry families, detailed below |
| `native.rs` | 1 / 6 | Native ABI/dispatch |
| `number` | 3 / 15 | Number conversion/format |
| `object` | 10 / 91 | Object algorithms/descriptors |
| `operations.rs` | 1 / 12 | Shared semantic operations |
| `promise` | 9 / 56 | Promise jobs/reactions/combinators |
| `promise.rs` | 1 / 31 | Promise core/retained context |
| `property` | 3 / 38 | S2 property hooks/keys |
| `proxy.rs` | 1 / 22 | Proxy traps/callbacks |
| `reflect` | 8 / 28 | Reflect property/call/construct |
| `regexp` | 5 / 38 | RegExp algorithms/retained errors |
| `regexp.rs` | 1 / 25 | RegExp core/realm |
| `set` | 1 / 12 | Set algorithms |
| `set.rs` | 1 / 16 | Set core/callbacks |
| `string` | 14 / 94 | String algorithms/RegExp interop |
| `symbol.rs` | 1 / 30 | Symbol realm/keys |
| `test262_host.rs` | 1 / 6 | Harness root/agent contexts |
| `tests` | 1 / 2 | Explicit test consumer |
| `typed_array` | 5 / 84 | TypedArray algorithms/callbacks |
| `typed_array.rs` | 1 / 13 | TypedArray core/realm |
| `value` | 1 / 5 | Value semantic helpers |
| `weak_map.rs` | 1 / 9 | WeakMap realm |
| `weak_ref.rs` | 1 / 3 | WeakRef realm |
| `weak_set.rs` | 1 / 6 | WeakSet realm |

Native installation match: **592 arms**, including feature-gated agent entries
and the TDZ marker. Mechanical signature ports (arm counts): arrays 50;
array_buffers 20; typed_arrays 56; data_views 26; disposable_stacks 14;
atomics 13; agent 8; date 32; math 37; errors 11; json 4; maps 15; sets 17;
weak_maps 7; finalization_registries 3; weak_refs 2; weak_sets 4; numbers 13;
promises 23; objects 36; reflect 13; regexp 22; strings 54; core 62;
crate::iterator 25; crate::generator 3; crate::async_function 2;
crate::async_generator 19; uninitialized_lexical 1. Extract each
`NativeFunction::<variant> => <target>` from `native/entry.rs`; keep every arm.

For individual production creation/retention sites (Vm roots/import/jobs/module,
generator/async, call/construct/field, tier/loop/property/coercion/global and
module host), apply the [entry-site inventory](T077-s3-activation-inventory.md)
to the Context entry table. Its old revision is not a substitute for this census.

## Work removed, with the correct denominator

[Count receipt](../diagnostics/T077-s3-counts.md), source `dd0905b2`, ten programs:
new 243 + from_map 2,788 + clone 72,543 + fresh frame 20,665,887 + minimal leaf
4,145,298 + fork values 100,578 + detached 0 + with_locals 0 = **24,987,337**.
Delete those CallEnv boundaries, per-entry metadata name insertion/search,
empty-frame capacity setup, fork/snapshot/apply-back and caller equality scans.
Replacement activation fields/logical IDs still initialize; exposure/arguments
cells and real host roots still cost work. Zero old constructions does not mean
24,987,337 avoided mallocs or a speedup. Recount clean S2 and S3b inputs twice;
retained cell/lease identity, output and complete counter maps must be stable.
S3a has already removed native argument transport/family probes: preserve its
[reviewed receipts](../diagnostics/T077-s3a-results.md), do not claim them again.
