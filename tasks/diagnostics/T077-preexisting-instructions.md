# Pre-existing fixes: native instruction replay

Frozen S3c public receipt: 109 inputs, source hashes verified against
`tasks/diagnostics/T077-s3c-counts.json`; base `49fba8b6`, candidate runtime
manifest `f000c96b56ba1b814e0265edc61be85a359d34a1f8bbdff915d7a7d51e929aa8`.
Both binaries use `cargo build -p qjs-cli --release --features perf-counters`.
Three instruction-only AB/BA/AB pairs per input, two workers; only the
`instructions retired` field of `/usr/bin/time -l` is extracted. No wall,
user or system time is retained or used. Medians below are native instructions,
not bytecode operations. This is diagnostic evidence, not timing acceptance.

Counter binary SHA256: base `55fb193a6d44d33394ea515f7ced0e7bd61ab2b072bba8bb4b5e00d1c787307a`,
candidate `a327b75465651d3c4bb5715650609379e7b1ad8f9b86791bdde8b71c84faaf66`.

Across 107 reproducible inputs, geomean after/before **1.000295450**.
Sum of input medians: **601,214,251,825 → 601,092,508,854**.
Largest ratio: `sentinel/capturing_closure_call` **1.013171926**.
The two original clock-dependent inputs remain visible with a clock marker;
they are excluded only from this reproducible-input aggregate. Their frozen
fixed-clock diagnostic counterparts are included. All 107 reproducible inputs
retain the same stdout across all six invocations.

| Lane | Instruction geomean |
| --- | ---: |
| development | 0.999935241 |
| external | 1.000177024 |
| broad | 1.000420444 |
| sentinel | 1.001792788 |
| diagnostic | 1.000443171 |
| repro | 1.000119179 |

