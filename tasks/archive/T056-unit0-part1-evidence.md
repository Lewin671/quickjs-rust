# T056 unit 0 part 1: evidence and an unimplemented M1 draft

The requested available measurements and verification are recorded below.
Unit 0's exact-attribution prerequisite remains open: the existing diagnostic
feature lacks required event counters and the brief prohibits runtime edits.
M1 is not implemented, registered or frozen. The draft passes mechanical
queue/host/image/profile validation, which does not supply missing evidence.

Selection uses only the canonical queue and exact-image profiles. No held-out
program was profiled, counted for mechanism selection or selected as a target.
The independent development lane is part 2, outside this collection.


Pinned source: `b481894d351d933e94717a9c295c15fe4a466804`.

## Binary and artifact identity

| Role | Source | Executable SHA-256 |
| --- | --- | --- |
| base | `9a0eeeca3521914d5743b10c77c7ecca6a856018` | `926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85` |
| candidate | `b481894d351d933e94717a9c295c15fe4a466804` | `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e` |
| quickjs-ng | `f7830186043e4488f2998759d60a514faf07cbc9` | `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40` |

| Collection | Queue job |
| --- | --- |
| baseline | `20261017T061057Z-t056-unit0-baseline30` |
| held-out | `20261017T061206Z-t056-unit0-heldout30` |
| attribution | `20261017T061409Z-t056-unit0-attribution` |
| acceptance-rss | `20261017T061702Z-t056-unit0-acceptance-rss-static` |
| v2-replay | `20261017T063820Z-t056-unit0-v2-replay` |
| counter-increments | `20261017T065255Z-t056-unit0-counter-increments` |
| full-check | `20261017T064825Z-t056-unit0-full-check-fixed` |

Baseline schedule: `workers-4`; decision readiness: `inconclusive`.
Measurement issues: `[]`; imprecise cases: `['external/sunspider-1.0/bitops-3bit-bits-in-byte']`.

Queue SHA-256: `0f4a1fda656257bccd614b0914796e855752ec3441de4e23f3e95f318c834570`.

Raw files are local queue artifacts under workspace `outputs/imac-bench/<job>/out/`. The results, counts, budgets and hashes recorded here are the durable evidence.

## Method and limits

Pinned source is `b481894d351d933e94717a9c295c15fe4a466804`, the brief's main.
All engine measurement and macOS sampling jobs use the workspace iMac queue.
The local arm64 run is an ABI/ownership probe only, with no benchmark timings.
No runtime, build recipe, order file, manifest or benchmark source is changed.
The comparison base is `9a0eeeca3521914d5743b10c77c7ecca6a856018`, an earlier
commit as prescribed by the performance workflow; this is not a same-source A/A.
Its differences include the RegExp validation fix and the subsequently recorded
x86 order, so base/candidate differences alone do not attribute an engine cost.
Standing and ranking use the current candidate versus pinned QuickJS-NG.

The formal comparison uses 30 blocks, three engines and the iMac-qualified
four-worker schedule. Formal multi-worker disturbance invalidates the whole
job; no pair replacement or pooling is applied to it. Layout/held-out panel
pair replacement and the precision continuation rule apply only to their
qualified collectors. A/A continuation is never forged by relabeling these
different source revisions. The adopted v2 artifact has no cap-imprecise case. Its standing rule would
require K=6 for a diagnostic-only case; this draft independently predeclares
K=6 because of the structural change and the earlier layout failures. Power allocation, where applicable, remains
`min(240, ceil(30*(h/.03)^2))`, retaining the original observations.

The held-out run is a separate 30-block standing/acceptance lane. Its programs
are never profiled, counted for mechanism selection, ranked or plan targets.
Its RSS collection is acceptance-only. The profiler selection is frozen in the
queued collector before reading held-out results: all broad/sentinel cases,
the top three entries in each selection lane, and the named property/call/value
programs discussed by T056 and its input critique. Extra broad cases are coverage,
not newly selected targets.

Diagnostic counters come from the same engine sources with `agents,perf-counters`;
its build identity is separate and none of its timings enter a performance table.
The primary counter/RSS runs use N=10,000. The primary table retains full-process
events including setup. A supplementary internal run at N=20,000 supplies
N-versus-2N event increments that remove fixed setup; neither run supplies timings. Per-operation columns divide by
the workload's reported nominal operations, and external cases retain a
whole-program denominator. Counts do not measure latency. The uninstrumented
sampler uses the exact queue executable, original external bundles, and the
original internal drivers with N=3,000,000. Sample stdout is checked. Main-thread
sample trees conserve their root sample count; exclusive/self counts and union
inclusive envelopes are separate. Inclusive envelopes overlap and must not be
added or treated as removable overhead. A fast process that produces no usable
call graph is reported as missing, never as zero cost.

Peak RSS uses the repository's direct-child `wait4` collector, macOS byte units,
three fresh processes for each case/engine. Both engines' stdout contracts are
checked. These N=10,000 internal resource baselines cannot be substituted for
the formal harness's calibrated N, or for a different future workload size.
They are a reproducible M3 baseline, not an allocation/free count. A future
memory gate must retain these same inputs and sizes and compare fresh paired
processes; startup, code image and allocator pages are included.

### Counter coverage checked against source

| Requested quantity | Available evidence | Limit |
| --- | --- | --- |
| Vm constructions / JS call | `nested_vm_constructions`, `ordinary_call_attempts`, call-tier counters | The construction site also counts the root script Vm; the raw ratio is not a pure callee rate. Native calls are separate. |
| Ordinary versus accelerated execution | `executed_ops`, opcode families, compact operations/activations, loop entries/declines | There is no accelerator-off switch. Natural declines and traces are diagnostic evidence; counters do not assign elapsed time. |
| Named/computed accesses | VM named/computed counters | Compact/typed-loop reads can bypass these VM counting sites. Zero VM named reads alone does not establish zero name lookups. |
| Whole-frame transfers | Dormant `push_frame`, selected linked disassembly, memory-copy sample envelope | No dynamic emitted-copy counter; a source move or a sampled memmove is not a count of whole FrameState copies. |
| Name comparisons/hashes per named access | Storage source and sampled string/hash symbols | No exact scoped counter exists; hashes also serve realm/binding tables. |
| IC probes/rejections | Cache source; `QJS_IC_TRACE` reports updates only | Update events are not probe/reject counts. Historical temporary counters are not present at this revision. |
| Value clones/drops | Value ownership symbols in uninstrumented samples | No Value event counters. Inlined/eliminated glue is not separately visible; sample counts are not event counts. |
| Heap allocations/frees | Allocation/free sample envelope and process peak RSS | Neither supplies event totals; no allocator event counter exists. |
| Actual lowered opcode histogram | Dispatch families and short CF/TL traces | CFVMCODE prints original bytecode; families do not uniquely count virtual field operations. |

The absent event counts are unavailable, not zero. Adding them would change
runtime/CLI diagnostics, which this brief explicitly prohibits. This unit retains no runtime diagnostic modifications. Resolve that boundary
in a follow-up instrumentation unit. Without the missing event evidence and
part 2's independent development programs, an evidence-bound JSON draft is not
authorization to start M1 runtime work.

### Source conclusions that do not depend on a timing result

`Vm::with_frame_bytecode` is the construction counter's only site. Ordinary
calls still use nested Vms. Production code never sets `pending_frame_entry`
to Some, so the dormant driver's `mem::replace` path does not price current
ordinary calls. The July 704-byte copy and its removal are history, not the
current frame size or cost. `eval_function_bytecode` already destructures its
four returned fields directly; emitted constructor copies need separate audit.

Small, Dynamic, Shaped and ShapedPair still retain string names; Dynamic has a
name-to-slot hash index. The IC chain is not one layout authority. Conversely,
`LoadVirtualValue` reads the frame's `virtual_values` bank directly. The broad
literal-field deficit cannot be assigned to atoms/ICs without demonstrating
actual named lookup. The complete family counters, traces and samples below
separate that issue from ordinary receiver access.


## Baseline standing

All standing ratios/CIs are raw-replayed with v2 case-median paired ratios and joint block bootstrap (20,000 draws, seed 0); lower is faster. The queue below retains the official queue statistics and its original hash. Internal medians use calibrated operation increments; external/held-out medians are whole-process wall time. The paired ratio need not equal the ratio of the two displayed marginal medians. No cross-lane headline score is calculated.

| Lane / suite | Cases | Rust/NG case-geomean |
| --- | ---: | ---: |
| broad | 25 | 0.987920 |
| sentinel | 6 | 1.055405 |
| external | 45 | 0.990027 |
| jetstream3-js-subset | 5 | 1.155542 |
| kraken-1.1 | 14 | 0.779318 |
| sunspider-1.0 | 26 | 1.093200 |
| held-out | 9 | 3.010448 |

