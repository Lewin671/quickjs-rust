# Round-2 instruction-only replay

Base `a2b51693` (the round-2 starting point); candidate `e3db17fd`, runtime-source
manifest `d257714fbe6e13ec48c9e7150a9820f45eb3d451f6c86dc0c8a6a4098bbfbe01`
(SHA256 of the sorted JSON mapping every runtime `.rs` path to its SHA256).
Both are plain `cargo build --release -p qjs-cli` builds of `git archive`
exports at equal-length paths, sharing one scratch target directory. Frozen
public109 inputs from `tasks/diagnostics/T077-s3c-counts.json`, each source
hash checked. Three AB/BA/AB pairs per input, two workers. Only
`instructions retired` is extracted from `/usr/bin/time -l`; no wall, user or
system time is retained or used. This is not timing or performance-acceptance
evidence.

Base binary SHA256 `7006c230f0637b7b4648c7f9d8fb37ffb85b678440a1147e17cc571fccf8d10a`.
Candidate binary SHA256 `4d24423efd6c2e038575b84bc0e36b915bd0cf00c2f354f4d7fd7d718206da0d`.

Across the 107 reproducible inputs: instruction geomean **0.973595871**;
sum of medians **537,502,946,775 → 526,721,613,978**. All 107 have identical stdout in all six
invocations. The two clock-dependent originals (`kraken-1.1/audio-beat-detection`,
`sunspider-1.0/crypto-aes`) are listed but excluded from the aggregate; their
fixed-clock counterparts are included. The largest within-build spread of the
three runs of one input is 1.026 (`broad/math_abs`).