| Frozen input | Base median | Candidate median | Ratio |
| --- | ---: | ---: | ---: |
| `development/lodash/transform` | 12,492,647,193 | 12,496,752,530 | 1.000328620 |
| `development/meriyah/parse` | 15,159,955,514 | 15,174,129,713 | 1.000934976 |
| `development/immutable/persistent` | 20,023,171,997 | 20,021,845,193 | 0.999933737 |
| `development/moment/dates` | 47,675,267,003 | 47,665,294,191 | 0.999790818 |
| `development/diff/compare` | 40,163,092,934 | 40,156,812,740 | 0.999843633 |
| `development/papaparse/csv` | 18,346,911,934 | 18,334,568,570 | 0.999327224 |
| `development/js-yaml/document` | 21,781,360,080 | 21,786,122,349 | 1.000218640 |
| `development/big.js/arith` | 21,326,998,476 | 21,317,823,024 | 0.999569773 |
| `development/matter-js/simulate` | 27,201,150,002 | 27,195,526,627 | 0.999793267 |
| `development/graphlib/paths` | 8,039,668,065 | 8,036,554,014 | 0.999612664 |
| `external/jetstream3-js-subset/cdjs` | 8,128,314,007 | 8,126,961,714 | 0.999833632 |
| `external/jetstream3-js-subset/hash-map` | 6,406,891,251 | 6,409,983,507 | 1.000482645 |
| `external/jetstream3-js-subset/gaussian-blur` | 6,919,764,945 | 6,922,012,919 | 1.000324863 |
| `external/jetstream3-js-subset/stanford-crypto-aes` | 3,277,721,953 | 3,278,743,959 | 1.000311804 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 6,546,420,364 | 6,555,294,606 | 1.001355587 |
| `external/kraken-1.1/ai-astar` | 29,690,387,462 | 29,700,260,886 | 1.000332546 |
| `external/kraken-1.1/audio-beat-detection` (clock) | 14,249,845,261 | 14,145,080,828 | 0.992648030 |
| `external/kraken-1.1/audio-dft` | 7,355,624,691 | 7,354,351,781 | 0.999826947 |
| `external/kraken-1.1/audio-fft` | 10,152,142,463 | 10,146,774,123 | 0.999471211 |
| `external/kraken-1.1/audio-oscillator` | 5,854,918,008 | 5,853,609,135 | 0.999776449 |
| `external/kraken-1.1/imaging-gaussian-blur` | 112,744,795,735 | 112,677,135,526 | 0.999399882 |
| `external/kraken-1.1/imaging-darkroom` | 14,198,873,649 | 14,179,896,293 | 0.998663460 |
| `external/kraken-1.1/imaging-desaturate` | 18,918,904,389 | 18,868,187,134 | 0.997319229 |
| `external/kraken-1.1/json-parse-financial` | 364,019,923 | 365,347,885 | 1.003648048 |
| `external/kraken-1.1/json-stringify-tinderbox` | 594,519,648 | 594,746,202 | 1.000381071 |
| `external/kraken-1.1/stanford-crypto-aes` | 8,414,215,084 | 8,413,467,590 | 0.999911163 |
| `external/kraken-1.1/stanford-crypto-ccm` | 5,408,760,671 | 5,414,064,266 | 1.000980556 |
| `external/kraken-1.1/stanford-crypto-pbkdf2` | 12,901,522,424 | 12,911,463,072 | 1.000770502 |
| `external/kraken-1.1/stanford-crypto-sha256-iterative` | 4,192,313,526 | 4,192,530,553 | 1.000051768 |
| `external/sunspider-1.0/3d-cube` | 339,239,737 | 338,974,169 | 0.999217167 |
| `external/sunspider-1.0/3d-morph` | 231,793,602 | 231,669,903 | 0.999466340 |
| `external/sunspider-1.0/3d-raytrace` | 486,599,520 | 486,743,701 | 1.000296303 |
| `external/sunspider-1.0/access-binary-trees` | 287,867,519 | 288,471,809 | 1.002099195 |
| `external/sunspider-1.0/access-fannkuch` | 650,384,911 | 650,163,798 | 0.999660027 |
| `external/sunspider-1.0/access-nbody` | 433,921,360 | 434,571,584 | 1.001498484 |
| `external/sunspider-1.0/access-nsieve` | 293,421,009 | 293,720,425 | 1.001020431 |
| `external/sunspider-1.0/bitops-3bit-bits-in-byte` | 177,111,876 | 177,139,544 | 1.000156218 |
| `external/sunspider-1.0/bitops-bits-in-byte` | 206,400,011 | 206,955,842 | 1.002692980 |
| `external/sunspider-1.0/bitops-bitwise-and` | 156,754,098 | 157,070,607 | 1.002019143 |
| `external/sunspider-1.0/bitops-nsieve-bits` | 390,256,001 | 389,949,698 | 0.999215123 |
| `external/sunspider-1.0/controlflow-recursive` | 116,211,385 | 116,422,410 | 1.001815872 |
| `external/sunspider-1.0/crypto-aes` (clock) | 541,648,997 | 542,870,777 | 1.002255667 |
| `external/sunspider-1.0/crypto-md5` | 181,864,706 | 182,566,905 | 1.003861107 |
| `external/sunspider-1.0/crypto-sha1` | 160,763,330 | 160,173,568 | 0.996331489 |
| `external/sunspider-1.0/date-format-tofte` | 705,575,140 | 705,970,127 | 1.000559809 |
| `external/sunspider-1.0/date-format-xparb` | 345,711,004 | 345,358,951 | 0.998981655 |
| `external/sunspider-1.0/math-cordic` | 476,079,133 | 475,677,762 | 0.999156924 |
| `external/sunspider-1.0/math-partial-sums` | 207,677,549 | 207,145,483 | 0.997438019 |
| `external/sunspider-1.0/math-spectral-norm` | 166,290,932 | 166,389,627 | 1.000593508 |
| `external/sunspider-1.0/regexp-dna` | 1,554,882,017 | 1,553,274,667 | 0.998966256 |
| `external/sunspider-1.0/string-base64` | 361,158,502 | 361,767,155 | 1.001685279 |
| `external/sunspider-1.0/string-fasta` | 837,747,563 | 837,712,937 | 0.999958668 |
| `external/sunspider-1.0/string-tagcloud` | 535,737,787 | 536,041,340 | 1.000566607 |
| `external/sunspider-1.0/string-unpack-code` | 736,927,797 | 736,121,837 | 0.998906324 |
| `external/sunspider-1.0/string-validate-input` | 361,569,473 | 361,083,143 | 0.998654947 |
| `broad/plain_function_call` | 86,014,645 | 86,103,227 | 1.001029848 |
| `broad/method_call` | 79,083,073 | 79,299,082 | 1.002731419 |
| `broad/captured_read` | 88,146,387 | 88,060,141 | 0.999021559 |
| `broad/captured_write` | 164,724,890 | 164,517,006 | 0.998737993 |
| `broad/many_locals_call` | 141,983,437 | 142,001,246 | 1.000125430 |
| `broad/property_read` | 388,879,472 | 388,319,467 | 0.998559952 |
| `broad/array_read` | 434,353,881 | 433,386,999 | 0.997773976 |
| `broad/function_call_two_args` | 84,981,612 | 85,291,114 | 1.003641988 |
| `broad/function_call_reordered` | 86,296,425 | 86,238,792 | 0.999332151 |
| `broad/top_level_function_call` | 145,325,096 | 146,595,002 | 1.008738381 |
| `broad/dynamic_method_call` | 460,175,318 | 459,662,375 | 0.998885331 |
| `broad/local_read` | 56,205,411 | 56,036,072 | 0.996987141 |
| `broad/global_read` | 51,445,232 | 51,504,307 | 1.001148309 |
| `broad/property_dynamic_read` | 147,389,827 | 148,619,306 | 1.008341682 |
| `broad/property_write` | 512,974,267 | 513,069,800 | 1.000186234 |
| `broad/array_dynamic_read` | 82,210,107 | 82,079,466 | 0.998410889 |
| `broad/array_write` | 563,578,317 | 564,173,186 | 1.001055521 |
| `broad/empty_loop` | 46,928,446 | 46,875,001 | 0.998861139 |
| `broad/branch_arithmetic` | 66,665,443 | 67,130,835 | 1.006981008 |
| `broad/math_abs` | 70,194,397 | 70,085,652 | 0.998450802 |
| `broad/array_index_of` | 126,446,970 | 126,141,461 | 0.997583896 |
| `broad/string_slice` | 172,334,986 | 172,356,293 | 1.000123637 |
| `broad/object_allocation` | 291,582,827 | 289,737,040 | 0.993669768 |
| `broad/array_allocation` | 292,194,927 | 292,704,290 | 1.001743230 |
| `broad/closure_allocation_call` | 364,334,981 | 363,801,404 | 0.998535477 |
| `sentinel/recursive_call_tree` | 4,430,780,026 | 4,426,651,241 | 0.999068158 |
| `sentinel/prototype_method_call` | 148,899,035 | 148,900,482 | 1.000009718 |
| `sentinel/polymorphic_call_site` | 93,860,464 | 94,159,488 | 1.003185836 |
| `sentinel/capturing_closure_call` | 100,593,264 | 101,918,271 | 1.013171926 |
| `sentinel/heterogeneous_property_read` | 132,323,732 | 131,980,218 | 0.997403988 |
| `sentinel/string_key_map_churn` | 179,873,940 | 179,514,972 | 0.998004336 |
| `diagnostic/audio-fixed-clock` | 11,358,772,246 | 11,364,583,222 | 1.000511585 |
| `diagnostic/aes-fixed-clock` | 541,625,363 | 541,828,343 | 1.000374761 |
| `repro/bitand-global` | 156,395,104 | 156,288,250 | 0.999316769 |
| `repro/bitand-var` | 156,961,822 | 156,271,633 | 0.995602822 |
| `repro/call-global-var-write` | 7,937,944,434 | 7,936,449,725 | 0.999811701 |
| `repro/call-implicit-global-write` | 9,183,269,791 | 9,186,596,093 | 1.000362213 |
| `repro/call-no-global` | 1,015,378,754 | 1,015,263,385 | 0.999886378 |
| `repro/closed-form-call` | 1,014,813,340 | 1,015,630,392 | 1.000805125 |
| `repro/compound-global-write` | 6,982,115,106 | 6,985,641,469 | 1.000505057 |
| `repro/eval-closure` | 4,990,985,524 | 4,995,670,562 | 1.000938700 |
| `repro/eval-parent-closure` | 2,178,202,737 | 2,178,985,685 | 1.000359447 |
| `repro/global-lexical-write` | 3,667,034,683 | 3,665,488,162 | 0.999578264 |
| `repro/partial-global` | 125,267,170 | 125,303,403 | 1.000289246 |
| `repro/partial-local` | 124,780,393 | 125,476,405 | 1.005577896 |
| `repro/strict-global-var-write` | 10,565,737,025 | 10,563,670,480 | 0.999804411 |
| `repro/upw0` | 988,681,654 | 988,314,549 | 0.999628692 |
| `repro/upw1` | 1,266,383,603 | 1,267,045,795 | 1.000522900 |
| `repro/upw2` | 1,533,614,980 | 1,532,961,984 | 0.999574211 |
| `repro/upw4` | 2,068,307,591 | 2,067,915,589 | 0.999810472 |
| `repro/validate-like` | 1,816,785,667 | 1,817,389,472 | 1.000332348 |
| `repro/vm-ops-local` | 5,861,777,249 | 5,861,874,928 | 1.000016664 |
| `repro/vm-ops-plain` | 3,028,129,722 | 3,027,641,281 | 0.999838699 |
| `repro/vm-ops` | 9,878,227,679 | 9,877,907,717 | 0.999967609 |