| Case | Unit | Rust median | NG median | Paired Rust/NG | 95% CI |
| --- | --- | ---: | ---: | ---: | --- |
| `broad/array_allocation` | ns/op | 83.3415 | 308.3351 | 0.270417 | [0.270268, 0.271032] |
| `broad/array_dynamic_read` | ns/op | 7.0170 | 15.9009 | 0.441474 | [0.440198, 0.442865] |
| `broad/array_index_of` | ns/op | 96.9268 | 67.9056 | 1.427334 | [1.424330, 1.429711] |
| `broad/array_read` | ns/op | 54.8364 | 16.8563 | 3.256710 | [3.244896, 3.276448] |
| `broad/array_write` | ns/op | 73.0800 | 48.4300 | 1.510102 | [1.504155, 1.519354] |
| `broad/branch_arithmetic` | ns/op | 19.3225 | 19.1349 | 1.010638 | [1.007660, 1.012333] |
| `broad/captured_read` | ns/op | 37.9306 | 49.7969 | 0.760252 | [0.758674, 0.762484] |
| `broad/captured_write` | ns/op | 110.3410 | 49.6714 | 2.218116 | [2.212831, 2.230842] |
| `broad/closure_allocation_call` | ns/op | 139.6278 | 526.1083 | 0.264141 | [0.262647, 0.266428] |
| `broad/dynamic_method_call` | ns/op | 209.3894 | 70.6255 | 2.967320 | [2.943119, 2.986250] |
| `broad/empty_loop` | ns/op | 6.8025 | 8.2965 | 0.820838 | [0.817671, 0.825092] |
| `broad/function_call_reordered` | ns/op | 36.6225 | 49.3241 | 0.741556 | [0.739592, 0.744991] |
| `broad/function_call_two_args` | ns/op | 37.3793 | 50.7158 | 0.736816 | [0.733918, 0.739176] |
| `broad/global_read` | ns/op | 9.2050 | 11.6578 | 0.788696 | [0.786278, 0.791026] |
| `broad/local_read` | ns/op | 5.9958 | 7.4403 | 0.805620 | [0.802266, 0.808749] |
| `broad/many_locals_call` | ns/op | 76.1881 | 190.0486 | 0.401296 | [0.399759, 0.403225] |
| `broad/math_abs` | ns/op | 25.0893 | 43.4521 | 0.577814 | [0.573925, 0.581429] |
| `broad/method_call` | ns/op | 52.4080 | 53.4887 | 0.984702 | [0.978888, 0.986518] |
| `broad/object_allocation` | ns/op | 82.0530 | 345.1601 | 0.237132 | [0.235953, 0.238925] |
| `broad/plain_function_call` | ns/op | 36.4292 | 49.3605 | 0.738702 | [0.734991, 0.741324] |
| `broad/property_dynamic_read` | ns/op | 45.5720 | 33.3886 | 1.364275 | [1.354702, 1.377026] |
| `broad/property_read` | ns/op | 60.0181 | 11.6374 | 5.159322 | [5.138408, 5.167103] |
| `broad/property_write` | ns/op | 87.8054 | 23.0735 | 3.804678 | [3.795621, 3.826056] |
| `broad/string_slice` | ns/op | 227.7922 | 151.0548 | 1.511827 | [1.489012, 1.526202] |
| `broad/top_level_function_call` | ns/op | 78.0371 | 54.6869 | 1.428650 | [1.418904, 1.436609] |
| `sentinel/capturing_closure_call` | ns/op | 65.3113 | 62.7851 | 1.040240 | [1.037489, 1.042301] |
| `sentinel/heterogeneous_property_read` | ns/op | 33.6354 | 23.4370 | 1.434949 | [1.422274, 1.442277] |
| `sentinel/polymorphic_call_site` | ns/op | 63.7291 | 63.5730 | 1.004506 | [0.999326, 1.007717] |
| `sentinel/prototype_method_call` | ns/op | 100.2251 | 82.1256 | 1.221704 | [1.217543, 1.224268] |
| `sentinel/recursive_call_tree` | ns/op | 31.6937 | 47.3118 | 0.671831 | [0.666247, 0.672892] |
| `sentinel/string_key_map_churn` | ns/op | 62.2935 | 55.4355 | 1.122966 | [1.113675, 1.128261] |
| `external/jetstream3-js-subset/cdjs` | ms/process | 1062.5571 | 775.0901 | 1.369875 | [1.365272, 1.376251] |
| `external/jetstream3-js-subset/hash-map` | ms/process | 616.8543 | 416.7230 | 1.478586 | [1.469795, 1.484779] |
| `external/jetstream3-js-subset/gaussian-blur` | ms/process | 509.4708 | 847.7011 | 0.603316 | [0.600586, 0.604610] |
| `external/jetstream3-js-subset/stanford-crypto-aes` | ms/process | 410.9777 | 259.0379 | 1.583424 | [1.576298, 1.591925] |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | ms/process | 878.5068 | 823.3557 | 1.064776 | [1.061028, 1.071730] |
| `external/kraken-1.1/ai-astar` | ms/process | 3285.9933 | 2754.3018 | 1.198000 | [1.187491, 1.201787] |
| `external/kraken-1.1/audio-beat-detection` | ms/process | 1304.7026 | 1669.0637 | 0.781415 | [0.778567, 0.784592] |
| `external/kraken-1.1/audio-dft` | ms/process | 746.9878 | 1375.0019 | 0.544172 | [0.539145, 0.547193] |
| `external/kraken-1.1/audio-fft` | ms/process | 875.3369 | 1560.8042 | 0.560346 | [0.559639, 0.561269] |
| `external/kraken-1.1/audio-oscillator` | ms/process | 544.6588 | 1305.0925 | 0.417560 | [0.415271, 0.419308] |
| `external/kraken-1.1/imaging-gaussian-blur` | ms/process | 11248.7974 | 10233.8836 | 1.097877 | [1.094101, 1.102884] |
| `external/kraken-1.1/imaging-darkroom` | ms/process | 1392.1489 | 1647.3407 | 0.841010 | [0.838747, 0.849765] |
| `external/kraken-1.1/imaging-desaturate` | ms/process | 1798.5371 | 3206.7729 | 0.558467 | [0.556451, 0.561932] |
| `external/kraken-1.1/json-parse-financial` | ms/process | 70.6990 | 179.1992 | 0.395079 | [0.392607, 0.397264] |
| `external/kraken-1.1/json-stringify-tinderbox` | ms/process | 76.0417 | 173.7388 | 0.437064 | [0.435242, 0.437656] |
| `external/kraken-1.1/stanford-crypto-aes` | ms/process | 1032.9708 | 655.0172 | 1.585967 | [1.575469, 1.593352] |
| `external/kraken-1.1/stanford-crypto-ccm` | ms/process | 695.2878 | 510.1420 | 1.362583 | [1.356060, 1.364679] |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | ms/process | 1451.0250 | 1335.3988 | 1.083770 | [1.080022, 1.090359] |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | ms/process | 482.0187 | 393.3370 | 1.226252 | [1.222339, 1.229733] |
| `external/sunspider-1.0/3d-cube` | ms/process | 40.6273 | 41.3519 | 0.980599 | [0.976443, 0.986709] |
| `external/sunspider-1.0/3d-morph` | ms/process | 21.9678 | 29.5514 | 0.743226 | [0.739179, 0.747459] |
| `external/sunspider-1.0/3d-raytrace` | ms/process | 51.1581 | 30.5490 | 1.677041 | [1.665135, 1.681137] |
| `external/sunspider-1.0/access-binary-trees` | ms/process | 36.4282 | 31.4916 | 1.159503 | [1.154795, 1.165609] |
| `external/sunspider-1.0/access-fannkuch` | ms/process | 63.5851 | 75.4032 | 0.842980 | [0.840593, 0.845948] |
| `external/sunspider-1.0/access-nbody` | ms/process | 31.0879 | 19.5084 | 1.593093 | [1.581368, 1.612048] |
| `external/sunspider-1.0/access-nsieve` | ms/process | 25.7934 | 40.3583 | 0.637664 | [0.633648, 0.639740] |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | ms/process | 16.9579 | 13.3031 | 1.262002 | [1.176502, 1.282594] |
| `external/sunspider-1.0/bitops-bits-in-byte` | ms/process | 22.7110 | 23.7183 | 0.957601 | [0.951226, 0.966076] |
| `external/sunspider-1.0/bitops-bitwise-and` | ms/process | 13.7792 | 22.8524 | 0.602486 | [0.598582, 0.606658] |
| `external/sunspider-1.0/bitops-nsieve-bits` | ms/process | 33.8731 | 24.7360 | 1.368715 | [1.363545, 1.372879] |
| `external/sunspider-1.0/controlflow-recursive` | ms/process | 10.9249 | 11.9237 | 0.917521 | [0.906815, 0.925288] |
| `external/sunspider-1.0/crypto-aes` | ms/process | 60.5383 | 47.8686 | 1.260953 | [1.254495, 1.265218] |
| `external/sunspider-1.0/crypto-md5` | ms/process | 20.4525 | 12.4888 | 1.628833 | [1.621319, 1.647843] |
| `external/sunspider-1.0/crypto-sha1` | ms/process | 18.5446 | 13.1991 | 1.405683 | [1.395534, 1.420897] |
| `external/sunspider-1.0/date-format-tofte` | ms/process | 132.1586 | 81.2493 | 1.629851 | [1.620321, 1.644716] |
| `external/sunspider-1.0/date-format-xparb` | ms/process | 47.7178 | 37.2574 | 1.278900 | [1.268464, 1.287907] |
| `external/sunspider-1.0/math-cordic` | ms/process | 40.9676 | 37.5490 | 1.091537 | [1.085565, 1.094082] |
| `external/sunspider-1.0/math-partial-sums` | ms/process | 21.7570 | 17.7607 | 1.228971 | [1.217414, 1.240124] |
| `external/sunspider-1.0/math-spectral-norm` | ms/process | 14.5685 | 13.5193 | 1.079736 | [1.074319, 1.089905] |
| `external/sunspider-1.0/regexp-dna` | ms/process | 167.3257 | 133.5839 | 1.253093 | [1.250639, 1.267219] |
| `external/sunspider-1.0/string-base64` | ms/process | 36.3861 | 34.7244 | 1.051458 | [1.040616, 1.054352] |
| `external/sunspider-1.0/string-fasta` | ms/process | 105.9568 | 121.5122 | 0.873277 | [0.868125, 0.881806] |
| `external/sunspider-1.0/string-tagcloud` | ms/process | 79.0005 | 74.5958 | 1.059833 | [1.051372, 1.064335] |
| `external/sunspider-1.0/string-unpack-code` | ms/process | 105.2403 | 154.3184 | 0.683002 | [0.678549, 0.688544] |
| `external/sunspider-1.0/string-validate-input` | ms/process | 42.3006 | 33.6301 | 1.259086 | [1.250796, 1.266975] |
| `held-out/acorn/parse` | ms/process | 1243.6081 | 1541.3414 | 0.806906 | [0.804656, 0.809505] |
| `held-out/esprima/parse` | ms/process | 7546.6225 | 2518.7116 | 2.997423 | [2.983703, 3.004965] |
| `held-out/marked/render` | ms/process | 315.8989 | 349.1754 | 0.901961 | [0.900109, 0.905749] |
| `held-out/mustache/render` | ms/process | 5399.1667 | 1587.9431 | 3.399327 | [3.371520, 3.413166] |
| `held-out/handlebars/render` | ms/process | 8623.2074 | 1219.7805 | 7.073085 | [7.036092, 7.088765] |
| `held-out/crypto-js/hash` | ms/process | 6423.6169 | 2523.2146 | 2.551662 | [2.543137, 2.561258] |
| `held-out/d3-array/aggregate` | ms/process | 7643.7944 | 496.4078 | 15.365897 | [15.308674, 15.433953] |
| `held-out/ramda/pipeline` | ms/process | 8784.0309 | 2193.6371 | 4.007939 | [3.993036, 4.030029] |
| `held-out/decimal.js/arith` | ms/process | 5793.7589 | 2347.1326 | 2.463872 | [2.458634, 2.476760] |

## Opportunity queue

Selection lanes only; held-out is absent.

| Lane | Rank | Case | Rust/NG |
| --- | ---: | --- | ---: |
| broad | 1 | `broad/property_read` | 5.159322 |
| broad | 2 | `broad/property_write` | 3.804677 |
| broad | 3 | `broad/array_read` | 3.256710 |
| broad | 4 | `broad/dynamic_method_call` | 2.967319 |
| broad | 5 | `broad/captured_write` | 2.218116 |
| broad | 6 | `broad/string_slice` | 1.511826 |
| broad | 7 | `broad/array_write` | 1.510102 |
| broad | 8 | `broad/top_level_function_call` | 1.428647 |
| sentinel | 1 | `sentinel/heterogeneous_property_read` | 1.434949 |
| sentinel | 2 | `sentinel/prototype_method_call` | 1.221704 |
| sentinel | 3 | `sentinel/string_key_map_churn` | 1.122965 |
| sentinel | 4 | `sentinel/capturing_closure_call` | 1.040240 |
| sentinel | 5 | `sentinel/polymorphic_call_site` | 1.004506 |
| sentinel | 6 | `sentinel/recursive_call_tree` | 0.671831 |
| external | 1 | `external/sunspider-1.0/3d-raytrace` | 1.677041 |
| external | 2 | `external/sunspider-1.0/date-format-tofte` | 1.629850 |
| external | 3 | `external/sunspider-1.0/crypto-md5` | 1.628831 |
| external | 4 | `external/sunspider-1.0/access-nbody` | 1.593093 |
| external | 5 | `external/kraken-1.1/stanford-crypto-aes` | 1.585967 |
| external | 6 | `external/jetstream3-js-subset/stanford-crypto-aes` | 1.583424 |
| external | 7 | `external/jetstream3-js-subset/hash-map` | 1.478586 |
| external | 8 | `external/sunspider-1.0/crypto-sha1` | 1.405679 |

## Execution counters

Diagnostic image SHA-256: `7a81240fed7c3593e132035451b3fc95a4cd9e1db167b329745b76fc700bb0b2`. The production image is never replaced by this build. All columns are full-execution raw counts. `Vm/call` is root-inclusive raw Vm constructions divided by the existing JS call-attempt counter; it is not an inferred pure callee rate. `ops/nominal` includes setup; external cases have no declared per-operation denominator.

