# T077 S3: activation and environment construction inventory

Audited at `dd0905b2`, 2026-10-29. Paths below are runtime `src/` relative.
Target stages are defined in the [plan](T077-s3-plan.md); counts and the retained
instrumentation patch live in [diagnostics](../diagnostics/T077-s3-counts.md).
No factory in this table is an approved new API yet.

## CallEnv creation authorities

All CallEnv struct literals are in `function/env.rs`. These eight constructors
are the exhaustive production allocation/clone boundary; forwarding constructors
are listed separately so they cannot be mistaken for additional objects.

| Factory / current responsibility | What it really needs | Replacement / stage |
| --- | --- | --- |
| `new(realm)` | Shared realm/intrinsics/global-lexical metadata, optional host | Context realm services, no local map; S3b. |
| `detached()` | Empty-realm conversion/JSON services without live activation | Explicit detached Context capability; S3b, retain owning results. |
| `from_map(map)` | Legacy native/dynamic creation context | Typed retained realm/native captures; S3b. Map may remain a cold host import input, not execution authority. |
| `with_locals(realm,locals)` | Settled async module export view | Return canonical live module cells and typed ModuleEvaluation; S3b. |
| `Clone::clone` | Copy metadata and isolated frame-binding view; shares some explicit cells | Clone owning context/leases by identity, never clone an execution name/value view; S3b. |
| `new_function_frame_with_capacity` | Fresh function binding identity, inherited realm, private/host/import context | BindingPlan + typed activation context + optional own eval holder; S3b. |
| `new_direct_leaf_function_frame` | Realm/host/import services for admitted slot-only body | Typed context with no binding container; S3b. Delete leaf-specific factory at S3e. |
| `fork_current_frame_values` | Materialize current dynamic view from visible slots and cells | ScopeLease and canonical cell routes; eliminate fork/write-back in S3b. |
| Forwarders: `empty_frame`, `new_function_frame`, `indirect_eval_frame` | First two delegate to capacity factory; indirect eval enables global lexical visibility | Service borrow or selected global environment; not three extra allocation sites. |

## Every production creation/entry site, grouped by semantic caller

Table names are functions or modules, rather than line numbers changed by the
throwaway probes. Explicit factory calls were enumerated with `rg`; test-only
constructors in embedded test modules are excluded. Clone sites funnel through
the constructor above; retained contexts below include all such owner families.

