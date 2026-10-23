# T068 profile evidence: collection, attribution and per-program data

Owner/current interpretation: [T068](../T068-post-t066-profile.md). Measured source `f49be95c798475f9ed9bb55b9138f6ca3c081413`,
2026-10-23 UTC. No engine change. Numerical tables below are repository-retained evidence;
raw samplers, workload bundles and analysis code are external artifacts, not assumed durable Git inputs.

## Exact procedure and interpretation

The canonical build used `scripts/lib.sh::qjs_build_revision`, which constructs a clean temporary worktree
and invokes `tools.benchmark.build_cache build-rust`. The binary was built once, then only hash-checked/read.
The existing pinned NG binary was reused. The sampler process targeted the direct engine PID (not `/usr/bin/time`).
Each complete, unamplified development script ran three times per role. `sample` attached immediately after
Popen, with duration 60 s, interval 1 ms and -mayDie, then the collector waited for the script and checked stdout.
All 60 engine processes ended before sampler return; every sample graph is usable. The 60 instruction runs
were separate fresh processes, three alternating role pairs, using the same bundled scripts and binaries.
Rust argv uses `--raw`, NG uses `--script`; NG prints one external sentinel, Rust two; checksums agree.

Resolve only `development` through `tools.benchmark.screen.resolve_cases` and the existing verified npm loader.
No held-out bundle, source or profile was read. No source-recognizing optimization, new dependency or FFI.
The profile collector itself is frozen in the collection job's command.sh; the final analyzer in the v2 audit's out/analyze.py.

The analyzer reads only the first main-thread `sample` call tree. Depth is the count column; self count is
node count minus direct children. It asserts nonnegative self counts and exact conservation, totals each
program's three repeats, and assigns every self sample one owner. Rust v0 symbols use the repository's
structural `tools.benchmark.layout_symbols.parse_symbol`; unsupported names retain their raw spelling.
Ancestor traversal charges transparent libc memory/hash/vector helpers to the nearest recognized category.
It does not move recognized allocator/ownership self samples wholesale into their semantic callers.
Inclusive counts union categories on each leaf's self-sample stack; recursion counts once per category/symbol.
Unknown stacks are other. Generic helpers' symbol rows can span categories even though each sample is assigned once.

Classification is an implementation-owner partition, not a disjoint semantic partition of JavaScript operations.
First matching rules win, in the order described below; otherwise climb the caller stack. The complete ordered
regex implementation and its hash are preserved by the v2 audit job, not maintained as production tooling.

| Rule order | Rust matching families and interpretation |
| --- | --- |
| 1 | `qjs::allocator`, SizeClassAllocator, libc malloc/realloc/free/mmap and page helpers → allocation; generic TLS alone inherits its caller. |
| 2 | `named_property_cache`, CachedValue, prototype/enum caches → named-cache, including their inline guards. |
| 3 | PropertyStorage/PropertyLayout/dynamic_storage, own_data_property/own_property_slot and ObjectRef property/layout access → property-storage. |
| 4 | qjs_parser/qjs_lexer, bytecode compile/compiler, visible realm/context initialization → parse-compile. |
| 5 | Context/realm drops, value::teardown → teardown; CallEnv/FrameBindings/DynamicBindings/function_env/apply_env/local-slot/slow-local work → environments. |
| 6 | Vm constructors/prepared/direct-call frame setup, argument pop/initial-slot work and frame drops → calls. |
| 7 | Remaining clone/drop_in_place/drop_slow/Weak upgrade/drop and compact execute::store_owned → value-rc. Container work is included; this is not pure RC instructions. |
| 8 | regexp/regex symbols → regexp; dedicated array/object/string/json/date/math modules → respective builtin families. |
| 9 | Indexed/dense/typed-array access and value::array/property::array → elements. Thus dedicated builtin Array frames and element primitives differ. |
| 10 | Named handlers, property access, prototype receiver/live-slot/literal-property and Function own-property routes → named-other. |
| 11 | Remaining environment/upvalue/binding/capture/lexical work → environments; native::call_native_family → opaque builtin-other. |
| 12 | Call dispatch/preparation, native call wrappers, wide inline_entry and exit_to_interpreter → calls. |
| 13 | JsString/WTF-16/Unicode/std string and str helpers → strings; visible throw/exception work → exceptions. |
| 14 | Typed/compact executors, interpreter handlers and number/coercion helpers → loop; remaining builtin-family/native symbols → builtin-other. |

NG classification likewise follows its symbols: libc and js_malloc/realloc/free → allocation;
free/dup value/object/property/shape → value-rc; RunGC/mark/scan/FreeContext/FreeRuntime → teardown;
NewAtom/atomize → strings; parser/compiler/initialization → parse-compile; lre/regexp → regexp;
dedicated Array/Object/String/JSON/Date/Math functions → builtin families; explicit fast-array/index
helpers → elements; generic Get/SetPropertyInternal/Value → mixed property handlers; shape/find/add/define
property → storage; closure/varref → environments; CallInternal → opaque loop and other call wrappers → calls;
throw → exceptions; strings/interning → strings; arithmetic/comparison/coercion helpers → loop; other JS_* → builtin-other.
NG has different inlining/architecture: a zero visible named-cache or exception cell is not zero semantic work.
Runtime NewAtom was mislabelled compile and several coercion helpers builtin-other in the first successful audit;
v2 corrects those and the other-category inclusive fallback, with identical raw profiles and instruction data.
No failed sampling job, discarded window, outlier removal, timing rerun or favorable data selection exists.

