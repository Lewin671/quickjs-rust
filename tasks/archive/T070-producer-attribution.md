# T070: Visible ownership producers in the exact base profiles

Supplemental diagnosis, 2026-10-23 UTC. Source is the three complete windows
per target in `20261023T052723Z-t070-exact-profiles-r2`, exact `b93e4daa`,
binary `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
Targets remain frozen; no held-out source or profile is used.

## Method and limits

Use the [T068 taxonomy v2](T068-profile-evidence.md) and its first-main-thread
self counts. Keep every node whose nearest recognized owner is value-rc,
then walk its ancestor chain to the first recognized category other than
value-rc. Attribute the self count to that exact canonical symbol. Include
all three windows; assert conservation against the ownership totals.

Original analyzer SHA-256: `2af736a75d8491c02b2af60921f300d3e858dcfd64a596d058ddaa053f85e9a7`;
producer `20261023T044006Z-t068-profile-audit-v2/out/analyze.py`. This is a
visible caller classification, not a bytecode-operation price or a causal
wall attribution. Inlined execution remains opaque. A wide caller can include
call preparation, property work and teardown as well as register transport;
none of these shares is a removable fraction.

Big.js has only 41/877 ownership self samples assigned to the wide driver,
while 651/877 have the typed-loop executor as the nearest semantic caller.
The candidate rewrites wide IR, not typed-loop boxed registers or dense-array
element reads. This difference in visible reach is consistent with its flat
big.js triage; it does not prove why any wall ratio changed. A later transport
unit needs per-tier consumption/recovery evidence before freezing targets.

## development/graphlib/paths

Pooled main-thread samples 3225; ownership self samples 620.

| Nearest visible semantic caller | Ownership self samples |
| --- | ---: |
| `qjs_runtime::bytecode::compact_fn::wide::activation::run` | 410 |
| `<qjs_runtime::bytecode::vm::Vm>::run_current_activation` | 23 |
| `qjs_runtime::function::call::run_prepared_bytecode_call` | 17 |
| `qjs_runtime::bytecode::compact_fn::property::try_plain_set_prop` | 13 |
| `core::ptr::drop_in_place::<qjs_runtime::function::env::CallEnv>` | 11 |
| `<qjs_runtime::value::object::ObjectRef>::write_existing_own_data_property` | 11 |
| `qjs_runtime::native::call_native_function` | 9 |
| `qjs_runtime::function::call::function_env` | 8 |
| Other visible callers | 118 |

## development/big.js/arith

Pooled main-thread samples 5418; ownership self samples 877.

| Nearest visible semantic caller | Ownership self samples |
| --- | ---: |
| `qjs_runtime::bytecode::typed_loop::execute::run::<qjs_runtime::bytecode::compact_fn::wide::loop_frame::WideLoopFrame>` | 651 |
| `<qjs_runtime::value::array::ArrayRef>::direct_dense_index_value` | 87 |
| `qjs_runtime::bytecode::compact_fn::wide::activation::run` | 41 |
| `qjs_runtime::bytecode::compact_fn::wide::activation::run_typed_loop_here` | 24 |
| `qjs_runtime::bytecode::typed_loop::execute::write_back::<qjs_runtime::bytecode::compact_fn::wide::loop_frame::WideLoopFrame>` | 19 |
| `<qjs_runtime::bytecode::typed_loop::TypedLoopScratch>::clear` | 14 |
| `<qjs_runtime::value::array::ArrayRef>::define_plain_elements` | 7 |
| `qjs_runtime::bytecode::compact_fn::wide::activation::exits::exit_to_interpreter` | 5 |
| Other visible callers | 29 |

## development/immutable/persistent

Pooled main-thread samples 9251; ownership self samples 1123.

| Nearest visible semantic caller | Ownership self samples |
| --- | ---: |
| `qjs_runtime::bytecode::compact_fn::wide::activation::run` | 575 |
| `<qjs_runtime::bytecode::vm::Vm>::run_current_activation` | 75 |
| `qjs_runtime::native::call_native_family` | 61 |
| `qjs_runtime::function::call::run_prepared_bytecode_call` | 49 |
| `<qjs_runtime::bytecode::vm::Vm>::load_local_slow` | 42 |
| `qjs_runtime::value::teardown::release` | 21 |
| `qjs_runtime::function::call::call_function` | 19 |
| `qjs_runtime::property::property_value_key_with_receiver` | 18 |
| Other visible callers | 263 |