| Case | JS call attempts | Vm constructions | Vm/call | VM ops | ops/nominal | VM named R/W | VM computed R/W | Compact ops | Loop entries / declines |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `broad/plain_function_call` | 10003 | 1 | 0.000100 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 56 | 1 / 0 |
| `broad/method_call` | 10004 | 2 | 0.000200 | 308 | 0.030800 | 10 / 0 | 4 / 0 | 43 | 1 / 0 |
| `broad/captured_read` | 10005 | 2 | 0.000200 | 276 | 0.027600 | 8 / 0 | 4 / 0 | 74 | 1 / 0 |
| `broad/captured_write` | 10005 | 3 | 0.000300 | 276 | 0.027600 | 8 / 0 | 4 / 0 | 140069 | 0 / 1 |
| `broad/many_locals_call` | 10003 | 1 | 0.000100 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 80 | 1 / 0 |
| `broad/property_read` | 4 | 2 | 0.500000 | 120290 | 4.009667 | 8 / 0 | 4 / 0 | 67 | 0 / 10000 |
| `broad/array_read` | 4 | 2 | 0.500000 | 150290 | 3.757250 | 8 / 0 | 4 / 0 | 73 | 0 / 10000 |
| `broad/function_call_two_args` | 10003 | 1 | 0.000100 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 99 | 1 / 0 |
| `broad/function_call_reordered` | 10003 | 1 | 0.000100 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 104 | 1 / 0 |
| `broad/top_level_function_call` | 10002 | 1 | 0.000100 | 307 | 0.030700 | 8 / 0 | 4 / 0 | 28 | 1 / 0 |
| `broad/dynamic_method_call` | 10004 | 2 | 0.000200 | 275293 | 27.529300 | 20008 / 0 | 4 / 0 | 91 | 0 / 10000 |
| `broad/local_read` | 4 | 1 | 0.250000 | 271 | 0.013550 | 8 / 0 | 4 / 0 | 119 | 1 / 0 |
| `broad/global_read` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 120 | 1 / 0 |
| `broad/property_dynamic_read` | 4 | 2 | 0.500000 | 271 | 0.009033 | 8 / 0 | 4 / 0 | 110129 | 0 / 1 |
| `broad/property_write` | 4 | 2 | 0.500000 | 240288 | 8.009600 | 8 / 0 | 4 / 0 | 115 | 1 / 9999 |
| `broad/array_dynamic_read` | 4 | 2 | 0.500000 | 271 | 0.006775 | 8 / 0 | 4 / 0 | 154 | 1 / 0 |
| `broad/array_write` | 4 | 2 | 0.500000 | 280290 | 7.007250 | 8 / 0 | 4 / 0 | 127 | 0 / 10000 |
| `broad/empty_loop` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 150 | 1 / 0 |
| `broad/branch_arithmetic` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 163 | 1 / 0 |
| `broad/math_abs` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 166 | 1 / 0 |
| `broad/array_index_of` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 177 | 1 / 0 |
| `broad/string_slice` | 4 | 1 | 0.250000 | 271 | 0.027100 | 8 / 0 | 4 / 0 | 180 | 1 / 0 |
| `broad/object_allocation` | 4 | 2 | 0.500000 | 40287 | 4.028700 | 8 / 0 | 4 / 0 | 163 | 0 / 10000 |
| `broad/array_allocation` | 4 | 2 | 0.500000 | 40287 | 4.028700 | 8 / 0 | 4 / 0 | 169 | 0 / 10000 |
| `broad/closure_allocation_call` | 10004 | 2 | 0.000200 | 90287 | 9.028700 | 8 / 0 | 4 / 0 | 175 | 0 / 10000 |
| `sentinel/recursive_call_tree` | 4 | 1 | 0.250000 | 192 | 0.000151 | 10 / 2 | 4 / 0 | 58 | 1 / 0 |
| `sentinel/prototype_method_call` | 10069 | 2 | 0.000199 | 192 | 0.019200 | 10 / 2 | 4 / 0 | 1167 | 1 / 1 |
| `sentinel/polymorphic_call_site` | 10003 | 1 | 0.000100 | 192 | 0.019200 | 10 / 2 | 4 / 0 | 77 | 1 / 0 |
| `sentinel/capturing_closure_call` | 10008 | 5 | 0.000500 | 204 | 0.020400 | 10 / 2 | 4 / 0 | 96 | 1 / 0 |
| `sentinel/heterogeneous_property_read` | 5 | 2 | 0.400000 | 192 | 0.006400 | 10 / 2 | 4 / 0 | 1527 | 1 / 1 |
| `sentinel/string_key_map_churn` | 5 | 3 | 0.600000 | 204 | 0.010200 | 10 / 2 | 4 / 0 | 31857 | 2 / 2 |
| `external/jetstream3-js-subset/cdjs` | 2293349 | 18046 | 0.007869 | 2187986 | n/a | 496456 / 96 | 100040 / 0 | 48853125 | 133 / 50026 |
| `external/jetstream3-js-subset/hash-map` | 2340047 | 9 | 0.000004 | 409 | n/a | 24 / 26 | 0 / 0 | 42300893 | 31 / 8 |
| `external/jetstream3-js-subset/gaussian-blur` | 360007 | 5 | 0.000014 | 9686059 | n/a | 720012 / 10 | 54960 / 732500 | 376 | 2500 / 361251 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 107179 | 25205 | 0.235167 | 52359 | n/a | 382 / 62 | 4 / 148 | 4260289 | 50558 / 314 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 2013734 | 249 | 0.000124 | 3337 | n/a | 246 / 872 | 0 / 0 | 37221078 | 0 / 3 |
| `external/kraken-1.1/ai-astar` | 63313 | 104 | 0.001643 | 580697 | n/a | 80434 / 1614 | 162588 / 20000 | 5894383 | 31264 / 10203 |
| `external/kraken-1.1/audio-beat-detection` | 8297 | 13005 | 1.567434 | 22536205 | n/a | 5412850 / 117902 | 2461662 / 907360 | 6119940 | 67928 / 160899 |
| `external/kraken-1.1/audio-dft` | 103 | 10264 | 99.650485 | 15578 | n/a | 92 / 154 | 0 / 0 | 443062 | 10281 / 2 |
| `external/kraken-1.1/audio-fft` | 5003 | 12004 | 2.399360 | 15578 | n/a | 92 / 154 | 0 / 0 | 1011907 | 13004 / 2 |
| `external/kraken-1.1/audio-oscillator` | 4552 | 2005 | 0.440466 | 146120 | n/a | 30606 / 12180 | 6100 / 4098 | 94343 | 1500 / 2049 |
| `external/kraken-1.1/imaging-gaussian-blur` | 2 | 1 | 0.500000 | 427271 | n/a | 0 / 0 | 0 / 0 | 4616 | 3 / 0 |
| `external/kraken-1.1/imaging-darkroom` | 22 | 1 | 0.045455 | 428158 | n/a | 38 / 0 | 36 / 0 | 1147 | 18 / 18 |
| `external/kraken-1.1/imaging-desaturate` | 200 | 201 | 1.005000 | 430280 | n/a | 1202 / 4 | 0 / 0 | 14800 | 200 / 200 |
| `external/kraken-1.1/json-parse-financial` | 0 | 1 | n/a | 20254 | n/a | 2000 / 0 | 0 / 0 | 0 | 0 / 1000 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0 | 1 | n/a | 27384 | n/a | 40 / 0 | 0 / 0 | 0 | 0 / 20 |
| `external/kraken-1.1/stanford-crypto-aes` | 315946 | 66508 | 0.210504 | 48471 | n/a | 412 / 60 | 10 / 148 | 11311641 | 133150 / 314 |
| `external/kraken-1.1/stanford-crypto-ccm` | 221675 | 51898 | 0.234118 | 455169 | n/a | 126200 / 60 | 2026 / 156 | 12758882 | 105447 / 1315 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 377143 | 16406 | 0.043501 | 16393 | n/a | 630 / 90 | 554 / 658 | 11033884 | 82111 / 437 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 32811 | 39 | 0.001189 | 65059 | n/a | 18428 / 58 | 8 / 144 | 1525204 | 12664 / 1314 |
| `external/sunspider-1.0/3d-cube` | 13132 | 63 | 0.004797 | 314 | n/a | 0 / 4 | 0 / 0 | 535843 | 5078 / 10 |
| `external/sunspider-1.0/3d-morph` | 15 | 1 | 0.066667 | 473 | n/a | 0 / 0 | 4 / 2 | 426 | 18 / 15 |
| `external/sunspider-1.0/3d-raytrace` | 56628 | 9 | 0.000159 | 22914 | n/a | 28 / 24 | 3698 / 164 | 3557887 | 33 / 965 |
| `external/sunspider-1.0/access-binary-trees` | 126216 | 1 | 0.000008 | 17092 | n/a | 1626 / 2 | 0 / 0 | 1296152 | 0 / 412 |
| `external/sunspider-1.0/access-fannkuch` | 1 | 78 | 78.000000 | 378 | n/a | 58 / 0 | 0 / 0 | 4738 | 106 / 30 |
| `external/sunspider-1.0/access-nbody` | 4560 | 2 | 0.000439 | 225 | n/a | 6 / 6 | 0 / 0 | 118257 | 9015 / 5 |
| `external/sunspider-1.0/access-nsieve` | 4 | 2 | 0.500000 | 65 | n/a | 0 / 0 | 0 / 0 | 168 | 7 / 1 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 128000 | 1 | 0.000008 | 68 | n/a | 0 / 0 | 0 / 0 | 23 | 2 / 0 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 1 | 1 | 1.000000 | 64 | n/a | 0 / 0 | 0 / 0 | 23 | 2 / 0 |
| `external/sunspider-1.0/bitops-bitwise-and` | 0 | 1 | n/a | 75 | n/a | 0 / 0 | 0 / 0 | 0 | 1 / 0 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 2 | 4 | 2.000000 | 655104 | n/a | 2 / 0 | 15674 / 15672 | 102 | 22522 / 0 |
| `external/sunspider-1.0/controlflow-recursive` | 9 | 1 | 0.111111 | 260 | n/a | 0 / 0 | 0 / 0 | 0 | 0 / 3 |
| `external/sunspider-1.0/crypto-aes` | 10174 | 261 | 0.025654 | 2357 | n/a | 340 / 0 | 0 / 0 | 976747 | 17359 / 9 |
| `external/sunspider-1.0/crypto-md5` | 11998 | 2 | 0.000167 | 275 | n/a | 0 / 0 | 0 / 0 | 361885 | 1 / 5 |
| `external/sunspider-1.0/crypto-sha1` | 86735 | 2 | 0.000023 | 257 | n/a | 0 / 0 | 0 / 0 | 1086 | 3 / 5 |
| `external/sunspider-1.0/date-format-tofte` | 17500 | 6502 | 0.371543 | 805065 | n/a | 8002 / 2 | 79000 / 11000 | 2322563 | 0 / 11501 |
| `external/sunspider-1.0/date-format-xparb` | 36035 | 6 | 0.000167 | 181296 | n/a | 32180 / 60 | 0 / 4 | 850073 | 0 / 4023 |
| `external/sunspider-1.0/math-cordic` | 25013 | 2 | 0.000080 | 128 | n/a | 0 / 0 | 0 / 0 | 925034 | 25000 / 1 |
| `external/sunspider-1.0/math-partial-sums` | 5 | 6 | 1.200000 | 1005 | n/a | 40 / 0 | 0 / 0 | 0 | 5 / 5 |
| `external/sunspider-1.0/math-spectral-norm` | 122642 | 6 | 0.000049 | 193 | n/a | 0 / 0 | 0 / 0 | 2883 | 170 / 5 |
| `external/sunspider-1.0/regexp-dna` | 0 | 1 | n/a | 994 | n/a | 82 / 0 | 142 / 0 | 0 | 0 / 20 |
| `external/sunspider-1.0/string-base64` | 4 | 3 | 0.750000 | 205172 | n/a | 32768 / 0 | 0 / 0 | 2326803 | 0 / 8196 |
| `external/sunspider-1.0/string-fasta` | 56005 | 2 | 0.000036 | 154 | n/a | 0 / 0 | 0 / 0 | 5951525 | 0 / 1 |
| `external/sunspider-1.0/string-tagcloud` | 40891 | 5 | 0.000122 | 310235 | n/a | 40036 / 14 | 10000 / 0 | 764463 | 2500 / 2500 |
| `external/sunspider-1.0/string-unpack-code` | 183230 | 9 | 0.000049 | 241573 | n/a | 64 / 0 | 19644 / 19612 | 616106 | 0 / 9816 |
| `external/sunspider-1.0/string-validate-input` | 20001 | 5 | 0.000250 | 137 | n/a | 0 / 0 | 0 / 0 | 1388035 | 4000 / 4 |