## Denominators and repeat stability

Counts are time-sampling observations, not retired instructions. They include sampled waits/system work on
the main thread. Do not convert 1 ms × count into exact CPU/wall durations or infer category CPI.
Sampler attachment loses the earliest interval; a process-complete end does not restore that missing startup.
Per-program category percentages are exactly 100 × the counts in the next tables divided by the pooled N below.
The 1/10-weighted percentage tables in T068 use those fractions; zero observed samples is not proof of no cost.

| Program (development/) | Rust repeat N (pooled) | NG repeat N (pooled) |
| --- | --- | --- |
| lodash/transform | 1469, 1480, 1495 (**4,444**) | 555, 551, 558 (**1,664**) |
| meriyah/parse | 2245, 2250, 2250 (**6,745**) | 695, 690, 692 (**2,077**) |
| immutable/persistent | 3085, 3139, 3072 (**9,296**) | 858, 849, 852 (**2,559**) |
| moment/dates | 8699, 8696, 8591 (**25,986**) | 2603, 2600, 2581 (**7,784**) |
| diff/compare | 4845, 4882, 4981 (**14,708**) | 1605, 1600, 1606 (**4,811**) |
| papaparse/csv | 1754, 1758, 1759 (**5,271**) | 891, 894, 889 (**2,674**) |
| js-yaml/document | 3672, 3676, 3676 (**11,024**) | 909, 898, 895 (**2,702**) |
| big.js/arith | 1810, 1841, 1796 (**5,447**) | 735, 738, 736 (**2,209**) |
| matter-js/simulate | 2597, 2625, 2598 (**7,820**) | 1159, 1156, 1170 (**3,485**) |
| graphlib/paths | 1082, 1078, 1083 (**3,243**) | 391, 390, 389 (**1,170**) |

Columns below abbreviate the programs in manifest order: Lod=lodash, Mer=meriyah, Imm=immutable, Mom=moment, Dif=diff, Papa=papaparse, YAML=js-yaml, Big=big.js, Matter=matter-js, Graph=graphlib.
Each cell is **self/inclusive sample counts**. Self counts sum to pooled N; inclusive counts overlap.

## RUST per-program categories

| Category | Lod | Mer | Imm | Mom | Dif | Papa | YAML | Big | Matter | Graph |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| loop | 2339/4393 | 2802/6729 | 4092/9286 | 8827/25981 | 4845/14708 | 1201/5271 | 4255/11023 | 3938/5446 | 5071/7804 | 1249/3243 |
| named-cache | 33/118 | 424/620 | 172/238 | 859/1283 | 332/608 | 52/73 | 364/448 | 10/11 | 334/623 | 47/78 |
| property-storage | 37/39 | 698/934 | 311/365 | 1853/2187 | 679/735 | 369/437 | 560/653 | 12/14 | 334/375 | 265/339 |
| named-other | 153/1147 | 531/2009 | 494/1223 | 2575/5936 | 1424/3990 | 322/1068 | 660/1822 | 39/101 | 753/1594 | 167/372 |
| elements | 13/45 | 17/19 | 34/101 | 23/53 | 568/777 | 33/44 | 3/6 | 406/520 | 168/247 | 17/25 |
| calls | 184/4344 | 573/6721 | 1123/9085 | 2798/25535 | 911/14689 | 293/5252 | 934/11016 | 49/5431 | 112/7805 | 305/3228 |
| environments | 966/1091 | 443/566 | 948/1287 | 1946/2309 | 2084/2688 | 347/394 | 915/1175 | 53/58 | 44/64 | 293/490 |
| value-rc | 448/484 | 769/808 | 1233/1332 | 2815/3115 | 1694/2007 | 446/503 | 1023/1062 | 832/837 | 892/899 | 569/599 |
| allocation | 140/140 | 405/405 | 585/585 | 1307/1307 | 1254/1254 | 373/373 | 689/689 | 36/36 | 45/45 | 220/220 |
| strings | 22/26 | 10/17 | 3/9 | 99/117 | 235/293 | 51/60 | 73/92 | 8/8 | 6/12 | 23/35 |
| regexp | 0/1 | 4/9 | 9/9 | 1005/1274 | 267/883 | 1443/2168 | 978/1526 | 16/28 | 3/5 | 0/0 |
| builtin-Array | 20/2448 | 7/31 | 23/302 | 269/396 | 50/658 | 46/2917 | 4/15 | 30/70 | 11/72 | 21/2765 |
| builtin-Object | 2/9 | 7/29 | 5/10 | 62/221 | 24/78 | 7/26 | 51/231 | 1/17 | 3/4 | 23/69 |
| builtin-String | 11/12 | 30/43 | 18/26 | 116/787 | 61/975 | 207/956 | 298/510 | 11/21 | 1/6 | 9/13 |
| builtin-JSON | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| builtin-Date | 0/0 | 0/0 | 0/0 | 206/521 | 9/9 | 0/0 | 1/2 | 0/0 | 2/2 | 0/0 |
| builtin-Math | 0/0 | 0/0 | 2/7 | 5/16 | 20/70 | 0/0 | 6/13 | 0/0 | 8/29 | 0/0 |
| builtin-other | 42/3436 | 24/68 | 212/1232 | 1165/5236 | 140/1617 | 79/3803 | 209/818 | 3/92 | 27/155 | 30/2803 |
| exceptions | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 1/1 | 0/0 | 0/0 | 0/0 |
| parse-compile | 21/45 | 1/12 | 4/5 | 0/0 | 0/0 | 0/0 | 0/0 | 0/1 | 6/10 | 0/0 |
| teardown | 12/16 | 0/0 | 26/67 | 53/145 | 111/203 | 2/21 | 0/0 | 2/7 | 0/0 | 5/8 |
| other | 1/1 | 0/0 | 2/2 | 3/3 | 0/0 | 0/0 | 0/0 | 1/1 | 0/0 | 0/0 |

