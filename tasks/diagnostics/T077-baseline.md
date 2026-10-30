# T077 rebuild baseline (lead receipt)

Job `20261030T045926Z-core-rebuild-baseline`, submitted by the lead 2026-10-30 UTC
through `imac-bench.sh compare --base 6663738f --candidate 6663738f --blocks 30`
(workers-4, iMac x86_64, state `success`, no measurement issues, one light
background sample below the disturbance threshold, 2075 s). Base and candidate
are the same binary, so the A/A column is the noise check.

- Source `6663738f000135120f0217f6307859d5e13e750b`; Rust binary SHA-256
  `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- QuickJS-NG `f7830186043e4488f2998759d60a514faf07cbc9`, binary
  `919f7235f61412ba69faf6aa05b534869aa7d69d1583b3d7aab3cd5b67cd6f40`.
- Bundle `report.json` `09b8708d…`, development manifest `2f35fddc…`, external
  manifest `740f0ca7…`, sentinel manifest `9477a8d5…` (full hashes in the job's
  `out/bundle/report.json` on the runner and in `outputs/imac-bench/` locally).
- Runtime/build inputs are unchanged through `d19d3aa6` (docs-only commits since).

| Lane | Cases | Rust/NG geomean | A/A geomean |
| --- | ---: | ---: | ---: |
| development | 10 | **2.632** | 1.0002 |
| external | 45 | 0.911 | 1.0005 |
| selection (broad) | 25 | 0.971 | 1.0001 |
| sentinel | 6 | 1.046 | — |

Per-case medians (Rust/NG wall ratio; A/A = candidate/base of the same binary):

| Lane | Case | Rust/NG | A/A |
| --- | --- | ---: | ---: |
| development | lodash/transform | 2.041 | 0.9968 |
| development | meriyah/parse | 2.992 | 1.0038 |
| development | immutable/persistent | 3.136 | 1.0027 |
| development | moment/dates | 3.276 | 0.9986 |
| development | diff/compare | 2.859 | 1.0035 |
| development | papaparse/csv | 1.861 | 1.0026 |
| development | js-yaml/document | 3.687 | 1.0020 |
| development | big.js/arith | 2.390 | 0.9978 |
| development | matter-js/simulate | 2.204 | 0.9993 |
| development | graphlib/paths | 2.465 | 0.9949 |
| external | jetstream3-js-subset/cdjs | 1.218 | 1.0014 |
| external | jetstream3-js-subset/hash-map | 1.480 | 1.0016 |
| external | jetstream3-js-subset/gaussian-blur | 0.603 | 1.0001 |
| external | jetstream3-js-subset/stanford-crypto-aes | 1.467 | 0.9979 |
| external | jetstream3-js-subset/raytrace-public-class-fields | 0.926 | 0.9954 |
| external | kraken-1.1/ai-astar | 1.249 | 0.9985 |
| external | kraken-1.1/audio-beat-detection | 0.780 | 0.9979 |
| external | kraken-1.1/audio-dft | 0.524 | 1.0008 |
| external | kraken-1.1/audio-fft | 0.557 | 1.0015 |
| external | kraken-1.1/audio-oscillator | 0.432 | 1.0024 |
| external | kraken-1.1/imaging-gaussian-blur | 1.039 | 0.9992 |
| external | kraken-1.1/imaging-darkroom | 0.825 | 0.9986 |
| external | kraken-1.1/imaging-desaturate | 0.543 | 1.0004 |
| external | kraken-1.1/json-parse-financial | 0.220 | 0.9972 |
| external | kraken-1.1/json-stringify-tinderbox | 0.364 | 0.9957 |
| external | kraken-1.1/stanford-crypto-aes | 1.489 | 0.9977 |
| external | kraken-1.1/stanford-crypto-ccm | 1.237 | 0.9977 |
| external | kraken-1.1/stanford-crypto-pbkdf2 | 1.039 | 1.0026 |
| external | kraken-1.1/stanford-crypto-sha256-iterative | 1.096 | 0.9962 |
| external | sunspider-1.0/3d-cube | 0.883 | 0.9979 |
| external | sunspider-1.0/3d-morph | 0.743 | 1.0008 |
| external | sunspider-1.0/3d-raytrace | 1.591 | 0.9997 |
| external | sunspider-1.0/access-binary-trees | 1.045 | 0.9981 |
| external | sunspider-1.0/access-fannkuch | 0.827 | 0.9985 |
| external | sunspider-1.0/access-nbody | 1.561 | 1.0040 |
| external | sunspider-1.0/access-nsieve | 0.626 | 1.0009 |
| external | sunspider-1.0/bitops-3bit-bits-in-byte | 1.163 | 1.0017 |
| external | sunspider-1.0/bitops-bits-in-byte | 0.951 | 1.0009 |
| external | sunspider-1.0/bitops-bitwise-and | 0.604 | 0.9998 |
| external | sunspider-1.0/bitops-nsieve-bits | 1.381 | 1.0044 |
| external | sunspider-1.0/controlflow-recursive | 0.910 | 1.0093 |
| external | sunspider-1.0/crypto-aes | 1.188 | 1.0006 |
| external | sunspider-1.0/crypto-md5 | 1.601 | 1.0006 |
| external | sunspider-1.0/crypto-sha1 | 1.372 | 1.0004 |
| external | sunspider-1.0/date-format-tofte | 1.277 | 0.9987 |
| external | sunspider-1.0/date-format-xparb | 1.111 | 0.9984 |
| external | sunspider-1.0/math-cordic | 1.081 | 1.0017 |
| external | sunspider-1.0/math-partial-sums | 1.266 | 1.0080 |
| external | sunspider-1.0/math-spectral-norm | 1.041 | 1.0068 |
| external | sunspider-1.0/regexp-dna | 0.982 | 1.0024 |
| external | sunspider-1.0/string-base64 | 1.013 | 1.0013 |
| external | sunspider-1.0/string-fasta | 0.671 | 1.0011 |
| external | sunspider-1.0/string-tagcloud | 0.887 | 0.9985 |
| external | sunspider-1.0/string-unpack-code | 0.450 | 1.0067 |
| external | sunspider-1.0/string-validate-input | 1.029 | 0.9990 |
| selection | array_allocation | 0.271 | 1.0003 |
| selection | array_dynamic_read | 0.441 | 1.0031 |
| selection | array_index_of | 1.420 | 1.0055 |
| selection | array_read | 3.283 | 1.0022 |
| selection | array_write | 1.528 | 0.9989 |
| selection | branch_arithmetic | 1.018 | 0.9991 |
| selection | captured_read | 0.756 | 1.0037 |
| selection | captured_write | 2.211 | 1.0010 |
| selection | closure_allocation_call | 0.268 | 0.9973 |
| selection | dynamic_method_call | 3.088 | 0.9971 |
| selection | empty_loop | 0.811 | 1.0002 |
| selection | function_call_reordered | 0.741 | 1.0024 |
| selection | function_call_two_args | 0.726 | 0.9999 |
| selection | global_read | 0.828 | 0.9993 |
| selection | local_read | 0.809 | 1.0009 |
| selection | many_locals_call | 0.399 | 1.0001 |
| selection | math_abs | 0.580 | 0.9975 |
| selection | method_call | 0.974 | 1.0037 |
| selection | object_allocation | 0.234 | 0.9971 |
| selection | plain_function_call | 0.735 | 1.0020 |
| selection | property_dynamic_read | 1.348 | 0.9993 |
| selection | property_read | 5.149 | 0.9990 |
| selection | property_write | 3.850 | 0.9980 |
| selection | string_slice | 0.956 | 1.0003 |
| selection | top_level_function_call | 1.390 | 0.9957 |
| sentinel | capturing_closure_call | 1.045 | 1.0007 |
| sentinel | heterogeneous_property_read | 1.411 | 0.9992 |
| sentinel | polymorphic_call_site | 0.982 | 0.9966 |
| sentinel | prototype_method_call | 1.264 | 1.0079 |
| sentinel | recursive_call_tree | 0.642 | 0.9991 |
| sentinel | string_key_map_churn | 1.113 | 1.0004 |