Ratios above 1.005: `broad/many_locals_call`, `sentinel/prototype_method_call`
and `kraken-1.1/audio-dft`. The first is attributed in the
[round-2 record](T077-preexisting-round2.md#instruction-counts) to code layout,
not to completion work: its execution counters are identical and the sign of
the per-iteration change flips with the number of locals.

| Frozen input | Base median instructions | Candidate median | Ratio |
| --- | ---: | ---: | ---: |
| `development/lodash/transform` | 11,581,232,160 | 11,251,785,818 | 0.971553429 |
| `development/meriyah/parse` | 13,471,740,491 | 12,987,048,758 | 0.964021595 |
| `development/immutable/persistent` | 17,967,445,275 | 17,189,090,437 | 0.956679716 |
| `development/moment/dates` | 42,446,848,251 | 41,482,851,939 | 0.977289331 |
| `development/diff/compare` | 35,674,587,800 | 34,618,034,377 | 0.970383584 |
| `development/papaparse/csv` | 17,109,530,104 | 16,804,133,674 | 0.982150507 |
| `development/js-yaml/document` | 19,841,646,714 | 18,862,403,589 | 0.950647084 |
| `development/big.js/arith` | 19,959,054,691 | 19,829,885,575 | 0.993528295 |
| `development/matter-js/simulate` | 22,449,972,268 | 21,594,044,504 | 0.961873995 |
| `development/graphlib/paths` | 7,321,041,419 | 7,184,355,453 | 0.981329710 |
| `external/jetstream3-js-subset/cdjs` | 7,318,050,410 | 7,016,284,713 | 0.958764195 |
| `external/jetstream3-js-subset/hash-map` | 5,842,257,040 | 5,742,931,389 | 0.982998754 |
| `external/jetstream3-js-subset/gaussian-blur` | 6,214,920,680 | 6,193,479,588 | 0.996550062 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 3,001,149,801 | 2,957,862,153 | 0.985576312 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 6,005,464,505 | 5,900,464,290 | 0.982515888 |
| `external/kraken-1.1/ai-astar` | 26,699,379,435 | 26,500,850,724 | 0.992564295 |
| `external/kraken-1.1/audio-beat-detection` (clock-dependent, excluded) | 16,382,806,183 | 16,216,603,663 | 0.989855064 |
| `external/kraken-1.1/audio-dft` | 7,105,907,392 | 7,146,964,834 | 1.005777931 |
| `external/kraken-1.1/audio-fft` | 9,997,866,515 | 9,933,067,423 | 0.993518708 |
| `external/kraken-1.1/audio-oscillator` | 5,828,799,191 | 5,555,542,982 | 0.953119639 |
| `external/kraken-1.1/imaging-gaussian-blur` | 103,499,886,314 | 102,839,066,554 | 0.993615261 |
| `external/kraken-1.1/imaging-darkroom` | 13,779,243,941 | 13,524,626,304 | 0.981521654 |
| `external/kraken-1.1/imaging-desaturate` | 18,850,115,697 | 18,612,419,115 | 0.987390179 |
| `external/kraken-1.1/json-parse-financial` | 357,234,899 | 355,935,404 | 0.996362351 |
| `external/kraken-1.1/json-stringify-tinderbox` | 586,795,866 | 586,855,298 | 1.000101282 |
| `external/kraken-1.1/stanford-crypto-aes` | 7,702,033,935 | 7,578,396,552 | 0.983947437 |
| `external/kraken-1.1/stanford-crypto-ccm` | 4,854,089,959 | 4,773,233,102 | 0.983342530 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 12,131,447,612 | 11,781,784,953 | 0.971177169 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 3,997,498,233 | 3,901,821,336 | 0.976065806 |
| `external/sunspider-1.0/3d-cube` | 308,995,716 | 299,584,223 | 0.969541672 |
| `external/sunspider-1.0/3d-morph` | 222,363,098 | 212,786,516 | 0.956932683 |
| `external/sunspider-1.0/3d-raytrace` | 439,216,330 | 432,861,633 | 0.985531738 |
| `external/sunspider-1.0/access-binary-trees` | 269,951,186 | 266,347,935 | 0.986652213 |
| `external/sunspider-1.0/access-fannkuch` | 628,655,264 | 617,879,448 | 0.982858943 |
| `external/sunspider-1.0/access-nbody` | 343,742,712 | 338,537,799 | 0.984858114 |
| `external/sunspider-1.0/access-nsieve` | 285,675,059 | 281,069,580 | 0.983878610 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 170,394,314 | 169,053,725 | 0.992132431 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 202,322,751 | 200,545,478 | 0.991215654 |
| `external/sunspider-1.0/bitops-bitwise-and` | 148,744,473 | 148,716,862 | 0.999814373 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 344,449,546 | 332,085,076 | 0.964103683 |
| `external/sunspider-1.0/controlflow-recursive` | 111,347,840 | 107,981,795 | 0.969770002 |
| `external/sunspider-1.0/crypto-aes` (clock-dependent, excluded) | 471,132,739 | 467,829,541 | 0.992988817 |
| `external/sunspider-1.0/crypto-md5` | 173,998,496 | 173,089,020 | 0.994773081 |
| `external/sunspider-1.0/crypto-sha1` | 153,859,729 | 152,643,751 | 0.992096840 |
| `external/sunspider-1.0/date-format-tofte` | 647,077,804 | 607,131,576 | 0.938266731 |
| `external/sunspider-1.0/date-format-xparb` | 317,274,169 | 307,821,812 | 0.970207606 |
| `external/sunspider-1.0/math-cordic` | 378,203,049 | 365,489,224 | 0.966383600 |
| `external/sunspider-1.0/math-partial-sums` | 196,307,562 | 196,002,665 | 0.998446840 |
| `external/sunspider-1.0/math-spectral-norm` | 158,201,617 | 156,621,347 | 0.990011038 |
| `external/sunspider-1.0/regexp-dna` | 1,546,637,320 | 1,546,239,557 | 0.999742821 |
| `external/sunspider-1.0/string-base64` | 318,290,005 | 308,153,115 | 0.968152032 |
| `external/sunspider-1.0/string-fasta` | 787,897,744 | 752,246,374 | 0.954751273 |
| `external/sunspider-1.0/string-tagcloud` | 505,009,352 | 499,613,846 | 0.989316028 |
| `external/sunspider-1.0/string-unpack-code` | 691,551,654 | 691,864,984 | 1.000453083 |
| `external/sunspider-1.0/string-validate-input` | 329,157,073 | 321,050,164 | 0.975370698 |
| `broad/plain_function_call` | 79,865,309 | 78,442,843 | 0.982189188 |
| `broad/method_call` | 73,833,758 | 72,760,926 | 0.985469628 |
| `broad/captured_read` | 82,154,655 | 80,194,147 | 0.976136374 |
| `broad/captured_write` | 152,550,965 | 145,013,886 | 0.950593043 |
| `broad/many_locals_call` | 128,183,276 | 134,191,361 | 1.046871052 |
| `broad/property_read` | 227,971,192 | 208,612,612 | 0.915083218 |
| `broad/array_read` | 268,761,609 | 249,622,424 | 0.928787504 |
| `broad/function_call_two_args` | 79,404,268 | 77,536,438 | 0.976476957 |
| `broad/function_call_reordered` | 79,750,146 | 78,388,679 | 0.982928345 |
| `broad/top_level_function_call` | 138,207,218 | 131,958,147 | 0.954784771 |
| `broad/dynamic_method_call` | 268,699,272 | 254,297,564 | 0.946402132 |
| `broad/local_read` | 52,441,578 | 50,390,023 | 0.960879228 |
| `broad/global_read` | 48,044,809 | 46,168,702 | 0.960950891 |
| `broad/property_dynamic_read` | 132,748,475 | 127,287,186 | 0.958859874 |
| `broad/property_write` | 336,948,911 | 308,173,747 | 0.914600810 |
| `broad/array_dynamic_read` | 79,217,510 | 78,192,260 | 0.987057786 |
| `broad/array_write` | 381,709,993 | 349,696,395 | 0.916131098 |
| `broad/empty_loop` | 43,598,606 | 41,954,121 | 0.962281248 |
| `broad/branch_arithmetic` | 62,383,094 | 59,801,186 | 0.958612056 |
| `broad/math_abs` | 66,620,299 | 64,688,996 | 0.971010292 |
| `broad/array_index_of` | 120,396,306 | 118,859,049 | 0.987231693 |
| `broad/string_slice` | 168,002,210 | 166,626,003 | 0.991808399 |
| `broad/object_allocation` | 138,143,819 | 118,923,263 | 0.860865610 |
| `broad/array_allocation` | 140,312,379 | 121,104,993 | 0.863109826 |
| `broad/closure_allocation_call` | 203,871,597 | 184,189,977 | 0.903460706 |
| `sentinel/recursive_call_tree` | 4,418,393,219 | 4,327,920,389 | 0.979523590 |
| `sentinel/prototype_method_call` | 132,928,869 | 134,257,364 | 1.009994029 |
| `sentinel/polymorphic_call_site` | 87,775,253 | 86,382,716 | 0.984135198 |
| `sentinel/capturing_closure_call` | 94,200,488 | 92,722,242 | 0.984307449 |
| `sentinel/heterogeneous_property_read` | 114,127,639 | 112,263,803 | 0.983668846 |
| `sentinel/string_key_map_churn` | 170,262,541 | 169,800,701 | 0.997287483 |
| `diagnostic/audio-fixed-clock` | 11,065,362,993 | 10,952,085,713 | 0.989762895 |
| `diagnostic/aes-fixed-clock` | 471,405,738 | 468,020,871 | 0.992819631 |
| `repro/bitand-global` | 148,428,551 | 148,419,434 | 0.999938577 |
| `repro/bitand-var` | 148,490,635 | 148,441,531 | 0.999669312 |
| `repro/call-global-var-write` | 6,051,801,954 | 6,060,774,831 | 1.001482679 |
| `repro/call-implicit-global-write` | 7,223,007,828 | 7,232,143,099 | 1.001264746 |
| `repro/call-no-global` | 980,841,831 | 981,178,280 | 1.000343021 |
| `repro/closed-form-call` | 957,207,947 | 937,116,178 | 0.979010027 |
| `repro/compound-global-write` | 5,173,304,703 | 4,956,882,998 | 0.958165676 |
| `repro/eval-closure` | 3,882,525,846 | 3,897,342,992 | 1.003816368 |
| `repro/eval-parent-closure` | 1,608,464,416 | 1,610,124,674 | 1.001032201 |
| `repro/global-lexical-write` | 2,701,657,263 | 2,478,789,470 | 0.917507007 |
| `repro/partial-global` | 117,973,729 | 118,154,199 | 1.001529747 |
| `repro/partial-local` | 117,788,682 | 112,651,030 | 0.956382465 |
| `repro/strict-global-var-write` | 7,125,846,441 | 7,155,260,535 | 1.004127804 |
| `repro/upw0` | 926,197,753 | 905,224,631 | 0.977355676 |
| `repro/upw1` | 1,175,173,108 | 1,154,103,582 | 0.982071130 |
| `repro/upw2` | 1,414,571,748 | 1,393,182,593 | 0.984879413 |
| `repro/upw4` | 1,892,441,139 | 1,871,211,771 | 0.988782019 |
| `repro/validate-like` | 1,158,138,472 | 1,127,084,680 | 0.973186460 |
| `repro/vm-ops-local` | 4,021,215,965 | 3,897,265,298 | 0.969175824 |
| `repro/vm-ops-plain` | 1,484,872,228 | 1,303,175,661 | 0.877634881 |
| `repro/vm-ops` | 7,810,154,081 | 7,351,919,441 | 0.941328348 |
