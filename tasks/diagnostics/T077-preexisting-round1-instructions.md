# Round-1 instruction-only replay

Base `4596f149`; candidate runtime-source manifest
`dbfedca78487da4957aed2ad40b7b7d2a12449cb8d7301de74f90c1d3020d0f5`.
Frozen public109 inputs from `tasks/diagnostics/T077-s3c-counts.json`, each
source hash checked. Both builds: `cargo build -p qjs-cli --release --features perf-counters`.
Three AB/BA/AB pairs per input, two workers. Only `instructions retired` is
extracted from `/usr/bin/time -l`; no wall/user/system time is retained or used.
The table contains medians, not timing or performance-acceptance evidence.

Base binary SHA256 `a327b75465651d3c4bb5715650609379e7b1ad8f9b86791bdde8b71c84faaf66`.
Candidate binary SHA256 `5e0ac4780a9443979829cbaeb1be854b6aba5ac0c7fd3aa8fab473a6f18cab49`.

Across the 107 reproducible inputs: instruction geomean **1.001799812**;
sum of medians **601,672,249,250 → 602,455,493,118**.
Largest ratio: `broad/global_read` **1.014241628**.
The two original clock-dependent inputs remain explicitly marked below and
are excluded from that aggregate; fixed-clock counterparts remain included.
All 107 reproducible inputs have identical stdout in all six invocations.

Deterministic semantic-counter replay independently ran two repetitions per
build on all 109 inputs. The same 107 repeatable inputs preserve every counter
and stdout: `executed_ops` 516,234,233 → 516,234,233;
`compact_function_ops` 1,126,742,759 → 1,126,742,759. The two clock-dependent
originals retain differing observations and are not substituted.

