# T056 M2: diagnosis inputs for the frame/activation design revision

- Status: closed (diagnosis input). No runtime change comes from this work.
- Implementation: none on main. Diagnostic counters and the PC sampler are on
  `agent/compact-dispatch-diag/claude` (`d0a8094e`, `cc970167`), which is never merged.
- Verified at: development lane, main `330cf1ab` (canonical image `1f35cff0…`) for the
  gap profile; `d0a8094e`, built with the canonical recipe, for the compact-tier attribution. Both on
  2026-10-20 on the iMac (i7-10700).
- Evidence: this record. Raw job directories are in the workspace under
  `outputs/imac-bench/<job>/`; scripts and tables are in `tmp/dev-lane-gap/` and `tmp/compact-dispatch/`.
  These are local artifacts, not retained in Git.
- Unresolved: every number is a diagnostic sample, not a profile receipt. M2's plan needs fresh
  post-M1 receipts.
- Next action: none here. [T056](../T056-interpreter-core-v2.md) owns the M2 plan.

## Jobs

| Job | Kind | Content |
| --- | --- | --- |
| `20261020T084918Z-dev-lane-gap-counters-r2` | no-timing | `perf-counters` counts and `time -l` for 10 development cases, quickjs-rust and NG |
| `20261020T092230Z-dev-lane-gap-sample` | `--timing`, not DISTURBED | macOS `sample` (1 ms) of 7 cases on both engines |
| `20261020T094140Z-compact-dispatch-build-counters` | no-timing | per-handler `cw_*` counters of the wide executor |
| `20261020T094728Z-compact-dispatch-build2` | no-timing | canonical-recipe builds and a codegen check |
| `20261020T095156Z-compact-dispatch-pcsample` | `--timing`, not DISTURBED | exact-PC sampler, 500 µs, 65,539 samples |
| `20261020T095414Z-compact-dispatch-counters2` | no-timing | named-read slow path by outcome |

The sampled `wide::activation::run` is instruction-identical in the line-table build and the plain
build (5,391 instructions). Against the canonical `330cf1ab` image it has the same 24,736 bytes and 95.9%
matching instructions.

## Gap profile (7 libraries; quickjs-rust 28.7 s of samples, NG 7.5 s)

The cases are js-yaml, immutable, moment, diff, meriyah, lodash and graphlib. Helper leaves (allocator,
`mem*`, hash) are charged to their nearest semantic caller. Dispatch, call/frame/env and Value clone/drop
together account for 70.6% of the excess over NG. Call/frame/env alone is 5,756 ms, 26.4% of the
excess. NG inlines call setup and refcounting into `JS_CallInternal`, so only the combined bucket
compares like with like.

| Cost class (sampled owners) | ms | % of rust | Main symbols |
| --- | ---: | ---: | --- |
| Env assembly, export, reconciliation | 2,171 | 7.6 | `apply_env` 398, `DynamicBindings::get` 227 (via `CallEnv::get` and Vm construction) and `::cell` 54, `function_env` 199, `FrameBindings::insert` 193 and `::get` 162, `new_function_frame_with_capacity` 173, `current_env` 157, `refresh_realm_backed_locals` 113, `insert_frame_cell` 83, env teardown 286 |
| Nested Vm and executor plumbing | 1,854 | 6.5 | Vm construction (`new_with_globals…`) 637, `call_function` 311, `call_callee_with_marker` 246, `call_direct_leaf_function` 209, `eval_direct_call_bytecode` 129, `run_prepared_bytecode_call` 118 |
| Direct-leaf CallEnv build | 224 | 0.8 | `direct_leaf_function_env`: a CallEnv per leaf call (realm marker, module host, this realm) |
| Static facts resolved by name per call | 210 | 0.7 | `Bytecode::local_slot` 176 (hash lookups from Vm construction and `load_global`), `can_seed_slot_backed_call` 34 |
| Argument vectors | ~400 | 1.4 | `pop_arguments` 296 (from `call_resolved` and the generic loop), `Vec<Value>` clones; `mem*` adds 1,189 ms not attributable to one owner |
| Slot-kind dispatch on local access | 439 | 1.5 | `load_local_slow` 213 (from `op_load_local`), `store_local_slow` 89, `clear_local` 71, `load_global` 66 |
| Native boundary | 504 | 1.8 | `call_native_function` 297, `try_fast_global_native_call` 125, `Function.prototype.call` 82; each native call also builds an `empty_frame()` CallEnv. `call_native_family` 711 is mostly inlined builtin bodies |

Nested Vms are built for 3–12% of ordinary calls: moment 12.2%, meriyah 12.1%, js-yaml 10.0%, immutable 9.0%,
graphlib 8.9%. Calls are short, at 15.6–51.6 operations each. Call/frame/env takes 26% of lodash, 23% of diff
and 27% of immutable.

## Compact tier (wide executor; 30.9 s unsampled wall over the same 7 cases)

`run` self time is 6.43 s (20.8% of wall) over 617.6M operations, 10.4 ns per operation. The dispatch block
itself (fetch, decode, indirect jump) is 1.2 ns per operation.

| Mechanism inside `run` | ms | Evidence |
| --- | ---: | --- |
| Driver round trip | 1,132 | Every call, return and exit leaves the inner loop. `Result<Action, RuntimeError>` is built in memory with unaligned narrow stores, then reloaded. The outer loop re-resolves the program, bytecode and upvalue owner on each of 63.9M entries. |
| Return clears the whole window | 251 | `clear_window` tests every register of the frame, whether or not it was written |
| Driver value moves | 199 | callee, receiver and argument registers moved one at a time |
| Value clone/move/drop | 1,606 | 16-byte values pass through stack temporaries; immediates store a 1-byte tag and reload 8 bytes; heap clones call `Value::clone` out of line |
| Upvalue read | 263 | per read: two u128 bit tests and a u128 popcount (`cell_received_upvalue_index`), plus a `RefCell` borrow of the cell; 16.7 ns self |

- A compact-to-compact call round trip costs about 78 ns (16.5M calls).
- Calls from compact code to anything else (8.1M) go through `call_from_activation`. That path builds a
  CallEnv, and for non-leaf bodies a nested Vm. It carries 15.4 s of wall under it, including the callees'
  real work.
- Exits (14.4M, all of which resume compact execution) cost 82 ns on average including their work.

Named reads in the compact tier: 71% of 44.6M miss the inline hot entry. The slow path splits as follows:

| Slow-path outcome | Share of slow reads |
| --- | ---: |
| Polymorphic site: cache hit on a non-hot entry | 37% |
| Text lookup that refills the cache | 18% |
| Receiver is not an ordinary object (arrays 4.16M, strings 3.18M, functions 1.79M) | 28% |
| General get | 11% |