| Site(s) | Inputs/retention actually required | Disposition |
| --- | --- | --- |
| `bytecode/vm_frame_init.rs::Vm::new` | Script realm, builtins, global declaration instantiation | One root Executor + Context; S3b/d. |
| `vm_frame_init::{new_with_globals,new_with_globals_and_with_stack,new_with_globals_upvalues_and_with_stack,new_with_globals_upvalues_with_stack_and_direct_call_slots}` | Forward realm/upvalues/with/slot inputs | All delegate to `with_frame_bytecode`; no independent VM objects. S3d removes wrapper ladder. |
| `vm_frame_init::with_frame_bytecode` | **Only FrameState struct literal**, one Vm/locals/upvalue routes/operand stack per entry | Fieldwise reusable activation builder, separate storage; S3d. |
| `vm.rs::{eval_bytecode,eval_call_bytecode,eval_direct_call_bytecode,eval_function_bytecode}` | Root, ordinary result-only, tier-or-minimal frame, or eval/derived result with binding state | One driver; only root enters Executor; S3d/e deletes nested VM/tier entry setup. |
| `vm_import::{eval_bytecode_with_module_resolver,eval_bytecode_with_module_resolver_in_agent,eval_bytecode_in_agent_context}` | Root script, referrer/resolver, optional agent/can-block | Typed root Context; no JS-call-specific Vm. S3b/d. |
| `vm_jobs::eval_bytecode_keep_jobs` | Root script arguments and jobs | Host entry boundary, not an ordinary callee; S3b/d. |
| `vm_module::{eval_prelude_script,new_module_realm,eval_module_body,eval_module_function_hoists}` | Shared graph realm, declarations/import/export cells, host, referrer, this=undefined | Root/module activation + canonical ModuleScope; S3b/d. |
| `vm_module::eval_async_module_body` | Same module cells plus owned suspension and async result promise | S3b removes settled `with_locals` projection; S3c reuses owning activation on resume. |
| `vm_generator::{start_suspended_at_body,run_from_start,run_from_yield,resume_generator}` | Generator prologue, owned locals/stack/cells/try/disposal/completion/resume mode | S3c replaces reconstruct-on-resume; transfer same activation owner, no new Vm. |
| `vm/wide_resume::resume_direct_call_bytecode` | Wide exit's exact Op PC, local values/cell routes/operand depth and receiver | S3d entry into existing driver, S3e deletes handback and translations. |
| `frame_stack::{push_frame,run_completion}` | General callee stack and completion restoration; **production push dormant** | Replace its by-value frame moves/test-only pending-entry protocol in S3d, rather than adding a third stack. |
| `function/call::{prepare_bytecode_call,run_prepared_bytecode_call,function_env,direct_leaf_function_env}` | Callable preparation, new binding identity, actual arguments, cells, this/new.target/private/home, completion | S3b typed BindingPlan/context, S3d callee block writes/continuation. Named-self restoration becomes own lexical cell, never caller mutation. |
| `function/call::{call_function,call_function_slice}` generator/async branches | GeneratorStart/context retained beyond call, parameter prologue policy | S3b/c typed owned activation context; S3d boundary dispatch. |
| `function/call::call_virtual_function` | Params/bytecode/env for unmaterialized literal | S3e deletes virtual call entry/metadata dependence; ordinary closure/call semantics until S4 escape pass. |
| `function/call::{initialize_direct_instance_fields,call_field_initializer,construct_function,callee_this_realm_env}` | Field evaluation/define capability, receiver and creation realm, new.target | S3b typed fields, S3d cold constructor steps; S3e removes direct-leaf field executor while preserving constant metadata. |
| `vm_call_env::{realm_env,current_env,call_env,callee_env}` | Realm-only services, own function identity, or explicit dynamic caller lease | S3b deletes provenance/scrub/write-back shell. Dynamic visibility uses a lease, not a frame snapshot. |
| `vm_frame_init::frame_call_env` | Visible dynamic bindings, cell identity and context | S3b deletes slot-to-name/value materialization and apply-env cycle. |
| `vm_call::{call_direct_eval,selected_direct_eval_call_env}` | Parse context plus selected live lexical/dynamic chain, parameter conflicts, with/private/arguments flags | S3b uses T074 descriptors/leases; S3d eval is a driver frame with completion destination. |
| `vm_props::primitive_prototype_env` | Realm primitive prototype services | Borrow Context capability, S3b. |
| `compact_fn/activation::{CompactActivation::eval_binary,call_from_activation}` | Coercion callbacks/ordinary fallback over realm and cells | S3b typed service borrow; S3d callback boundary; S3e deletes driver. |
| `compact_fn/wide/activation::{WideActivation::eval_binary,to_property_key,eval_unary,to_numeric,update,construct_from_activation,call_from_activation}` | Callback-capable primitive/property/call services | S3b ports capability calls; no per-operation empty env. S3e deletes driver. |
| `compact_fn/wide/activation/exits::handle_exit` | Property/enumeration effect or exact interpreter recovery | Same capability and continuation ABI S3b/d; delete exit adapter S3e. |
| `compact_fn/property` read/write/index/global helpers | Realm prototypes, accessors, exotic/coercion callback handling | S2 property API consumer; S3b Context; useful helpers consolidate with common semantics by S3e. |
| `typed_loop/execute` guarded native/helper calls | Realm services, indexed cells and effect recovery | S3b Context; S3e common-frame/state-map port, S4 delete separate executor. |
| `conversion::to_primitive_method_env` / `sync_to_primitive_method_env` | Method realm, dynamic scope visibility only when required; result/throw | S3b removes env copying; S3d resumable coercion steps, no JS recursion hidden in sync helper. |
| `global::{indirect_eval_frame,native_eval_script}` | Selected realm/global declarative scope; no caller lexical scope | S3b typed global eval context; S3d driver frame. Direct eval remains separate. |
| `regexp::regexp_receiver_error_value` | Retained function native context and regexp/string state | Typed retained capture/realm; S3b. |
| `function/value::{new_user_with_options,creation_env}` | Function creation realm/prototypes and lazy observable properties | Typed creation context; S3b; serialize shared FunctionData edits with S1/S2. |
| `error::runtime_error_to_value` | Realm services for observable message/name coercion | Context capability; S3b/d. |
| `module/mod::eval_modules_with_resolver`; `module/link` module namespace/instantiation/evaluation/job helpers | Shared realm/live binding graph and host jobs | S3b typed ModuleContext; S3c suspension; S3d root/boundary driver. |

## Storage and argument vectors (not an allocation census)

`pop_arguments` builds a Vec even when general/native dispatch then borrows it.
Zero/one and fixed two/three argument direct-leaf VM calls use stack slices today;
therefore "every call allocates arguments" would be false. `call_function_slice`
allocates only after its direct-leaf predicate declines. Compact/wide fallbacks
copy a register argument slice to Vec; inline calls move to a fresh window.
Bound forwarding clones/extends stored arguments; apply/spread construct lists
with observable iteration and must keep explicit staging/close state.

General frames take pooled locals and an OperandStackRecycler vector; requesting
one is not necessarily malloc. They still construct FrameState, local upvalue
routes/masks, cold state when demanded and a nested Vm. Pools clear owners and
bound retention; shared cells cannot be pooled. Register storage stays owning
Rc Value through S3/S4; S5 changes representation behind the same borrow ABI.

S3c/d temporarily keep the old locals/operand storage **inside one owned block**;
S3e replaces it with RegisterBlock. The one-time storage migration is real work,
not a claim of zero rework. Do not flatten blocks into store[base + r] or resolve
FrameId per operation; hot stack/local slices and later regs are bound once per
non-reentrant segment. Pool requests, arguments staging and actual heap growth
must be counted separately in implementation acceptance.