| Frozen input | Base median instructions | Candidate median | Ratio |
| --- | ---: | ---: | ---: |
| `development/lodash/transform` | 12,553,653,982 | 12,604,036,054 | 1.004013339 |
| `development/meriyah/parse` | 15,212,450,389 | 15,290,998,157 | 1.005163387 |
| `development/immutable/persistent` | 20,098,145,925 | 20,158,173,250 | 1.002986710 |
| `development/moment/dates` | 47,814,848,903 | 47,874,240,898 | 1.001242124 |
| `development/diff/compare` | 40,266,932,158 | 40,323,880,960 | 1.001414282 |
| `development/papaparse/csv` | 18,364,891,841 | 18,439,240,910 | 1.004048435 |
| `development/js-yaml/document` | 21,802,863,404 | 21,838,790,996 | 1.001647838 |
| `development/big.js/arith` | 21,347,485,286 | 21,341,657,624 | 0.999727009 |
| `development/matter-js/simulate` | 27,256,280,956 | 27,346,075,415 | 1.003294450 |
| `development/graphlib/paths` | 8,064,500,185 | 8,103,629,588 | 1.004852056 |
| `external/jetstream3-js-subset/cdjs` | 8,143,997,139 | 8,184,409,117 | 1.004962180 |
| `external/jetstream3-js-subset/hash-map` | 6,408,168,642 | 6,448,699,049 | 1.006324803 |
| `external/jetstream3-js-subset/gaussian-blur` | 6,922,360,628 | 6,918,068,108 | 0.999379905 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 3,281,176,710 | 3,284,744,069 | 1.001087219 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 6,555,904,342 | 6,582,674,802 | 1.004083412 |
| `external/kraken-1.1/ai-astar` | 29,671,146,784 | 29,682,791,225 | 1.000392450 |
| `external/kraken-1.1/audio-beat-detection` (clock) | 14,157,446,269 | 14,278,103,504 | 1.008522528 |
| `external/kraken-1.1/audio-dft` | 7,352,838,486 | 7,352,263,639 | 0.999921820 |
| `external/kraken-1.1/audio-fft` | 10,145,056,100 | 10,150,083,148 | 1.000495517 |
| `external/kraken-1.1/audio-oscillator` | 5,850,278,615 | 5,854,658,019 | 1.000748580 |
| `external/kraken-1.1/imaging-gaussian-blur` | 112,734,124,378 | 112,678,590,647 | 0.999507392 |
| `external/kraken-1.1/imaging-darkroom` | 14,134,247,107 | 14,132,816,653 | 0.999898795 |
| `external/kraken-1.1/imaging-desaturate` | 18,908,374,205 | 18,925,552,304 | 1.000908492 |
| `external/kraken-1.1/json-parse-financial` | 364,754,978 | 366,851,848 | 1.005748708 |
| `external/kraken-1.1/json-stringify-tinderbox` | 593,758,912 | 597,939,486 | 1.007040861 |
| `external/kraken-1.1/stanford-crypto-aes` | 8,423,663,980 | 8,430,365,221 | 1.000795526 |
| `external/kraken-1.1/stanford-crypto-ccm` | 5,409,241,150 | 5,419,368,221 | 1.001872180 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 12,894,526,984 | 12,906,836,046 | 1.000954596 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 4,190,789,572 | 4,191,992,390 | 1.000287015 |
| `external/sunspider-1.0/3d-cube` | 339,857,743 | 341,103,469 | 1.003665434 |
| `external/sunspider-1.0/3d-morph` | 232,186,803 | 233,312,628 | 1.004848790 |
| `external/sunspider-1.0/3d-raytrace` | 487,555,941 | 490,214,847 | 1.005453540 |
| `external/sunspider-1.0/access-binary-trees` | 288,050,831 | 289,869,251 | 1.006312844 |
| `external/sunspider-1.0/access-fannkuch` | 652,204,725 | 651,612,465 | 0.999091911 |
| `external/sunspider-1.0/access-nbody` | 434,632,770 | 437,172,730 | 1.005843922 |
| `external/sunspider-1.0/access-nsieve` | 293,408,019 | 294,491,057 | 1.003691235 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 177,388,525 | 177,363,540 | 0.999859151 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 206,668,746 | 208,839,360 | 1.010502865 |
| `external/sunspider-1.0/bitops-bitwise-and` | 158,541,580 | 156,993,219 | 0.990233723 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 391,312,096 | 389,817,790 | 0.996181294 |
| `external/sunspider-1.0/controlflow-recursive` | 117,441,652 | 117,588,153 | 1.001247436 |
| `external/sunspider-1.0/crypto-aes` (clock) | 542,769,636 | 542,695,255 | 0.999862960 |
| `external/sunspider-1.0/crypto-md5` | 182,880,785 | 181,905,690 | 0.994668139 |
| `external/sunspider-1.0/crypto-sha1` | 160,315,055 | 160,507,359 | 1.001199538 |
| `external/sunspider-1.0/date-format-tofte` | 705,965,650 | 707,785,962 | 1.002578471 |
| `external/sunspider-1.0/date-format-xparb` | 345,468,779 | 346,291,389 | 1.002381141 |
| `external/sunspider-1.0/math-cordic` | 476,205,510 | 476,540,186 | 1.000702797 |
| `external/sunspider-1.0/math-partial-sums` | 209,621,294 | 207,297,652 | 0.988915048 |
| `external/sunspider-1.0/math-spectral-norm` | 166,802,857 | 166,496,799 | 0.998165151 |
| `external/sunspider-1.0/regexp-dna` | 1,551,803,227 | 1,554,140,284 | 1.001506027 |
| `external/sunspider-1.0/string-base64` | 364,008,977 | 361,737,995 | 0.993761192 |
| `external/sunspider-1.0/string-fasta` | 838,547,948 | 842,070,159 | 1.004200369 |
| `external/sunspider-1.0/string-tagcloud` | 537,717,719 | 538,252,641 | 1.000994801 |
| `external/sunspider-1.0/string-unpack-code` | 737,363,661 | 737,872,560 | 1.000690160 |
| `external/sunspider-1.0/string-validate-input` | 361,369,653 | 362,255,369 | 1.002450997 |
| `broad/plain_function_call` | 86,139,704 | 86,444,164 | 1.003534491 |
| `broad/method_call` | 79,098,671 | 79,542,173 | 1.005606946 |
| `broad/captured_read` | 87,956,310 | 88,134,786 | 1.002029144 |
| `broad/captured_write` | 165,343,473 | 165,956,757 | 1.003709152 |
| `broad/many_locals_call` | 141,861,214 | 142,036,952 | 1.001238802 |
| `broad/property_read` | 388,520,540 | 388,338,907 | 0.999532501 |
| `broad/array_read` | 433,330,676 | 433,593,820 | 1.000607259 |
| `broad/function_call_two_args` | 85,065,473 | 85,494,894 | 1.005048123 |
| `broad/function_call_reordered` | 86,287,608 | 86,297,242 | 1.000111650 |
| `broad/top_level_function_call` | 145,174,362 | 145,026,947 | 0.998984566 |
| `broad/dynamic_method_call` | 459,563,085 | 460,992,311 | 1.003109967 |
| `broad/local_read` | 56,051,558 | 56,258,820 | 1.003697703 |
| `broad/global_read` | 51,876,654 | 52,615,462 | 1.014241628 |
| `broad/property_dynamic_read` | 147,244,526 | 148,841,984 | 1.010849015 |
| `broad/property_write` | 514,415,134 | 512,616,353 | 0.996503250 |
| `broad/array_dynamic_read` | 81,919,093 | 81,928,856 | 1.000119179 |
| `broad/array_write` | 563,776,230 | 564,846,534 | 1.001898455 |
| `broad/empty_loop` | 46,867,781 | 46,942,152 | 1.001586826 |
| `broad/branch_arithmetic` | 66,360,194 | 66,624,677 | 1.003985567 |
| `broad/math_abs` | 70,291,497 | 70,218,417 | 0.998960329 |
| `broad/array_index_of` | 126,337,575 | 126,321,323 | 0.999871361 |
| `broad/string_slice` | 171,928,404 | 172,273,713 | 1.002008446 |
| `broad/object_allocation` | 289,570,388 | 289,685,657 | 1.000398069 |
| `broad/array_allocation` | 292,160,595 | 291,974,261 | 0.999362221 |
| `broad/closure_allocation_call` | 364,747,756 | 363,896,780 | 0.997666947 |
| `sentinel/recursive_call_tree` | 4,426,201,732 | 4,430,287,291 | 1.000923039 |
| `sentinel/prototype_method_call` | 149,639,401 | 149,414,784 | 0.998498945 |
| `sentinel/polymorphic_call_site` | 94,436,827 | 94,672,911 | 1.002499915 |
| `sentinel/capturing_closure_call` | 101,173,501 | 100,827,018 | 0.996575358 |
| `sentinel/heterogeneous_property_read` | 132,026,842 | 132,134,196 | 1.000813123 |
| `sentinel/string_key_map_churn` | 179,958,009 | 180,916,485 | 1.005326109 |
| `diagnostic/audio-fixed-clock` | 11,356,141,648 | 11,355,484,304 | 0.999942116 |
| `diagnostic/aes-fixed-clock` | 542,505,297 | 542,853,367 | 1.000641597 |
| `repro/bitand-global` | 156,275,597 | 157,035,056 | 1.004859741 |
| `repro/bitand-var` | 156,492,366 | 156,416,144 | 0.999512935 |
| `repro/call-global-var-write` | 7,930,090,308 | 7,946,340,675 | 1.002049203 |
| `repro/call-implicit-global-write` | 9,173,111,813 | 9,185,793,371 | 1.001382471 |
| `repro/call-no-global` | 1,015,713,168 | 1,015,554,196 | 0.999843487 |
| `repro/closed-form-call` | 1,014,415,590 | 1,015,771,690 | 1.001336829 |
| `repro/compound-global-write` | 6,982,337,293 | 6,983,964,633 | 1.000233065 |
| `repro/eval-closure` | 4,990,677,322 | 4,984,242,220 | 0.998710575 |
| `repro/eval-parent-closure` | 2,178,146,897 | 2,178,695,617 | 1.000251921 |
| `repro/global-lexical-write` | 3,664,934,086 | 3,667,871,487 | 1.000801488 |
| `repro/partial-global` | 125,854,732 | 126,115,291 | 1.002070315 |
| `repro/partial-local` | 124,912,033 | 125,296,686 | 1.003079391 |
| `repro/strict-global-var-write` | 10,563,503,465 | 10,555,828,914 | 0.999273484 |
| `repro/upw0` | 987,617,381 | 997,609,456 | 1.010117354 |
| `repro/upw1` | 1,264,736,749 | 1,276,081,456 | 1.008970015 |
| `repro/upw2` | 1,531,706,601 | 1,543,618,929 | 1.007777160 |
| `repro/upw4` | 2,067,638,098 | 2,083,294,499 | 1.007572119 |
| `repro/validate-like` | 1,815,529,043 | 1,817,047,802 | 1.000836538 |
| `repro/vm-ops-local` | 5,857,936,286 | 5,859,181,695 | 1.000212602 |
| `repro/vm-ops-plain` | 3,025,984,930 | 3,027,322,513 | 1.000442032 |
| `repro/vm-ops` | 9,876,881,447 | 9,897,956,092 | 1.002133735 |