## NG per-program categories

| Category | Lod | Mer | Imm | Mom | Dif | Papa | YAML | Big | Matter | Graph |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| loop | 1185/1639 | 918/1943 | 1260/2540 | 3268/7782 | 1525/4809 | 535/2615 | 1561/2695 | 1384/2209 | 3215/3482 | 503/1153 |
| named-cache | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| property-storage | 29/173 | 126/670 | 139/526 | 367/1806 | 375/1671 | 35/223 | 33/149 | 23/87 | 1/4 | 219/402 |
| named-other | 34/87 | 93/645 | 91/290 | 481/1494 | 119/582 | 103/226 | 97/167 | 294/592 | 99/223 | 69/460 |
| elements | 40/62 | 9/9 | 63/254 | 21/72 | 22/73 | 18/138 | 13/17 | 279/334 | 111/111 | 19/26 |
| calls | 8/1147 | 12/47 | 12/591 | 244/4409 | 17/684 | 55/2027 | 51/743 | 4/243 | 5/68 | 4/1053 |
| environments | 4/60 | 0/0 | 9/102 | 23/734 | 1/18 | 0/0 | 3/64 | 0/0 | 0/0 | 0/1 |
| value-rc | 14/90 | 41/180 | 38/287 | 144/701 | 111/625 | 22/208 | 23/136 | 2/25 | 2/6 | 100/151 |
| allocation | 240/240 | 710/710 | 848/848 | 2400/2400 | 2310/2311 | 983/983 | 554/554 | 159/159 | 26/26 | 164/164 |
| strings | 34/48 | 28/28 | 14/22 | 240/312 | 139/422 | 74/157 | 83/134 | 1/14 | 8/13 | 57/77 |
| regexp | 0/0 | 0/2 | 0/0 | 305/528 | 87/478 | 561/1149 | 214/394 | 8/28 | 0/0 | 0/0 |
| builtin-Array | 33/1144 | 23/52 | 36/329 | 28/80 | 19/325 | 32/1423 | 1/8 | 21/163 | 7/45 | 11/1047 |
| builtin-Object | 1/5 | 0/0 | 1/12 | 25/552 | 0/0 | 0/0 | 12/206 | 0/0 | 0/0 | 0/0 |
| builtin-String | 15/15 | 6/20 | 5/9 | 13/313 | 17/415 | 139/619 | 27/84 | 2/17 | 0/0 | 3/3 |
| builtin-JSON | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| builtin-Date | 0/0 | 0/0 | 0/0 | 24/57 | 1/1 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| builtin-Math | 0/0 | 0/0 | 0/0 | 10/10 | 12/12 | 0/0 | 1/1 | 0/0 | 0/0 | 0/0 |
| builtin-other | 9/1641 | 21/1944 | 28/2540 | 185/7783 | 55/4809 | 90/2615 | 28/2695 | 32/2209 | 9/3482 | 11/1152 |
| exceptions | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| parse-compile | 0/1639 | 0/1943 | 0/2540 | 2/7782 | 0/4809 | 2/2615 | 0/2695 | 0/2209 | 0/3482 | 0/1152 |
| teardown | 18/41 | 90/197 | 15/29 | 4/5 | 1/3 | 25/83 | 1/8 | 0/0 | 2/3 | 10/23 |
| other | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |

## Top 25 Rust symbols

Ranked by equal-program weighted self share, not pooled sample count; inclusive is recursion-unioned.
Memory/TLS helpers list their largest charged owner; their total self share spans multiple categories.