### Internal count increments

N=20,000 minus N=10,000; identical diagnostic executable, outputs checked against NG at each N. Vm/call uses increments of both existing counters, removing the fixed root construction. These remain counter-defined events, not nanosecond costs.

| Case | Nominal op increment | JS call increment | Vm increment | Vm/call increment | VM ops/nominal increment | Named R/W increment |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `broad/plain_function_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/method_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/captured_read` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/captured_write` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/many_locals_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/property_read` | 30000 | 0 | 0 | n/a | 4.000000 | 0 / 0 |
| `broad/array_read` | 40000 | 0 | 0 | n/a | 3.750000 | 0 / 0 |
| `broad/function_call_two_args` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/function_call_reordered` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/top_level_function_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `broad/dynamic_method_call` | 10000 | 10000 | 0 | 0.000000 | 27.500000 | 20000 / 0 |
| `broad/local_read` | 20000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/global_read` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/property_dynamic_read` | 30000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/property_write` | 30000 | 0 | 0 | n/a | 8.000000 | 0 / 0 |
| `broad/array_dynamic_read` | 40000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/array_write` | 40000 | 0 | 0 | n/a | 7.000000 | 0 / 0 |
| `broad/empty_loop` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/branch_arithmetic` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/math_abs` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/array_index_of` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/string_slice` | 10000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `broad/object_allocation` | 10000 | 0 | 0 | n/a | 4.000000 | 0 / 0 |
| `broad/array_allocation` | 10000 | 0 | 0 | n/a | 4.000000 | 0 / 0 |
| `broad/closure_allocation_call` | 10000 | 10000 | 0 | 0.000000 | 9.000000 | 0 / 0 |
| `sentinel/recursive_call_tree` | 1270000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `sentinel/prototype_method_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `sentinel/polymorphic_call_site` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `sentinel/capturing_closure_call` | 10000 | 10000 | 0 | 0.000000 | 0.000000 | 0 / 0 |
| `sentinel/heterogeneous_property_read` | 30000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |
| `sentinel/string_key_map_churn` | 20000 | 0 | 0 | n/a | 0.000000 | 0 / 0 |

### Call-tier counts

Raw N=10,000/full-program counts. These columns are not a partition: direct-leaf admission can subsequently run a standalone compact activation, and JS call attempts include compact direct calls. `same_vm_frame_entries` is zero for all cases.

| Case | Closed leaf | Direct-leaf admission | General call frames | Compact standalone | Compact direct | Compact caller-env | Native |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `broad/plain_function_call` | 9999 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/method_call` | 10000 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/captured_read` | 9999 | 2 | 1 | 3 | 2 | 1 | 3 |
| `broad/captured_write` | 0 | 2 | 1 | 3 | 10001 | 1 | 3 |
| `broad/many_locals_call` | 9999 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/property_read` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/array_read` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/function_call_two_args` | 9999 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/function_call_reordered` | 9999 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/top_level_function_call` | 10000 | 2 | 0 | 2 | 0 | 0 | 3 |
| `broad/dynamic_method_call` | 10000 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/local_read` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/global_read` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/property_dynamic_read` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/property_write` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/array_dynamic_read` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/array_write` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/empty_loop` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/branch_arithmetic` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/math_abs` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/array_index_of` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/string_slice` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `broad/object_allocation` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/array_allocation` | 0 | 3 | 0 | 3 | 0 | 1 | 3 |
| `broad/closure_allocation_call` | 10000 | 3 | 0 | 3 | 0 | 1 | 3 |
| `sentinel/recursive_call_tree` | 0 | 2 | 0 | 3 | 1 | 1 | 3 |
| `sentinel/prototype_method_call` | 9999 | 2 | 0 | 3 | 67 | 1 | 67 |
| `sentinel/polymorphic_call_site` | 9999 | 2 | 0 | 3 | 1 | 1 | 3 |
| `sentinel/capturing_closure_call` | 9999 | 2 | 4 | 3 | 2 | 1 | 3 |
| `sentinel/heterogeneous_property_read` | 0 | 2 | 0 | 3 | 2 | 1 | 67 |
| `sentinel/string_key_map_churn` | 0 | 2 | 0 | 3 | 2 | 1 | 1027 |
| `external/jetstream3-js-subset/cdjs` | 0 | 112917 | 18038 | 112918 | 2162393 | 1 | 650927 |
| `external/jetstream3-js-subset/hash-map` | 180000 | 15 | 1 | 16 | 2160029 | 1 | 5 |
| `external/jetstream3-js-subset/gaussian-blur` | 360000 | 2 | 1 | 1 | 4 | 0 | 9 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 0 | 76 | 0 | 75 | 107103 | 0 | 31512 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0 | 74 | 180 | 8 | 2013480 | 0 | 20821 |
| `external/kraken-1.1/ai-astar` | 2 | 10005 | 0 | 10006 | 53305 | 1 | 33923 |
| `external/kraken-1.1/audio-beat-detection` | 0 | 1295 | 1 | 295 | 7001 | 0 | 1427 |
| `external/kraken-1.1/audio-dft` | 0 | 2 | 1 | 2 | 100 | 0 | 6 |
| `external/kraken-1.1/audio-fft` | 0 | 2 | 1 | 2 | 5000 | 0 | 6 |
| `external/kraken-1.1/audio-oscillator` | 0 | 2049 | 503 | 2049 | 2000 | 0 | 5561 |
| `external/kraken-1.1/imaging-gaussian-blur` | 0 | 2 | 0 | 2 | 0 | 0 | 57 |
| `external/kraken-1.1/imaging-darkroom` | 0 | 19 | 0 | 19 | 3 | 0 | 1 |
| `external/kraken-1.1/imaging-desaturate` | 0 | 200 | 0 | 200 | 0 | 0 | 1 |
| `external/kraken-1.1/stanford-crypto-aes` | 0 | 78 | 7 | 77 | 315861 | 0 | 83135 |
| `external/kraken-1.1/stanford-crypto-ccm` | 0 | 8025 | 20 | 12153 | 204555 | 9075 | 163846 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 0 | 121 | 7 | 120 | 377007 | 8 | 360678 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 0 | 1091 | 24 | 3090 | 29686 | 2010 | 35564 |
| `external/sunspider-1.0/3d-cube` | 0 | 4 | 0 | 4 | 13128 | 0 | 2064 |
| `external/sunspider-1.0/3d-morph` | 0 | 15 | 0 | 15 | 0 | 0 | 2 |
| `external/sunspider-1.0/3d-raytrace` | 0 | 43 | 1 | 42 | 56584 | 0 | 2796 |
| `external/sunspider-1.0/access-binary-trees` | 0 | 1616 | 0 | 1616 | 124600 | 0 | 5 |
| `external/sunspider-1.0/access-fannkuch` | 0 | 1 | 0 | 1 | 0 | 0 | 5 |
| `external/sunspider-1.0/access-nbody` | 0 | 5 | 0 | 5 | 4555 | 0 | 5 |
| `external/sunspider-1.0/access-nsieve` | 0 | 1 | 0 | 1 | 3 | 0 | 4 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 127999 | 1 | 0 | 1 | 0 | 0 | 1 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 0 | 1 | 0 | 1 | 0 | 0 | 1 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 0 | 1 | 0 | 1 | 1 | 0 | 1 |
| `external/sunspider-1.0/controlflow-recursive` | 0 | 9 | 0 | 0 | 0 | 0 | 1 |
| `external/sunspider-1.0/crypto-aes` | 0 | 172 | 0 | 116 | 9774 | 114 | 1879 |
| `external/sunspider-1.0/crypto-md5` | 0 | 1 | 0 | 1 | 11997 | 0 | 3 |
| `external/sunspider-1.0/crypto-sha1` | 86731 | 1 | 0 | 1 | 3 | 0 | 4 |
| `external/sunspider-1.0/date-format-tofte` | 0 | 16500 | 1000 | 16500 | 0 | 0 | 8502 |
| `external/sunspider-1.0/date-format-xparb` | 0 | 8022 | 2 | 8022 | 28011 | 0 | 125 |
| `external/sunspider-1.0/math-cordic` | 12 | 1 | 0 | 1 | 25000 | 0 | 3 |
| `external/sunspider-1.0/math-partial-sums` | 0 | 5 | 0 | 0 | 0 | 0 | 1 |
| `external/sunspider-1.0/math-spectral-norm` | 122398 | 4 | 0 | 4 | 240 | 0 | 1 |
| `external/sunspider-1.0/string-base64` | 0 | 4 | 0 | 4 | 0 | 0 | 1 |
| `external/sunspider-1.0/string-fasta` | 0 | 3 | 0 | 3 | 56002 | 0 | 935 |
| `external/sunspider-1.0/string-tagcloud` | 0 | 2 | 2 | 25887 | 15001 | 25886 | 25039 |
| `external/sunspider-1.0/string-unpack-code` | 0 | 10542 | 8 | 92008 | 9748 | 81466 | 9949 |
| `external/sunspider-1.0/string-validate-input` | 0 | 1 | 0 | 1 | 20000 | 0 | 4 |

Missing requested dynamic event totals (for every case): whole-frame emitted copies; scoped property-name comparisons/hashes; IC probes/rejects; Value clone/drop totals; allocation/free totals; exact lowered-op histogram. These cells remain **unavailable**, not zero.

## Uninstrumented sampled shares

Percent of main-thread samples. Named pipeline and frame preparation columns are inclusive union envelopes and overlap other columns. Value glue, string/hash and virtual fields use self samples. Alloc/free is an inclusive union envelope. These are sampling shares, not dynamic event counts or removable fractions. Inline work can be charged to its caller. Profiles with fewer than 100 main-thread samples are sparse diagnostics, not reliable quantitative attribution; a zero group is not evidence of zero cost. Every parsed tree conserves its main-thread sample count.

| Case | Samples | Named pipeline incl. | Frame prep incl. | Value glue self | Alloc/free incl. | String/hash self | Virtual fields self |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `broad/plain_function_call` | 58 | 0.00% | 0.00% | 0.00% | 1.72% | 0.00% | 0.00% |
| `broad/method_call` | 104 | 9.62% | 0.00% | 6.73% | 0.00% | 0.00% | 0.00% |
| `broad/captured_read` | 68 | 0.00% | 0.00% | 2.94% | 0.00% | 0.00% | 0.00% |
| `broad/captured_write` | 252 | 0.00% | 0.00% | 7.14% | 0.00% | 0.00% | 0.00% |
| `broad/many_locals_call` | 162 | 0.00% | 0.00% | 0.62% | 0.00% | 0.00% | 0.00% |
| `broad/property_read` | 416 | 0.00% | 0.00% | 11.06% | 0.00% | 0.00% | 49.52% |
| `broad/array_read` | 517 | 0.00% | 0.00% | 7.74% | 0.19% | 0.00% | 57.06% |
| `broad/function_call_two_args` | 66 | 0.00% | 0.00% | 4.55% | 0.00% | 0.00% | 0.00% |
| `broad/function_call_reordered` | 66 | 0.00% | 0.00% | 1.52% | 0.00% | 0.00% | 0.00% |
| `broad/top_level_function_call` | 165 | 0.00% | 0.00% | 9.70% | 0.00% | 0.00% | 0.00% |
| `broad/dynamic_method_call` | 484 | 10.33% | 0.00% | 12.81% | 0.00% | 0.00% | 5.99% |
| `broad/local_read` | 6 | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% |
| `broad/global_read` | missing: no main thread samples | — | — | — | — | — | — |
| `broad/property_dynamic_read` | 325 | 0.00% | 0.00% | 17.54% | 0.00% | 4.00% | 0.00% |
| `broad/property_write` | 621 | 0.00% | 0.00% | 11.43% | 0.00% | 0.00% | 41.22% |
| `broad/array_dynamic_read` | 45 | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% |
| `broad/array_write` | 686 | 0.00% | 0.00% | 12.68% | 0.00% | 0.00% | 39.36% |
| `broad/empty_loop` | missing: no usable sample call graph | — | — | — | — | — | — |
| `broad/branch_arithmetic` | 23 | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% |
| `broad/math_abs` | 37 | 0.00% | 0.00% | 5.41% | 0.00% | 0.00% | 0.00% |
| `broad/array_index_of` | 217 | 0.00% | 0.00% | 6.45% | 0.46% | 0.00% | 0.00% |
| `broad/string_slice` | 537 | 0.00% | 0.00% | 7.26% | 47.49% | 0.00% | 0.00% |
| `broad/object_allocation` | 179 | 0.00% | 0.00% | 5.59% | 0.00% | 0.00% | 46.37% |
| `broad/array_allocation` | 182 | 0.00% | 0.00% | 9.89% | 0.00% | 0.00% | 44.51% |
| `broad/closure_allocation_call` | 318 | 0.00% | 0.00% | 5.35% | 0.00% | 0.00% | 50.94% |
| `sentinel/recursive_call_tree` | 1672 | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% | 0.00% |
| `sentinel/prototype_method_call` | 219 | 8.22% | 0.00% | 6.39% | 0.00% | 0.00% | 0.00% |
| `sentinel/polymorphic_call_site` | 133 | 0.00% | 0.00% | 10.53% | 0.00% | 0.00% | 0.00% |
| `sentinel/capturing_closure_call` | 136 | 0.00% | 0.00% | 5.88% | 0.00% | 0.00% | 0.00% |
| `sentinel/heterogeneous_property_read` | 234 | 16.24% | 0.00% | 8.12% | 0.00% | 0.00% | 0.00% |
| `sentinel/string_key_map_churn` | 285 | 0.00% | 0.00% | 14.74% | 0.00% | 8.42% | 0.00% |
| `external/jetstream3-js-subset/cdjs` | 880 | 3.98% | 4.09% | 11.02% | 17.39% | 1.14% | 0.00% |
| `external/jetstream3-js-subset/hash-map` | 482 | 17.22% | 0.00% | 5.60% | 3.73% | 0.21% | 0.00% |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 319 | 3.45% | 0.94% | 10.03% | 9.09% | 0.31% | 0.00% |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 733 | 6.28% | 0.00% | 9.00% | 13.37% | 2.46% | 0.00% |
| `external/kraken-1.1/ai-astar` | 1663 | 0.36% | 0.06% | 20.93% | 1.62% | 0.18% | 0.00% |
| `external/kraken-1.1/stanford-crypto-aes` | 860 | 3.37% | 1.05% | 8.02% | 8.49% | 0.35% | 0.00% |
| `external/sunspider-1.0/3d-raytrace` | 13 | 7.69% | 0.00% | 15.38% | 7.69% | 0.00% | 0.00% |
| `external/sunspider-1.0/access-binary-trees` | 2 | 0.00% | 0.00% | 0.00% | 50.00% | 0.00% | 0.00% |
| `external/sunspider-1.0/access-nbody` | missing: no main thread samples | — | — | — | — | — | — |
| `external/sunspider-1.0/crypto-aes` | 21 | 0.00% | 0.00% | 23.81% | 9.52% | 0.00% | 0.00% |
| `external/sunspider-1.0/crypto-md5` | missing: no usable sample call graph | — | — | — | — | — | — |
| `external/sunspider-1.0/crypto-sha1` | missing: no usable sample call graph | — | — | — | — | — | — |
| `external/sunspider-1.0/date-format-tofte` | 87 | 2.30% | 10.34% | 4.60% | 22.99% | 4.60% | 0.00% |
| `external/sunspider-1.0/math-partial-sums` | missing: no usable sample call graph | — | — | — | — | — | — |
| `external/sunspider-1.0/math-spectral-norm` | missing: no usable sample call graph | — | — | — | — | — | — |

## Memory baseline

Three fresh processes per case and engine. Numbers are exact bytes from wait4 ru_maxrss. Internal N=10,000; external/held-out inputs are the original frozen whole programs. M3 must not worsen the matching base memory measurement; these observations are a reference envelope, not an allocator-event estimate or a license for percentage slack. Future gates need fresh paired base/candidate resource evidence at these same sizes.

| Case | Rust median bytes | Rust max bytes | NG median bytes | NG max bytes |
| --- | ---: | ---: | ---: | ---: |
| `broad/array_allocation` | 4935680 | 4935680 | 1757184 | 1757184 |
| `broad/array_dynamic_read` | 5005312 | 5005312 | 1757184 | 1757184 |
| `broad/array_index_of` | 5013504 | 5013504 | 1761280 | 1761280 |
| `broad/array_read` | 4993024 | 5103616 | 1757184 | 1757184 |
| `broad/array_write` | 4931584 | 4931584 | 1761280 | 1761280 |
| `broad/branch_arithmetic` | 5001216 | 5001216 | 1757184 | 1757184 |
| `broad/captured_read` | 5033984 | 5033984 | 1757184 | 1757184 |
| `broad/captured_write` | 4947968 | 4947968 | 1757184 | 1757184 |
| `broad/closure_allocation_call` | 4943872 | 4943872 | 1761280 | 1761280 |
| `broad/dynamic_method_call` | 4993024 | 4993024 | 1761280 | 1761280 |
| `broad/empty_loop` | 5001216 | 5001216 | 1757184 | 1757184 |
| `broad/function_call_reordered` | 5025792 | 5025792 | 1757184 | 1757184 |
| `broad/function_call_two_args` | 5025792 | 5025792 | 1757184 | 1757184 |
| `broad/global_read` | 5001216 | 5001216 | 1757184 | 1757184 |
| `broad/local_read` | 5001216 | 5001216 | 1757184 | 1757184 |
| `broad/many_locals_call` | 5025792 | 5025792 | 1757184 | 1757184 |
| `broad/math_abs` | 5001216 | 5206016 | 1757184 | 1757184 |
| `broad/method_call` | 5005312 | 5005312 | 1761280 | 1761280 |
| `broad/object_allocation` | 4935680 | 4935680 | 1757184 | 1757184 |
| `broad/plain_function_call` | 5025792 | 5025792 | 1757184 | 1757184 |
| `broad/property_dynamic_read` | 5005312 | 5005312 | 1757184 | 1757184 |
| `broad/property_read` | 5017600 | 5017600 | 1757184 | 1757184 |
| `broad/property_write` | 5013504 | 5013504 | 1757184 | 1757184 |
| `broad/string_slice` | 5013504 | 5013504 | 1765376 | 1765376 |
| `broad/top_level_function_call` | 4825088 | 4825088 | 1757184 | 1757184 |
| `external/jetstream3-js-subset/cdjs` | 75935744 | 75984896 | 5005312 | 5095424 |
| `external/jetstream3-js-subset/gaussian-blur` | 14848000 | 14950400 | 4669440 | 4669440 |
| `external/jetstream3-js-subset/hash-map` | 38989824 | 39133184 | 18841600 | 18903040 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 6004736 | 6082560 | 1904640 | 2002944 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 39071744 | 39219200 | 6242304 | 7053312 |
| `external/kraken-1.1/ai-astar` | 64827392 | 64835584 | 9932800 | 9961472 |
| `external/kraken-1.1/audio-beat-detection` | 18059264 | 21520384 | 4067328 | 4468736 |
| `external/kraken-1.1/audio-dft` | 46510080 | 46710784 | 33542144 | 33562624 |
| `external/kraken-1.1/audio-fft` | 15654912 | 16003072 | 3792896 | 3981312 |
| `external/kraken-1.1/audio-oscillator` | 15388672 | 15589376 | 4218880 | 4677632 |
| `external/kraken-1.1/imaging-darkroom` | 289705984 | 297476096 | 40271872 | 40271872 |
| `external/kraken-1.1/imaging-desaturate` | 319565824 | 319565824 | 40550400 | 40550400 |
| `external/kraken-1.1/imaging-gaussian-blur` | 286109696 | 293441536 | 40239104 | 40239104 |
| `external/kraken-1.1/json-parse-financial` | 4038656 | 4038656 | 1814528 | 1867776 |
| `external/kraken-1.1/json-stringify-tinderbox` | 23273472 | 23273472 | 4902912 | 6500352 |
| `external/kraken-1.1/stanford-crypto-aes` | 42352640 | 46080000 | 5771264 | 5926912 |
| `external/kraken-1.1/stanford-crypto-ccm` | 22478848 | 24875008 | 5070848 | 5234688 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 8552448 | 8634368 | 2375680 | 2392064 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 8609792 | 8691712 | 2101248 | 2433024 |
| `external/sunspider-1.0/3d-cube` | 5500928 | 5500928 | 1794048 | 1794048 |
| `external/sunspider-1.0/3d-morph` | 4558848 | 4558848 | 2396160 | 2396160 |
| `external/sunspider-1.0/3d-raytrace` | 5685248 | 5685248 | 2101248 | 2121728 |
| `external/sunspider-1.0/access-binary-trees` | 3981312 | 4042752 | 1695744 | 1695744 |
| `external/sunspider-1.0/access-fannkuch` | 4132864 | 4132864 | 1658880 | 1671168 |
| `external/sunspider-1.0/access-nbody` | 4292608 | 4292608 | 1646592 | 1658880 |
| `external/sunspider-1.0/access-nsieve` | 6213632 | 6213632 | 6606848 | 6606848 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 3608576 | 3657728 | 1585152 | 1585152 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 3641344 | 3641344 | 1585152 | 1585152 |
| `external/sunspider-1.0/bitops-bitwise-and` | 3301376 | 3301376 | 1564672 | 1564672 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 3948544 | 3948544 | 1773568 | 1773568 |
| `external/sunspider-1.0/controlflow-recursive` | 3604480 | 3670016 | 1835008 | 1835008 |
| `external/sunspider-1.0/crypto-aes` | 5799936 | 5799936 | 1953792 | 1953792 |
| `external/sunspider-1.0/crypto-md5` | 4882432 | 4882432 | 2093056 | 2093056 |
| `external/sunspider-1.0/crypto-sha1` | 4538368 | 4538368 | 1871872 | 1871872 |
| `external/sunspider-1.0/date-format-tofte` | 23474176 | 23621632 | 1884160 | 1884160 |
| `external/sunspider-1.0/date-format-xparb` | 5111808 | 5111808 | 1990656 | 1990656 |
| `external/sunspider-1.0/math-cordic` | 3862528 | 3862528 | 1626112 | 1626112 |
| `external/sunspider-1.0/math-partial-sums` | 3637248 | 3637248 | 1605632 | 1605632 |
| `external/sunspider-1.0/math-spectral-norm` | 3981312 | 3981312 | 1634304 | 1634304 |
| `external/sunspider-1.0/regexp-dna` | 11206656 | 11206656 | 3551232 | 3981312 |
| `external/sunspider-1.0/string-base64` | 4141056 | 4141056 | 1753088 | 1753088 |
| `external/sunspider-1.0/string-fasta` | 4001792 | 4001792 | 1888256 | 1929216 |
| `external/sunspider-1.0/string-tagcloud` | 13561856 | 16523264 | 6000640 | 6000640 |
| `external/sunspider-1.0/string-unpack-code` | 10416128 | 11067392 | 6778880 | 6946816 |
| `external/sunspider-1.0/string-validate-input` | 4603904 | 4603904 | 2330624 | 2330624 |
| `held-out/acorn/parse` | 43098112 | 44351488 | 10547200 | 10661888 |
| `held-out/crypto-js/hash` | 76447744 | 76509184 | 5439488 | 5992448 |
| `held-out/d3-array/aggregate` | 48480256 | 53313536 | 20787200 | 21651456 |
| `held-out/decimal.js/arith` | 13647872 | 13701120 | 2875392 | 2924544 |
| `held-out/esprima/parse` | 108613632 | 108699648 | 41218048 | 41230336 |
| `held-out/handlebars/render` | 24498176 | 26505216 | 4354048 | 4435968 |
| `held-out/marked/render` | 14376960 | 15069184 | 3702784 | 3887104 |
| `held-out/mustache/render` | 6819840 | 6893568 | 2277376 | 2322432 |
| `held-out/ramda/pipeline` | 20893696 | 21225472 | 3878912 | 3932160 |
| `sentinel/capturing_closure_call` | 4333568 | 4333568 | 1724416 | 1724416 |
| `sentinel/heterogeneous_property_read` | 4358144 | 4358144 | 1732608 | 1732608 |
| `sentinel/polymorphic_call_site` | 4325376 | 4325376 | 1724416 | 1724416 |
| `sentinel/prototype_method_call` | 4366336 | 4366336 | 1732608 | 1732608 |
| `sentinel/recursive_call_tree` | 4358144 | 4358144 | 1724416 | 1724416 |
| `sentinel/string_key_map_churn` | 4911104 | 4911104 | 1925120 | 1925120 |

## Value-word qualification

Qualification is for `x86_64-apple-darwin` on the measurement iMac and
`aarch64-apple-darwin` on this Mac, using Rust 1.95.0. It is not an arm64e,
CHERI, sanitizer-tagged-pointer or arbitrary future-address-space claim.
The local ABI probe was the brief's explicit arm64 work, not a local benchmark.
It allocates 16,384 simultaneously live `Box<Aligned([u64;4])>` values with
explicit 16-byte payload alignment, exposes each original pointer's provenance,
asserts its upper 16 bits are zero, packs tag `0xfff9`, decodes the low 48 bits,
reconstructs the identical pointer with `with_exposed_provenance_mut`, reads its
initialized payload through the reconstructed Box, and destroys it exactly once.
This tests an ownership round trip, not refcount/weak-header implementation.

Apple's [arm64 ABI](https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms)
specifies 8-byte pointer size/alignment. The installed SDK's ARM header defines
the macOS maximum raw user address as `0x00007ffffe000000`; its Intel header
uses `0x00007fffffe00000` (`VM_MAX_USER_PAGE_ADDRESS` is
`0x00007ffffffff000`). These are below 48 payload bits. Header SHA-256 values
are recorded with the probe output. [XNU's ARM header](https://github.com/apple-oss-distributions/xnu/blob/xnu-12377.1.9/osfmk/mach/arm/vm_param.h)
and [Intel header](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/i386/vm_param.h)
provide the source context; a current SDK/source limit is not a perpetual ABI
promise or a proof about every OS mapping.

The release protocol must therefore check the full allocation address before
encoding it, retain the enum backend on unsupported targets, and never silently
mask a nonzero upper address/tag/PAC bit. Allocate the header and payload with
an explicit Rust `Layout` and alignment; neither pointer alignment nor today's
malloc behavior proves the layout of an Rc control block. No Rc-layout guess,
FFI, signed function pointer or process-global handle table is needed.

Rust's [exposed-provenance constructor](https://doc.rust-lang.org/std/ptr/fn.with_exposed_provenance_mut.html)
requires provenance exposed from the same live allocation. Packing an address
obtained only with `addr()` and later fabricating a pointer is not that protocol.
A raw-pointer word using `map_addr` can preserve strict provenance instead;
this is an alternative to evaluate before selecting the backend, not an
implemented engine path. [Rust's pointer documentation](https://doc.rust-lang.org/std/primitive.pointer.html)
explains this distinction and the limitations of exposed provenance for tools.
Keeping only integer bits does not retain ownership: strong/weak counts must
keep the allocation/tombstone alive, clones must retain, moves must transfer,
and last-owner destruction must use the original allocation layout once.
Header-kind checks, borrow rules, COW uniqueness and weak-address reuse remain
separate proof obligations. Numeric NaNs must be canonicalized before they
could collide with `0xfff8/0xfff9`; infinities, signed zero and raw buffer bytes
must retain their specified behavior. No such engine backend exists in this unit.

Conclusion: both requested ABIs can be candidates for an 8-byte backend subject
to checked address admission and the stated provenance/ownership contract.
This is not release qualification of an unimplemented header/backend. M3 still
requires its focused lifetime/bit-pattern tests, both backends, Miri where its
provenance mode is supported, full conformance and measured payoff/resource gates.


### Observed ABI probe outputs

```text
arch=aarch64 pointer_bytes=8 pointer_align=8 usize_bytes=8 payload_align=16 samples=16384 min=0x100ea1c30 max=0xc45480000 observed_bits=36 rc_payload=0x100ea1c40 exposed_roundtrip=pass
arch=x86_64 pointer_bytes=8 pointer_align=8 usize_bytes=8 payload_align=16 samples=16384 min=0x600003e00000 max=0x600003ec9120 observed_bits=47 rc_payload=0x600000c48010 exposed_roundtrip=pass
```

Both probes used identical Rust source; all 16,384 full-address assertions and exposed-provenance Box ownership round trips passed. The observed address range is empirical, not an ABI ceiling.

Local SDK header SHA-256: ARM `054f1c94f82c4b15b5c84efc5a84f256f7f991749e69e7d2a2c3ca03e3551fff`, Intel `c7b2525d070dd46a40f28cbcf829dd2c90b0bcf310d2d4a7a0c47a040fb8b947`.

## Findings and ordering judgment

1. **M1 cannot claim the leading broad field deficits as atom wins.** At
   N=20,000 minus N=10,000, property_read/array_read/property_write each add
   zero VM named reads/writes, zero compact ops and zero Vm constructions.
   They add respectively 120,000/150,000/240,000 VM dispatches. Per declared
   operation that is 4.0/3.75/8.0 dispatches. Each adds 10,000 declined loop
   edges. Uninstrumented samples place 206/416, 295/517 and 256/621 self
   samples in virtual-field handlers; direct Value clone/drop glue has
   46/416, 40/517 and 71/621 self samples. These are field-bank/boxed-dispatch
   costs, not evidence of repeated property-name lookup. Preserve that route;
   materializing its placeholder to populate a property IC would be the wrong fix.
2. **There is genuine ordinary named-access reach, but it needs a different
   measurement claim.** dynamic_method_call adds 20,000 VM named reads and
   10,000 JS call attempts for 10,000 declared calls, with zero new Vms and
   10,000 declined edges. Its observed named-pipeline envelope is 50/484
   main-thread samples (10.33%). Its v2 wall band is 1.106054806907: a payoff
   target there must also beat the reciprocal 0.904114329376. A nominal 3%
   gain would not establish a target win under that qualified rule. Keep it
   as an ordinary-path mechanism discriminator/control, not an invented easy target.
3. **The two provisional real-program payoff targets exercise shared property
   helpers mostly from accelerators.** HashMap records 42,300,893 compact
   ops, 409 VM ops and nine Vm constructions for 2,340,047 JS attempts.
   Public-field Raytrace is a second independent constructor/field program.
   Their visible named-pipeline envelopes are 83/482 (17.22%) and 46/733
   (6.28%). Visible string/hash self samples are only 1/482 in HashMap; inlined
   comparisons may be charged elsewhere. This justifies investigating layout
   authority beyond string interning; it does not prove an ordinary-driver speedup or make those
   inclusive percentages removable. Count ordinary fallback separately in
   part 2 before implementation. Source storage adapters must cover all four
   stores and computed-key mutation, not just the receivers those programs use.
4. **A callee's inclusive chain is not call overhead.** Kraken AES has
   66,508 Vm constructions / 315,946 JS attempts, but the 841/860 samples
   under call/execution wrappers include its typed/compact callees. Visible
   preparation helpers account for only nine inclusive samples (five self),
   direct Value glue for 69 self samples, allocator routines for 73 inclusive
   samples and copying routines for 14 self samples. CDjs has 36/880 visible
   preparation samples (ten self), 97 Value-glue self samples and 153 allocator
   samples. Helpers can be inlined, so these are observed symbol scopes,
   not complete upper bounds on frame-related work. They falsify treating
   98% of an inclusive call chain as a removable frame cost.
5. **Do not infer a uniform execution tier from the sentinel label.** The
   recursive-call sentinel adds 1,270,000 nominal operations between the two
   N values but zero measured JS call attempts/Vms/VM ops/compact ops. The
   capturing-closure sentinel adds 10,000 JS attempts but zero Vms. Those
   observations do not price ordinary frame construction. The complete
   counters and lowered-code discriminators must accompany a future control.
6. **No supported numeric speedup forecast is established.** Current sampling
   sees direct Value glue and real property work; exact scoped hash/probe,
   clone/drop, allocation and emitted-copy totals remain missing. Inlining
   prevents assigning every cost to one symbol. Preserve the facade as the
   first dependency, then use the missing counters and part 2's independent
   development programs to decide whether atoms/layout authority or boxed
   ownership should follow. Do not move all of M3 ahead solely from the broad
   virtual cases, and do not freeze M1's priority solely from their 3–5x gaps.
   If that additional evidence cannot price an ordinary-path M1 payoff,
   split the facade from atom linking and re-rank Value transport before M1.2/1.3.

The ordinary named pipeline here means the union of observed cache, named-get
and live-slot-reader frames. It omits inlined work charged to an executor and
includes mandatory work inside those helpers. Value glue means the direct
Value Clone/drop-in-place symbols only; aggregate/inlined ownership work can
appear elsewhere. String/hash samples are not scoped exclusively to property
names. These definitions are intentionally narrower than dynamic event counts.

### Linked copy audit

Static address ranges were extracted from this exact c7f0f66a image, not from
an instrumented build. Adjacent-symbol distances include alignment:

| Linked function | Bytes | memcpy callsites | Nearby immediate byte counts |
| --- | ---: | ---: | --- |
| Vm::run_current_activation | 9360 | 0 | none in this function's range |
| eval_function_bytecode | 2048 | 3 | 192, 192, 192 |
| eval_direct_call_bytecode | 1952 | 5 | 192, 184, 79, 184, 184 |
| run_prepared_bytecode_call | 2368 | 3 | 304, 79, 85 |

These sites establish that aggregate transport remains. They are not dynamic
counts, do not identify every copied type and do not establish that any site
copies an entire current FrameState. The constructor may be inlined outside
these selected ranges. The dormant push_frame move is a different source path;
its absence cannot be used to label constructor copies zero. The historical
704-byte frame measurement is not reused.

## M1.1 draft and provisional M1 budgets

The deliverable `M1.1-draft.json` is a schema-3 **three-stage M1 migration with
current_stage=1**, rather than a facade leaf falsely required to earn a final
payoff. It is deliberately outside tasks/performance-units: it is not registered
or frozen, and no M1 runtime implementation is present. Mechanical validation
checks queue/image/host/profile bindings; it does not fill the missing event
evidence, prove a forecast or waive unit 0's prerequisites.

| Item | Draft value | Evidence / obligation |
| --- | --- | --- |
| Fixed source base | b481894d351d933e94717a9c295c15fe4a466804 | Exact fresh queue candidate c7f0f66a image; later tooling/docs are not substituted for its source identity. |
| Payoff targets | external/jetstream3-js-subset/hash-map; external/jetstream3-js-subset/raytrace-public-class-fields | Exact-image samples; two independent real programs; HashMap is external rank 7, inside rank ceiling 8. |
| Intermediate cumulative case cap | 1.025 candidate/base | Slightly tighter than the targets' measured B_i=1.025024955526; cumulative from the fixed base, not 2.5% per stage. All stricter policy control/lane gates still apply. |
| Final M1 payoff | <=0.97 on each target, every pair improving | A 3% minimum clears 1/B_i=0.975586003647. This is an acceptance requirement, not a predicted speedup. |
| Frozen panel shape | K=6, 5 pairs each; offsets 0/16/64/240/512/1008 | Earlier cache/layout experiments failed unrelated controls. Predeclare every placement; no favorable offset selection. |
| Mechanism class | cpu-work; no memory-wall targets | M1 replaces name/lookup CPU work and keeps Value at 16 bytes; M3 is a separate instruction-blind migration with resource obligations. |
| Formal evidence | 30 blocks, complete three-engine portfolio, after a passing stage screen | Current imprecise small-case evidence is not a promotion waiver; retain formal precision requirements. |
| Held-out promotion | 30 pairs, all nine programs, program/lane upper-CI gates | Independent acceptance only, after targets/candidate are frozen. No held-out program was profiled or selected. |
| Attempt limit | 1 valid failed implementation screen for this implementation shape | T029/T041/T049/T053 already show that removing one visible cost need not earn end-to-end payoff. No revived leaf attempt, late target change or layout shopping. A K=6 completion is one screen. |
| M1.1 mechanism obligations | Value remains 16 bytes with current ownership; zero extra allocation/temporary owner per typed view; link once per code/domain, zero steady-state named-operand re-interning | Source/design contracts; exact counts to verify these are still absent. |
| Final M1 mechanism obligations | one layout lookup for ordinary own hits; zero property-name string hashing/comparison on already-linked named access | Computed ToPropertyKey/interning and exotic algorithms remain real work. No literal-only admission or tier switch may supply the payoff. |

The policy's lane ceilings remain broad 1.012949330023, sentinel
1.015687001890, external 1.007380237554 and equal-case portfolio
1.008262331965; held-out is 1.0029. Full-precision/host/image/resource/correctness
requirements are not replaced by this table. The adopted v2 artifact contains
no diagnostic-only cap-imprecise case. K=6 is an explicit draft choice here.

To earn 3% solely inside the visible named envelopes would require roughly
17.4% of HashMap's envelope and 47.8% of public-field Raytrace's envelope to
be removed, assuming sampled shares tracked the relevant latency. Mandatory
reads, validation and ownership remain in both envelopes. Those requirements
are not measured removable fractions, so **no 3% forecast is asserted**. Missing
scoped counters and part 2 prevent converting this parameterized draft into
an implementation authorization or a frozen numeric migration budget.

M3's current RSS reference is the complete 85-case table above. An 8-byte word
does not imply half the process RSS: payloads, code, metadata, refcounts and
header-kind overhead remain. Retain these same workloads/sizes, refresh M3's
own fixed-base paired resource evidence after intervening migrations, and
require no memory worsening; do not invent a percentage allowance from three
RSS observations. Atom/shape churn retained-memory ceilings are also not yet
priced by this steady-program RSS table.

### Tooling correction and verification

At b481894d the schema-3 perf-loop wrapper called the existing layout rule
without stage/stage_limit. A completely neutral six-layout synthetic packet
therefore failed 0.97 final payoff during an intermediate stage; the same
packet passed when the stage budget was supplied. ddca5bdf fixes only that
plumbing: intermediate stages use cumulative targets/regression cap, final
stages and leaves keep payoff. Four focused tests cover neutrality, persistent
over-budget cost, final payoff and leaf behavior. The 556-test fast tool gate
passed, then check.sh at ddca5bdf passed on the iMac, including 5,170 curated
Test262 cases and all feature/tool checks. All 31 internal profile outputs at
N=3,000,000 match NG. All 31 N=10,000 counter/resource checksums agree across
roles, all 76 opcode-family sums conserve executed_ops, and all 31 N/2N count
deltas are nonnegative. No full configured Test262 scan or new burndown is
claimed: no runtime/parser/lexer semantics changed in this unit.

## Open issues / next action

- The brief's no-runtime-code boundary conflicts with the absent dynamic event
  counters. A diagnostic scope clarification was requested; no runtime patch
  was made. Define and add those counters in an authorized follow-up before
  freezing M1. Keep diagnostic images out of all timing decisions.
- Finish part 2's independent development-program profiling. Current canonical
  targets mainly exercise property helpers from accelerators; they are not
  two independent ordinary-driver proofs.
- The new formal bundle's bitops-3bit-bits-in-byte interval remains imprecise:
  relative half-width 8.1566% versus base and 7.2666% versus NG. This non-A/A
  comparison cannot be appended with the identical-source A/A continuation
  tool. Do not relabel it, drop samples, pool another run or mark it a
  cap-imprecise policy exception. Future promotion still needs valid precision
  evidence under the standing allocation rule where applicable.
- Short-process sampler attachment is incomplete for several controls/top
  cases; original no-graph artifacts are retained and reported, not zeros.
  Three pre-attached attempts per case were completed through the shared host;
  only tofte yielded new usable graphs. A launch-capable profiler/protocol is
  still needed for the original fast cases. All outcomes are recorded below.
- The pointer-word protocol is feasible on the probed ABIs subject to full
  address checks and ownership proof; the engine header/backend and weak/COW
  lifetime implementation remain unqualified and unimplemented.

Next useful work: close instrumentation definitions/boundary and part 2,
then review the measured ordering and complete/freeze a new plan before any
runtime edits. Preserve this draft as a review artifact, not a promotion claim.

## Draft validation binding

`M1.1-draft.json` and `M1.1-draft-validation.json` are review artifacts in the requested report directory, outside the frozen plan registry.

```json
{
  "artifact_type": "quickjs-performance-unit-validation",
  "base_sha": "b481894d351d933e94717a9c295c15fe4a466804",
  "priority_mode": "queue",
  "queue_sha256": "0f4a1fda656257bccd614b0914796e855752ec3441de4e23f3e95f318c834570",
  "schema_version": 1,
  "status": "valid",
  "unit_id": "interpreter-core-v2-atoms-layouts",
  "unit_sha256": "7749210104f1c71e6c1c3e8acaf00c01fddf5f0cfd7736b47d19d5759301507e"
}
```

## Original profile receipts and raw sample identities

All listed receipts bind to candidate image `c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e`. macOS redacts user paths in sample headers; PID, invocation/image hash and common qjs Mach-O UUID `4D132B36-EA8A-3C25-9D4A-EBD8ED2B19C2` provide additional cross-checks. Visible frame scopes omit inlined work.

| Case | Main-thread samples | Raw sample SHA-256 | Receipt SHA-256 |
| --- | ---: | --- | --- |
| `broad/plain_function_call` | 58 | `78d8b9f2cf74798d757257eff301b46f92dcf4f0ba6175e8243462b03fdf9a6a` | `c819d9976987f95a37a7e13044204259d63676201ea5c4498472e878fdb87417` |
| `broad/method_call` | 104 | `40fbf95054353127741c9eb7e3bbe9aca35c10208d6dc702dc6690490fda0643` | `fdb013c8b021c93e1d3bf0c016e6a297fd24ff3272d59a06c1239ea38f0e921d` |
| `broad/captured_read` | 68 | `6ce6109095fc9e9de92fa18c20e7edb05b5746c6d49305ec7f1f1de6de95b83d` | `90daab8b3379e4d785f8e1fb9cf5f03a9fe5fe6897443a0b4e90fb32f120e428` |
| `broad/captured_write` | 252 | `6f41ef114aabab830c6cb6a7cc85ff74eaa8e16295f2e4c420d715fb21d1c411` | `a3eb12975a63535baaad74fd49cf611f872d6ef2ee44289846e4ecc32f9034de` |
| `broad/many_locals_call` | 162 | `d71e4c9fafefe81b3e8c6a3d305d213e450dc0d7712f6213441450b8eabec412` | `f50e5ee5ab1ec1705cc6ad9bd59f5732b94906c6021249ab312a684a8fd1f93c` |
| `broad/property_read` | 416 | `d13c59ddaad658860f08e97916006f6cb9b6598212a3863c18700f84403f2503` | `89daf87a2a08d16fd43ef8fb9bb3128873335b869a6b0ff4f4e737ba61c14228` |
| `broad/array_read` | 517 | `5ecc5ff2a2a24aad7ee6176511902d7b94fc9dfc9bc7577da8ba656e1288677e` | `c181bdebca6c47062ed2ee4c9e2bf6662e9a45e7da25aac45cef095f59c6d42f` |
| `broad/function_call_two_args` | 66 | `b9bfa692a1637a20fdbd6bffb0b78711b3bb730368d3529cff37c66c8e44f305` | `2be9f240ec36ed33f34ac33409e380fcba5f16c4d8464e88b03ccdd73e5289e5` |
| `broad/function_call_reordered` | 66 | `8db8a625f7c6d17ceb2e9aaa58f176e83b9aa0d921a7df3934f835ce05c942e6` | `773b96c911d51fe88566ab2b0e213e43c29e787835970b29e3307a59ccbe7e43` |
| `broad/top_level_function_call` | 165 | `e425f37fd4abd97d4b19d3b27d6cafadf28f29a12339810741054a7b29268382` | `336a3b0e7ddd64562c170b80564805ab190605e6b7d93d27ea8c828febbff251` |
| `broad/dynamic_method_call` | 484 | `b41e03cbe85fb474a40b04aafaf83b484cf887175539d3ba7827476453311dd6` | `4d1c9ecd605ac362dda7cc8e935e5b4f2e8513fda1f9edba2406b5ae2f8684ab` |
| `broad/local_read` | 6 | `fe54d22a4f5882dbf81a961bb47cd0462a9117ea2a070f297ee897f88f61c084` | `a1c072ddf15503e340581c5e9870bbacc4b73b868425769d281f99ab3fedacb1` |
| `broad/global_read` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `broad/property_dynamic_read` | 325 | `2539ce24cc3be5d820c9438efde76699e2e89e61550c70d7d48aa314ada92573` | `75fcb91d8b77b097d4acabb5b0fe0e4f05bc6edbcd295d5efbfb0a7f8d888ff2` |
| `broad/property_write` | 621 | `83a94c44c34e40ab8d77ea7a7c5c9834946d2b399552782043f8faca0070fb0f` | `58da22c4c0f3c1170633fccfff5d0332603e9238e69cf731167f0feb75325f4f` |
| `broad/array_dynamic_read` | 45 | `05c18afd6bc65df0ea8c9606ff58ac5540b05bdca9954315ba44086caaaa5a80` | `8925af8f6c1388b7d098b1be11a9ad50462bd582f524c3ed4a7758264a0c333c` |
| `broad/array_write` | 686 | `18fafda54f5569f6cb4cfafd420693d4313a5674b67fee08db0de09023bad2ae` | `b9b66b8bb7a9fc14a1eb86f506cf8dc9015d74e05d94f42f69a9a44531faad32` |
| `broad/empty_loop` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `broad/branch_arithmetic` | 23 | `164c6e3caad61c6b1c331aa2ff5b5d1dfb1400c8971d8606ed9668f9de5b68cb` | `2da94371e4ca24f69acc154437d1a416476f2735c8bf5b40dfe64e3406f5ad82` |
| `broad/math_abs` | 37 | `60a7138d78a090ba01855f45a3d6aac3e9702d766996b621cf064a2bf0a9cc46` | `166ebdee89abbeb5016b5745721b57aa333c5bd5c6bae21e56077964757c257e` |
| `broad/array_index_of` | 217 | `8f4aff45c712226f5833ad0d9059f6dbfb277c4f09b09256de6ee2ad4ff4a3f9` | `421dbf8ea8e12f98bc0883b8bfc5d6623811619f477a30bfcd2354c601a44f9a` |
| `broad/string_slice` | 537 | `ea4388e6146ac91eededb9af8d5ed44a45d149750d385459d12381d0c4211c00` | `1ab2c19917154bc675333c0cf8cbd15f3d7783e78853fb62c13714774e157c3c` |
| `broad/object_allocation` | 179 | `fb3ec7d0ee27a1ae1eefe9eaf812b75a82fa3f6d779e45b8a96421915a240b2e` | `f63f30343825c73d5d794ad2cfe567c8bec2e32128aa37e3094cc1d0214a719b` |
| `broad/array_allocation` | 182 | `5b7f7592b65a9e4524869ebc17265049a128c9104a9b92efa3d4f9fc9856ccbf` | `521141667df8dccdc8146f7114b1677aa77286214adf7eab27142355e3a2a98e` |
| `broad/closure_allocation_call` | 318 | `ce420244f77f9fefd4e353f0d948f6837d9305f76741c3326d12933b11dc0926` | `f21e33260774e5caf8789168e8809691d0339982e661eba10970b92f544b7d11` |
| `sentinel/recursive_call_tree` | 1672 | `b1b2d48238b9525eec2feea3be14c188d769f772b8e81e939d1a0c6521df5f20` | `5e5ec3f99456f93765aaba26f4401b3090c29d21117bec5894f69ee25d2f1668` |
| `sentinel/prototype_method_call` | 219 | `14c061a97aa3f9576208efcf6f448a130a92f5f9593ffa9c817a9d03b6ed0ef7` | `815c3f911b1e3d257f61ae381ca0def365a01eb367d6df16d1e7bdfc7f20d4c1` |
| `sentinel/polymorphic_call_site` | 133 | `e17e36b580b7aab38582133bd4ec1eab0483478e1e2c782dc7f444aac707a75a` | `dcfa87a3ea79bae7e6c04f96ef7247e4886bbcb2b0acf12acb6563f13f27dfb1` |
| `sentinel/capturing_closure_call` | 136 | `bbb5491634f3eaf46070800253e8251a2f3b68590ff217c9f289e351cdcdc32a` | `91df5ba292f111217d4dcc870dccc146dec511c15496babcdc77e720e0e2c5dc` |
| `sentinel/heterogeneous_property_read` | 234 | `26320b8082186815ebd8eea1a9ea6c9e7dfe0c77f0c949bbaa6294006c45a41f` | `ae627229dbb6ff023d3e04f61e90475b61761708446b343ff7722f7ada53e585` |
| `sentinel/string_key_map_churn` | 285 | `d0f6e9411418e30d8bcf501b82c8c74a058c273e7693693c9abfd90485500fdf` | `c848f765111b2efe360468a3f9a4a035548d0ec61d68233854c4afec61889946` |
| `external/jetstream3-js-subset/cdjs` | 880 | `161daa24e148559e80b2b15c0dce25204b69a5c070b7aae77a96b0227d2972b3` | `9d732755b92add516ae27953ad185374d1a01a22951705b96318acd8a0893ed7` |
| `external/jetstream3-js-subset/hash-map` | 482 | `17bf6ea5d51d495c49b0eb8236f726f43230e57043ef558e73482d74cfa8b272` | `dcf322e29745b74bd8a361f2274f741e51ce9de29c6e99a0995b26e2e0b13510` |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 319 | `44e236657ebe84f746651585a4598be7f294197bb17f4e92d9cb77dd6c96fb5a` | `97163d3930b48220eb8c03a00ce91dc987b5bb26e9492d3b65d144f9d7e699d5` |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 733 | `ffbf14e2dec9b120a601e929343edc2d083bdde52d1c31e8fe01ae601629bafc` | `f704ef094f7322130f88dc8c135f1f345136550e0224ce26a76e7f18a9a69b2a` |
| `external/kraken-1.1/ai-astar` | 1663 | `ec930468a276dfa1b952e4f0a30fce9553198a41f670175dfbf28f037560031c` | `8d8b1174dcbd7b9f1ffc345b7547cc9dfcb47b1483ea264a160aaeaf70888f5c` |
| `external/kraken-1.1/stanford-crypto-aes` | 860 | `5d94d4be871414f25d5f2911be0ce599cab797fd1b97de86863c7d71bebbb8d1` | `76b72a716b40bb7315bf0aa6e68e1920a30ac946b9acfa23f022d4dd5f7d5d86` |
| `external/sunspider-1.0/3d-raytrace` | 13 | `9df3b63b49e77d37603decf1f767c31828dda0f0e76a6bc609a0477077028c04` | `400cb74d20e377681d7ef2a27886fab71eaf729c303fed241c2e67ae8f99a69a` |
| `external/sunspider-1.0/access-binary-trees` | 2 | `5ccc42bbb1a73a1300fa8c4e000476871c2fa7fba4f9abd35e969fda24c1ca9a` | `5d9b78493247de8a641b1f1b92e323c55cb4231fed840a7009b8b35f8874815d` |
| `external/sunspider-1.0/access-nbody` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `external/sunspider-1.0/crypto-aes` | 21 | `87857ad90d838d1d18240740c774c59f049894a65d14b1b454851d9ec201ad04` | `e0e3f5b107e5f02dd1c69294d4577e871467e5f4849bab382b085d70c4d9b561` |
| `external/sunspider-1.0/crypto-md5` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `external/sunspider-1.0/crypto-sha1` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `external/sunspider-1.0/date-format-tofte` | 87 | `be7be2b548e3108457a3d3dd3e9a07ea884cf7eac73d9509f5cb7acdd7bfa396` | `3f324a8a18949422690b3be08948f08b3ffd44caffc4ba1eeefc183fd13bf813` |
| `external/sunspider-1.0/math-partial-sums` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |
| `external/sunspider-1.0/math-spectral-norm` | unavailable | retained initial no-graph/missing-file artifact | no usable sample claimed |

## Supplementary short-process sampling outcome

Queue job `20261017T074818Z-t056-unit0-short-samples`, source/tooling ddca5bdf, exact same b481894d image. The process-name `sample -wait` collector starts before launching the unmodified program, with three fixed attempts per case. It introduces no JS wrapper, repetition loop or image patch. The job exits 0 after 41 seconds, with four light background samples and no heavy disturbance. All attempts are retained; no performance-gate samples are replaced or pooled.

The 27 attempts yield three usable tofte graphs. The seven initially missing programs remain without a usable graph; the original 3d-raytrace graph remains only 13 main-thread samples. A sampler exit of -15 means the collector terminated an unattached sampler after the original workload had already exited; 255 is a sampler failure. These are absent data, not zero cost. A process-launch profiler or separately justified amplification/profiling protocol is needed to finish this part of attribution without silently changing the program or its execution tier.

| Case | Fixed attempts | Usable new graphs | Main-thread samples per usable graph | Sampler exit statuses |
| --- | ---: | ---: | --- | --- |
| `broad/global_read` | 3 | 0 | unavailable | -15, 255, 255 |
| `broad/empty_loop` | 3 | 0 | unavailable | -15, -15, -15 |
| `external/sunspider-1.0/access-nbody` | 3 | 0 | unavailable | 255, -15, 255 |
| `external/sunspider-1.0/crypto-md5` | 3 | 0 | unavailable | -15, -15, -15 |
| `external/sunspider-1.0/crypto-sha1` | 3 | 0 | unavailable | 255, -15, 255 |
| `external/sunspider-1.0/math-partial-sums` | 3 | 0 | unavailable | 255, 255, 255 |
| `external/sunspider-1.0/math-spectral-norm` | 3 | 0 | unavailable | -15, -15, 255 |
| `external/sunspider-1.0/3d-raytrace` | 3 | 0 | unavailable | 255, 255, 255 |
| `external/sunspider-1.0/date-format-tofte` | 3 | 3 | 38, 37, 56 | 0, 0, 0 |

The three new graphs are sparse and reported separately from the initial 87-sample tofte graph; they are not a precision-qualified latency estimator. The checked cached-image UUID is `4D132B36-EA8A-3C25-9D4A-EBD8ED2B19C2`, matching every usable original/supplemental graph. Image bytes remain SHA-256 c7f0f66a519181b214bdc5ca6404cf428f0d18cbd1ab1d0e26386097619b949e.

| Case / repeat | Samples | Named incl. | Visible prep incl. | Value self | Allocator incl. | Raw SHA-256 | Receipt SHA-256 |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `external/sunspider-1.0/date-format-tofte` / 0 | 38 | 0.00% | 13.16% | 2.63% | 44.74% | `435a8f0f7f421dfd8abb15ac589317a70a22689ca97f91010b02542992edc32c` | `2e5a498a486ff98fcbd07f127684a3b731aa564eac301fd30245d30119619127` |
| `external/sunspider-1.0/date-format-tofte` / 1 | 37 | 2.70% | 13.51% | 8.11% | 29.73% | `be9a80fe29190467fd7dc5456543a176a557a32921a0646058f906976901bbee` | `70f8d1fe4e10fa3990ef420756e9f97e612ea1755fdf0d20ff702b9b4121c6da` |
| `external/sunspider-1.0/date-format-tofte` / 2 | 56 | 1.79% | 16.07% | 8.93% | 25.00% | `75ee3f462c577ca9490341561ccf0491f1165cd7eba5f27283964f8e9f23ab7f` | `d3e8a2e26d6ae1b27ac79084d0fb64f1ccf3e0a9ab75b8d10e12e7ecb7976350` |

## ABI probe source

Standalone qualification source (not runtime implementation), identical on both hosts; SHA-256 `dafaeb106b2f848a8a9442ecb41047fe01c937f2eeeb496eeabc6f5077308f2c`.

```rust
// ABI qualification only; never used for runtime timing.
use std::{mem::{align_of, size_of}, rc::Rc};
#[repr(align(16))]
struct Aligned([u64; 4]);
fn main() {
    let values: Vec<_> = (0..16384u64).map(|i| Box::new(Aligned([i;4]))).collect();
    let mut low = usize::MAX;
    let mut high = 0usize;
    let mut union = 0usize;
    for value in values {
        let original = Box::into_raw(value);
        let address = original.expose_provenance();
        assert_eq!(address >> 48, 0, "allocation must never be truncated");
        assert_eq!(address % align_of::<Aligned>(), 0);
        let word = 0xfff9_0000_0000_0000u64 | address as u64;
        let decoded = (word & 0x0000_ffff_ffff_ffff) as usize;
        let restored = std::ptr::with_exposed_provenance_mut::<Aligned>(decoded);
        assert_eq!(restored, original);
        // SAFETY: original came from Box::into_raw for this exact allocation/type;
        // its provenance was exposed while live, decoded address is unchanged,
        // alignment was checked, and no other owner reconstructs or frees it.
        let value = unsafe { Box::from_raw(restored) };
        assert_eq!(value.0[0], value.0[3]);
        low = low.min(address); high = high.max(address); union |= address;
        drop(value);
    }
    let rc = Rc::new([0u8; 128]);
    println!("arch={} pointer_bytes={} pointer_align={} usize_bytes={} payload_align={} samples=16384 min={:#x} max={:#x} observed_bits={} rc_payload={:#x} exposed_roundtrip=pass", std::env::consts::ARCH, size_of::<*mut ()>(), align_of::<*mut ()>(), size_of::<usize>(), align_of::<Aligned>(), low, high, usize::BITS-union.leading_zeros(), Rc::as_ptr(&rc).addr());
}
```

Gate source: `ddca5bdf`; runtime/build inputs are unchanged from b481894d. The final staged documentation delta passed check-touched.sh --staged --explain, check-context.py and diff whitespace checks before the evidence commit. No project remote push or merge is part of this unit.