| Symbol (qjs_runtime:: prefix omitted) | Self % | Inclusive % | Owner category |
| --- | ---: | ---: | --- |
| `bytecode::compact_fn::wide::activation::run` | 24.314 | 94.642 | loop |
| `bytecode::typed_loop::execute::run::<bytecode::compact_fn::wide::loop_frame::WideLoopFrame>` | 8.342 | 11.863 | loop |
| `<value::Value as core::clone::Clone>::clone` | 5.807 | 5.807 | value-rc |
| `<bytecode::vm::Vm>::run_current_activation` | 4.572 | 99.807 | loop |
| `core::ptr::drop_in_place::<value::Value>` | 3.396 | 3.396 | value-rc |
| `<std::thread::local::LocalKey::<qjs::allocator::Pool>>::with::<qjs::allocator::allocate::{closure}, O<u8>>` | 2.369 | 2.794 | allocation |
| `bytecode::compact_fn::execute::store_owned` | 1.823 | 2.052 | value-rc |
| `_platform_memcmp$VARIANT$Base` | 1.736 | 1.736 | mixed; property-storage largest |
| `_platform_memmove$VARIANT$Haswell` | 1.466 | 1.466 | mixed; calls largest |
| `<regexp::matcher::first::FirstMatcher>::match_group_once_to` | 1.350 | 2.606 | regexp |
| `<bytecode::vm::Vm>::new_with_globals_upvalues_with_stack_and_direct_call_slots` | 1.283 | 2.450 | calls |
| `<value::object::ObjectRef>::own_data_property_read` | 1.271 | 1.777 | property-storage |
| `<bytecode::named_property_cache::NamedPropertyCache>::probe` | 1.208 | 2.120 | named-cache |
| `native::call_native_family` | 1.000 | 25.415 | builtin-other |
| `<value::object::ObjectRef>::write_existing_own_data_property` | 0.996 | 1.404 | property-storage |
| `bytecode::compact_fn::property::get_prop_named` | 0.978 | 6.558 | named-other |
| `<bytecode::vm::Vm>::apply_env` | 0.937 | 1.591 | environments |
| `<std::thread::local::LocalKey::<qjs::allocator::Pool>>::with::<qjs::allocator::release::{closure}, ()>` | 0.860 | 1.185 | allocation |
| `tlv_get_addr` | 0.820 | 0.820 | mixed; allocation largest |
| `bytecode::compact_fn::wide::activation::exits::exit_to_interpreter` | 0.815 | 19.212 | calls |
| `regexp::matcher::atom_end` | 0.750 | 0.792 | regexp |
| `<value::object::ObjectRef>::literal_data_slot_value` | 0.745 | 1.242 | named-other |
| `<bytecode::vm::Vm>::op_get_prop_named` | 0.724 | 3.164 | named-other |
| `<value::object::PropertyStorage>::get` | 0.718 | 1.006 | property-storage |
| `<value::array::ArrayRef>::direct_dense_index_value` | 0.658 | 1.311 | elements |

## NG top 25 symbols

Same equal-case weighting as Rust; these are NG's internal category denominators, not shares of Rust's excess cost.

| Symbol (qjs_runtime:: prefix omitted) | Self % | Inclusive % | Owner category |
| --- | ---: | ---: | --- |
| `JS_CallInternal` | 48.004 | 98.723 | loop |
| `nanov2_pointer_size` | 7.551 | 7.551 | allocation |
| `nanov2_allocate_from_block` | 3.014 | 3.014 | allocation |
| `nanov2_free_to_block` | 2.627 | 2.826 | allocation |
| `nanov2_allocate` | 2.520 | 5.671 | allocation |
| `lre_exec_backtrack` | 2.385 | 3.512 | regexp |
| `js_get_fast_array_element` | 2.349 | 2.349 | elements |
| `JS_GetPropertyValue` | 1.864 | 5.588 | named-other |
| `_platform_memmove$VARIANT$Haswell` | 1.766 | 1.766 | mixed; property-storage largest |
| `JS_GetPropertyInternal` | 1.444 | 3.129 | named-other |
| `malloc_size` | 1.395 | 6.435 | allocation |
| `add_property` | 1.087 | 10.867 | property-storage |
| `js_clone_shape` | 1.058 | 2.332 | property-storage |
| `js_free_shape0` | 0.975 | 1.378 | value-rc |
| `nanov2_size` | 0.945 | 0.949 | allocation |
| `free` | 0.793 | 2.848 | allocation |
| `js_call_c_function` | 0.725 | 33.247 | calls |
| `js_relational_slow` | 0.671 | 1.159 | loop |
| `_malloc_zone_malloc` | 0.642 | 6.111 | allocation |
| `__JS_NewAtom` | 0.639 | 1.000 | strings |
| `js_eq_slow` | 0.604 | 0.972 | loop |
| `JS_SetPropertyInternal2` | 0.552 | 6.162 | named-other |
| `JS_NewObjectFromShape` | 0.545 | 4.766 | property-storage |
| `JS_ToPrimitiveFree` | 0.504 | 0.557 | loop |
| `add_shape_property` | 0.488 | 3.905 | property-storage |

## Ranked-unit reach by program

Cells are exclusive self percentages for the family used in each proposal's upper envelope.
The narrower proposed implementation cannot be assumed to remove its entire family's costs.

| Program | Calls + env % | All property % | Value ownership % | Allocation % | RegExp % |
| --- | ---: | ---: | ---: | ---: | ---: |
| lodash/transform | 25.878 | 5.018 | 10.081 | 3.150 | 0.000 |
| meriyah/parse | 15.063 | 24.507 | 11.401 | 6.004 | 0.059 |
| immutable/persistent | 22.278 | 10.510 | 13.264 | 6.293 | 0.097 |
| moment/dates | 18.256 | 20.346 | 10.833 | 5.030 | 3.867 |
| diff/compare | 20.363 | 16.556 | 11.518 | 8.526 | 1.815 |
| papaparse/csv | 12.142 | 14.096 | 8.461 | 7.076 | 27.376 |
| js-yaml/document | 16.772 | 14.369 | 9.280 | 6.250 | 8.872 |
| big.js/arith | 1.873 | 1.120 | 15.274 | 0.661 | 0.294 |
| matter-js/simulate | 1.995 | 18.171 | 11.407 | 0.575 | 0.038 |
| graphlib/paths | 18.440 | 14.770 | 17.545 | 6.784 | 0.000 |

RegExp subject preparation lives in `crates/qjs-runtime/src/regexp/matcher.rs` (`PreparedInput`); atom/group scanning is under its `matcher/` tree. `fast_scan.rs` already classifies/scans quantified single-code-point atoms; the proposal targets repeated metadata decoding in those existing paths.

## Artifact provenance

Workspace fetch root: `outputs/imac-bench/<job>/` (the wrapper places it at the co-work-space root).
These are external artifacts; the numerical observations above and the summary task are the durable conclusions.
Final v2 receipt verification is 30/30 Rust and 30/30 NG using `profile.verify_profiles` with each role's exact
source/image identity and copied-asset hashes. This diagnostic identity binding is not a sealed formal queue
or `validate-unit` approval. A future unit still needs that fresh queue. Original PID, raw-profile, stdout,
workload and executable digests were independently checked, plus all 60 raw instruction/stdout records.

| Artifact | SHA-256 |
| --- | --- |
| `20261023T042914Z-t068-canonical-identity/command.sh` | `700861109ee222f9057b28710fb35d03c732f28409776b8f241ea1572684dfb7` |
| `20261023T042914Z-t068-canonical-identity/out/identity.json` | `92746223c7cf11ea0710fd805b673fd2f92561fe8710a07262ef4716f39dd52d` |
| `20261023T043027Z-t068-development-exact-profiles/command.sh` | `5e90e808e94b64cb4b0c8601f93db22fc7cda6fe281b3b7978b752993c282d3a` |
| `20261023T043027Z-t068-development-exact-profiles/intruders.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `20261023T043027Z-t068-development-exact-profiles/out/selection.json` | `890b9d33f37b74161159e2667d79404e931f37746702b2d2723aad22ef8b3869` |
| `20261023T043027Z-t068-development-exact-profiles/out/identity.json` | `92746223c7cf11ea0710fd805b673fd2f92561fe8710a07262ef4716f39dd52d` |
| `20261023T043027Z-t068-development-exact-profiles/out/profiles.json` | `b070737ee70bc51ee19e69dab39dc80a56d980bb10a3a41f269c4da4df7db012` |
| `20261023T043027Z-t068-development-exact-profiles/out/instructions.json` | `2b8ea4b01d4eb3bb971aa0af65bb86923d75d52e7f326f0da151850378243536` |
| `20261023T043827Z-t068-profile-audit/command.sh` | `10c9204983bd567e8caa51bfd145a12086f8ad10505a163ae6e790de5e161d58` |
| `20261023T043827Z-t068-profile-audit/out/analysis.json` | `e0a0f0fe2b529a6395d5801ac7749a2eefb21251fcdc009f51cd4c9e39bc5bc0` |
| `20261023T044006Z-t068-profile-audit-v2/command.sh` | `aeda9838cd1dfa7083356e39e361ca4f44f3a6cae162f4d2604259cf732d391a` |
| `20261023T044006Z-t068-profile-audit-v2/out/analyze.py` | `2af736a75d8491c02b2af60921f300d3e858dcfd64a596d058ddaa053f85e9a7` |
| `20261023T044006Z-t068-profile-audit-v2/out/analysis.json` | `837bfa721f013cb4569a11b97962b899fe660567c88a5babbbbb712a68e700c3` |

### Workload identities

All six profiles and six instruction executions per program use the same bundle hash.
The pinned loader also verifies npm tarball/driver inputs before producing that bundle.

| Program | Workload bundle SHA-256 |
| --- | --- |
| lodash/transform | `991439c433c02d22d7c3bd49246059cf1fbb3b0b5d0500d34ea1b7f68925583b` |
| meriyah/parse | `349b451206a4f46670b0ca0f5fd1cc27e459b411806231b5203390d8909c6bd1` |
| immutable/persistent | `98aff279f5ab31b94f89cce2dcc94616744c7186b70b2e16466e4640c8fc5ec1` |
| moment/dates | `54b2f358cc54092074a4460f199771508cdc8369170027a47446d7b6761c1090` |
| diff/compare | `fbb64c6f59b7718700595025d56b2aa6f5413bba10d41f9fca158ac31d6d00eb` |
| papaparse/csv | `f417c24c26a51bb1b8c2e288887530e3013a68e4cc7cd9c46cb04beb7a8dd1e7` |
| js-yaml/document | `fc1411db299a717af599ac2e308e68018df72c79cca569c9f3e354e40debe9fa` |
| big.js/arith | `6a044d09d50474adc03898681e4308740c5852975c411d36330871e4b8a4d346` |
| matter-js/simulate | `e96eb031af93f5afa39748c115421cad8a1e8c2b9cf7a04b118fe4a013a75fcc` |
| graphlib/paths | `87d777c0409dea06f66052bfa763379f6370cdad05a97301f79dd7e8fbddadaf` |

### Individual receipt/sample identities

Profile-relative source is `out/profiles/development--<package>--<case>/<role>/<repeat>/receipt/receipt.json`.
`profile.sample` inside each receipt directory is byte-identical to the original `../raw.sample`.
Base source for Rust is f49be95c; NG is f7830186; full source/binary hashes are in the summary task.

| Program | Role | Repeat | Receipt SHA-256 | Copied/raw sample SHA-256 |
| --- | --- | ---: | --- | --- |
| lodash/transform | rust | 0 | `5322d99e190d2c830e675d2caa40a38383a81b4256e96ee5255f34bc1080641a` | `95a17c36db3bd1dfe4bd122df4bcd837b905d3ccfc5b30ab90311101a0d593db` |
| lodash/transform | ng | 0 | `69845414c46268f7a28c14d4f5052aef5248d683eefd9efd47919715908c9659` | `a2f36bc6d72e9269141b640615b3b8713d8c30fcf9ef247ed87c5930e9e806b1` |
| lodash/transform | ng | 1 | `7841a7aafd3a5b289069a588b3ae9d6477456532f4d4b4372361fa4e4aa23023` | `c4ada82b356a0af2dd37a6fc602a285ddf85b6609d7a93faa57faac173a8b1a8` |
| lodash/transform | rust | 1 | `8a2f5a64080c2a21c13d9c9a8d4fc286f7601999ff1d2e93c15175c8e077aedd` | `848b41f04083bafe015b07d0bcb32559d74d662634807069da4e1663f85d28b6` |
| lodash/transform | rust | 2 | `30eb381afedd3e4ebe4e34c40f9492f355c6cc512475827b64a089aba53e6f6a` | `f268cf04ab96e48ef4f33452260f30c4bdec33a833a77752ec21bdc95b31053e` |
| lodash/transform | ng | 2 | `468d62b210e21f9116ca916e46a1bfcb45267b3fe69634e22950c7fd61c5d137` | `a3ffd99005edf7b189595ce2efe4d8a6adfd3354f99bae6d982930c5d0a04a79` |
| meriyah/parse | rust | 0 | `0000f3ec5c480abf267afedee2edd363c0c01e8d47cb97a36a34761fb91be482` | `ca12d124ff3cb51c5c54534fd77a5016142fd09fac2f9bfa5142aade4250e111` |
| meriyah/parse | ng | 0 | `d6cc4411171add66be571a07e650262e14873580d444b5e3b4eaaa13b15f58db` | `3c329d9c6f1423ab968240fd2b64a1a69bd30d3fc706ba71afa258dad36af399` |
| meriyah/parse | ng | 1 | `b59bbea19b1d1bcf15e018407705df7b9a965a62a01b469f123655d124f926c4` | `04a9c567d24b1a173ae6d2e439b7a6236ead91286afea907415d026ba29bf04f` |
| meriyah/parse | rust | 1 | `80b991091ca9675babb4988dfcdaf5d18406f2df68c8681a6945f17a8bfaaa4f` | `f6d2e23fa72368a88dec2b4a1aeba66beb82d0b2c28cb92e9ce04462745426b0` |
| meriyah/parse | rust | 2 | `46602d88ba77ccf80b27fa71cbe0d71f3e9caf27aa36a8e214d689a76b7a58bb` | `5f7522e11c0d354a7435440ef078a683faed2cce835f15f3ba5d9e7484b01d4b` |
| meriyah/parse | ng | 2 | `ddc02ad71423c13d88161d38eab4831909bb9fdeba65a0d30fbbd28e9edf85f4` | `16171a77ae762cb1bf18e4272dbdcb6196ed3ad7eed765d2d7b796b281efcd01` |
| immutable/persistent | rust | 0 | `db5154900e2f12e54fbcf85ab3101acb66816607d32ea6f7535f45e6bd19b61e` | `32356addb89341ac0c217e171b3a78c980b71c5d92b3d925f625dfb7b33bd778` |
| immutable/persistent | ng | 0 | `28b22d8aaeff306328b9049f1eb0923b5ae79a1019069104b2501b98e80ee5a6` | `7acffe65e0691af58009edd894e7c1edc48a7379acda46ec27a4b533974d0dbd` |
| immutable/persistent | ng | 1 | `241669a499e4e43ea95cb7aabcfc92d3b9fa400e06984fd26036cc171586bd66` | `cbcddb9a309d479094534224db93d093fb4c3a99217e786b03e72134bce46d26` |
| immutable/persistent | rust | 1 | `6a74a10fc10a502c261400827dd43750e7e2c4fd254119ba3630c439ec3b864d` | `deb49e1e976ac02e49c3a76a4b86b7f6eb4de9f00d079859560615ae566cc93a` |
| immutable/persistent | rust | 2 | `4b6026158dce244d9d16f3a1f242d978236647f0c74eb5cac99a80407af74947` | `036a23ccdeefb11148a7e93af40a7bc632d0753e9ffcb0a1219fe574fb297cf0` |
| immutable/persistent | ng | 2 | `b1d96d751a089503e6bbc1210c391a0e8f58c22eb71fd92b0519ec9436660d86` | `3141c8448da78be939d6cee687d6e627a6dcfa5b09fa29938d0e03d4856d6efc` |
| moment/dates | rust | 0 | `42cef72b9fac2d06178918c6db008ad9e38cd0876175a6576ca83b6b037c326b` | `37bba6e4a19e93a132ceed48ce4b8f8f2e2502684175548f08d08d0ff87f81cd` |
| moment/dates | ng | 0 | `61bc76d327c6fa2bb5a89b4e17f650c7ee72ee62cc326bdaaa38682cf1908687` | `64ba8931b6e4636bb57fa57fccf3687a8225160fec647d00a5f38e55356d399f` |
| moment/dates | ng | 1 | `52842b14b3b8a45098344cda1f78adec73aad9279a8fa1eb3bc07d13de4d98d7` | `13ec338afbd03ef14e39bd5df9cd173e1dea775b4ac98a04c2fbe1950bfc09cf` |
| moment/dates | rust | 1 | `f3e38cf882a2e888910d1a64093d898e85116289de5bb172ea1ec9cd303e2142` | `135b4bb3c87854ea4ccb6e41e039b163fc17407895d691db1949e80469a56ea9` |
| moment/dates | rust | 2 | `db3c077fd8bdac6401c861747b26d495e16c8e4aaf8836ef4817bd66b874b219` | `0cccb2a729a246a7333fc5136beec8381fa1854dc6966c1b6a0f98c76bf61a37` |
| moment/dates | ng | 2 | `3be88db70f43f704b5ed3c9601efddc4281dce457f9ce9bd32afc8db06cf0fbc` | `18d5364023decb219d7bf63452261d30c344602b54bba6b838d44ab165a39d8b` |
| diff/compare | rust | 0 | `bf3e40f279bd1016dfdca8ed8684f05243a6da19a45cb1bceef8c39eedd97fe1` | `47982633b908395e91abf5182dba0282fcbe645342f663628b6b1ca2964c637f` |
| diff/compare | ng | 0 | `66b03e9b6a7596bf42598b9a7c8ee2a2363c52b7658b126ac3d7e1ed05890e64` | `5b3dfbfbdd70fc15c6ecc47ea81c5b9f5b7a89de50e29a71fb5687a7a9c78ff1` |
| diff/compare | ng | 1 | `469f2396202652210e091f787eada76a65f24b8cd17fe46dfd5d9c85df408e83` | `7fd9843e700e4ab2df622f708f63f60a961a8b7e7ef4f615bfdf78e6da7ddb6d` |
| diff/compare | rust | 1 | `3e291cec080fb714c22417a591cc8e5ca9c71848fa50566b6fe0f57dc89a0ce3` | `faf5d2aa2a7590d92f3d4aed98e8dfd2da10d98b64b5a612da78b5dc46e7aa33` |
| diff/compare | rust | 2 | `2c622de8647ae2935f18043ba3fdbc096abe7621a80e66e38363333f778f0e92` | `3f05ff6b366e4f9baa8f5a74fb1728b2a9d115431f8313723166459ae8d1589e` |
| diff/compare | ng | 2 | `f6beac1199b92b6f3d6f3c2c9d3b9be4733119ad9d8ab537af53a13a3153148b` | `06b3a8fe8411f5534e9b8d2e41cb1f3433669a4721dbcef4ed98940433f1ced8` |
| papaparse/csv | rust | 0 | `01cabc58ba4eb7bff2f3b59f36419fd24d099353b550bef06089543432bdbb8f` | `39df4d0d28354e836f3cfb60436afa989c6c2b1ebfa66dab35175ac3f0c05b2d` |
| papaparse/csv | ng | 0 | `7b46d2355b4b359e6fc7c4138a2fee6532b122edf978cefb2fcc489a89c00306` | `a1663ae05cf26e2b3ed4c72a198db462f162947df69a8a9188149bce52be7192` |
| papaparse/csv | ng | 1 | `0725fe02d07fe326974cd231041b8ad851190005c5cd6b728e9a72d79c6227c4` | `3906db7802222b0b7faa0e28474e969f98bfd559bb9be1df78a67f89d9ea7910` |
| papaparse/csv | rust | 1 | `9a8f75e40dd154aa40301dee05f3fc4a984c9660cebfc285962a3fe3a9c3c5e5` | `a1ba07ed1855e00dee8c3258a82ceb1d2b8d68b5d599f93bc28f23a819f07f9b` |
| papaparse/csv | rust | 2 | `c2d95eacb5c1762273687f36ccc91bda3721022f00e9f3acb4d1cd601f26e93b` | `18191531eb7e330af776c245d9d0d5759000d6b3b6bdf28a03d4f088c973d6ff` |
| papaparse/csv | ng | 2 | `0fe26ea987f9466caf4298f353cd349b730c52162dcde42958fd239efaa1bcab` | `3f27efa0c364f4ec9fbf9f5febd355771c1389553ec822152914bba2120fe283` |
| js-yaml/document | rust | 0 | `247627c2b2dc714e574801e316535af0b9bf86568ddc38d919a36cdfd523dcee` | `ab440765012695e104a110e8f6e2f17b6515f062c1268f022ea48f1427876f03` |
| js-yaml/document | ng | 0 | `42405765781a3b0357e0fa8be28e9dd7717c95a46e6463c520f689e5109fee82` | `7ab2350461f3f8789a646c9422f6b70a2b533a07fcbdb21a20c8077ae55e3f9d` |
| js-yaml/document | ng | 1 | `c3bfafa20d16b541afeae0d1df31669173c99510bc99e0e9ee6a949604b50868` | `4b4b9522c6f799f677bacea5c33ed75a4bf24d024e37f732ff59ff4cb6a4dd6b` |
| js-yaml/document | rust | 1 | `d278957f0976ee1501e72c5a3d9094ce754422117dab4cfe72fe31106e203a06` | `ef92268b3b9bf1ac1ec49736cc858f456ed94cfe7d9fc7e433a44645a3e982a0` |
| js-yaml/document | rust | 2 | `024f702e85b1e490927fbf1e5814fba0fbd339780ae72f618f634980aef4cd3a` | `37f132692184ae51dd7b4fe9f3ae25b891a8c93daa5f8727963b42d4c3c1a885` |
| js-yaml/document | ng | 2 | `7b73a23cb3af2b4173720f944dadc1e223082de1a1c223d24a42281ff4de8fa3` | `ca753ca839799f3740d11ef750fa6fed830ed98d4fef83708975566bfbd0b13f` |
| big.js/arith | rust | 0 | `554b3fe7b1dbf8b09bb13cab4bf2010cd4be5262c5a3435e7ce70732b05f7d71` | `b599007a2284c5c084c9a501d1bd45d007c686ecb949c2a22ff7188ae38695b9` |
| big.js/arith | ng | 0 | `161ac2ccc9bec5e9f807895b16cccc8af3422f1dea4fefdc5e1f01e57a66c437` | `4a8347ab755b3893ef80688dbc6d13605c582b4977831e597041294e7920ff68` |
| big.js/arith | ng | 1 | `99741d98d6a4ad3aa1919ae19872ed6aa4ddd1884b6750b35a93da185e78e586` | `01ecc5197d3dd1ab8a1b011843ed26a0fa7883d4d00e784c6283e93d5e03c15c` |
| big.js/arith | rust | 1 | `a1f3b6f2637689186d8c84d492c47fbbbda21ba2bb2cac9870a7de76745f0337` | `8bf54a16363f72c45d558d41ee130093edbab66527dc39e05cc4dddd2015a5be` |
| big.js/arith | rust | 2 | `518288ace80aef0f03e546a08165f8f392c3c6bb45d7ee9e0a70a33be29f7f05` | `2f68975ebabebea42650650689608cb4a9171a82cf7b53105b080fc7bfe94f42` |
| big.js/arith | ng | 2 | `4f2b7b8bfbeda4e4b8f56d0936d03a54649d2138ca3df70693f0f7099e3000ad` | `0056b52b90004c3b62cf341a14a72b60f6b52d363660ed964be1f7fe671a6182` |
| matter-js/simulate | rust | 0 | `eef7de98e652f50b18223c3758d6c858420ca4b1d3ad3452802e511e984a46e7` | `75c043cafeef355c6d4811749c7f0760e79eae2822e39ade44282f25cf37c0ea` |
| matter-js/simulate | ng | 0 | `856239f4491af70b76983ef3d30baf3653b1765f148719fcaec0698c502fd671` | `c818fcd614d8116f40e6b5a8289f2cfbacb186562d5d61b04ca06f38d8d86827` |
| matter-js/simulate | ng | 1 | `f092ff116c74c93b24276bef25148f243045e4982a8e6affe65d440d6db7f9be` | `7741997dd4048c1d9cf1b91e18a726578db6dca5e4cc9e43b1ed7210485bcd52` |
| matter-js/simulate | rust | 1 | `d094b1218d2562acb6cf7f38ed139821b6e5ffc7a78ed8f943c6f9a3621b8ab1` | `2713ba03a021a22243783132b4c0eb83897decca1ac6f66386893bd2a067d792` |
| matter-js/simulate | rust | 2 | `555aa871686e8fd4d18008f353e56bc5dcd7a9df49554478cd5f862ceab55d43` | `f06eab208c7103541912e8ef019c6745ed4d9c3ee276dc0274966c2221b47010` |
| matter-js/simulate | ng | 2 | `6f0400cc376a98f9548dce13d768b31149f83da133082ca5c423e611357e6150` | `8dfc7d9b298836ab2523b0839bdaf347223f2a9f9af8a2e7a2dc081337c5dd17` |
| graphlib/paths | rust | 0 | `1321169ab5900c2b7432fd6fb25ace49f7be200af35010b4077e3176e1487f50` | `75f1acf2aa436671e59b4efc90d36981a4a99bb746e9d19944502c4bbc07a5de` |
| graphlib/paths | ng | 0 | `103ac7b1f49f340e441f773352f5520bb852a09439cb913ac5567b5eea0bb91a` | `8154f4c9f463070eeafcfae8078e7ef513371b10e15732705647cf41ca66b278` |
| graphlib/paths | ng | 1 | `77e490a71b69be42022c8660dd76e66d1e919552bd23163bac93faf0e126fbb7` | `3176a91be1ad5798932f97109bb1394f1105dde17bf74cee89709031b4a4a619` |
| graphlib/paths | rust | 1 | `fc8b74754ef8f9a61b3768dead4c3e3a720b3a4cb96a9bcc054deecb2b8b6f12` | `abec1b6d31a22147d576e6cf3cf7537f3d5a3430a36b7c8c3b102fb12198bad8` |
| graphlib/paths | rust | 2 | `20a5f9c94e0e257d4991a88f6993f13059e2244c6372ae9f672141a8242240f3` | `b12bdfd4af693bbad902517d2dbaadc511882d818b1c3264489a3a872009b38e` |
| graphlib/paths | ng | 2 | `54a2b7d3e4bb58540eb722cac791b87e881853b815d3c5b1790be5ed4bf90649` | `fbf8ab6f7aa675f987e4abd7708da158ed81d0586ed9e692655ca94e34baae4d` |
