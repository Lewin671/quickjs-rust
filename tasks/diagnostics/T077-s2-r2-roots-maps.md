# S2 roots/maps construction work

Candidate `90719950`, base `dcee3c56`; non-timing job `20261030T190528Z-t077-s2-r2-construction-roots-maps`.
All 24 cases/roles complete with exact stdout equality and three canonical instruction samples per role.
[Raw records](T077-s2-r2-roots-maps-counts.jsonl) retain all counters/input/image hashes. Same [method](T077-s2-round2-method.md).
No wall estimate or held-out input; this is not formal acceptance. Whole-program normalized work is not causal construction cost.

| case | instruction ratio | candidate alloc requests/object | schema acquire/clone/drop | leases clone/drop/upgrade | storage R/W | transition hit/miss |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| development/big.js/arith | 0.99650 | 58.314 | 8.472/4.821/8.259 | 0.079/14.754/14.712 | 103.349/5.034 | 1.350/0.053 |
| development/diff/compare | 0.92956 | 10.042 | 1.980/1.267/1.978 | 0.155/5.596/5.370 | 34.546/1.144 | 0.511/0.071 |
| development/graphlib/paths | 1.07235 | 21.467 | 5.448/3.427/5.230 | 10.635/15.402/4.422 | 81.745/11.269 | 0.154/1.352 |
| development/immutable/persistent | 0.99786 | 10.394 | 4.246/2.988/4.172 | 0.760/3.107/2.096 | 31.049/4.991 | 0.001/0.455 |
| development/js-yaml/document | 0.92474 | 107.002 | 2.843/1.259/2.686 | 1.500/25.311/24.171 | 224.037/24.166 | 0.181/0.100 |
| development/lodash/transform | 1.00130 | 18.795 | 3.172/1.042/2.630 | 4.567/7.304/3.245 | 50.306/1.097 | 0.051/0.015 |
| development/matter-js/simulate | 0.86972 | 125.800 | 3.612/2.102/2.863 | 5.849/12.985/7.135 | 8314.982/1018.905 | 0.159/0.346 |
| development/meriyah/parse | 0.88513 | 16.142 | 4.294/2.287/3.471 | 9.076/21.207/12.132 | 88.689/13.161 | 1.076/0.089 |
| development/moment/dates | 1.01500 | 21.330 | 8.956/4.938/8.951 | 7.914/26.096/18.032 | 169.680/6.869 | 0.317/0.410 |
| development/papaparse/csv | 1.04848 | 258.991 | 6.664/1.319/4.083 | 7.400/169.495/164.560 | 827.339/9.511 | 0.032/0.026 |
| external/sunspider-1.0/access-binary-trees | 0.90029 | 2.266 | 4.085/4.041/4.014 | 0.023/1.056/1.044 | 8.164/3.067 | 0.010/0.016 |
| external/kraken-1.1/json-parse-financial | 0.91808 | 9.435 | 2.304/1.080/2.249 | 0.019/0.827/0.817 | 7.567/0.076 | 0.736/0.014 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.09458 | 2.082 | 4.837/4.834/4.834 | 0.002/1.004/1.003 | 14.678/4.252 | 0.001/0.001 |
| external/jetstream3-js-subset/cdjs | 1.05140 | 5.785 | 3.406/3.088/2.949 | 0.003/0.773/0.772 | 36.996/3.383 | 0.001/0.002 |
| broad/string_slice | 1.08340 | 15805.883 | 33.211/18.109/10.992 | 7.914/10.055/5.688 | 7861.469/16.055 | 3.125/5.539 |
| broad/top_level_function_call | 1.01270 | 172.945 | 33.211/18.109/10.992 | 7.914/10.125/5.758 | 49.211/15641.070 | 3.125/5.539 |
| control/constructor/1000 | 1.02225 | 9.943 | 6.273/4.631/2.972 | 0.866/2.806/2.348 | 12.987/6.145 | 0.361/0.599 |
| control/constructor/2000 | 0.98729 | 5.739 | 4.734/3.864/2.515 | 0.459/2.427/2.184 | 11.112/5.606 | 0.191/0.317 |
| control/literal/1000 | 1.06803 | 9.883 | 4.480/2.846/1.190 | 0.863/1.908/1.448 | 10.256/4.367 | 0.358/0.597 |
| control/literal/2000 | 1.04562 | 5.705 | 2.843/1.978/0.630 | 0.457/1.481/1.238 | 8.254/3.724 | 0.190/0.316 |
| control/class/1000 | 1.05427 | 10.825 | 7.151/5.515/3.853 | 0.865/2.798/2.339 | 14.716/6.146 | 0.358/0.598 |
| control/class/2000 | 1.03450 | 6.674 | 5.669/4.802/3.452 | 0.458/2.422/2.180 | 12.968/5.607 | 0.190/0.317 |
| control/json/1000 | 1.04265 | 11.658 | 7.141/2.845/3.851 | 0.863/3.682/3.224 | 16.473/4.366 | 2.132/0.597 |
| control/json/2000 | 1.01976 | 7.586 | 5.664/1.977/3.451 | 0.457/3.361/3.119 | 14.840/3.723 | 2.070/0.316 |

Construction target remains open. Live-fanout normalization leaves 223,734 graphlib misses;
the [persistent pressure correction](../proposals/T077-s2-transition-pressure.md) is a subsequent source change.
Class-field publication and primitive IC leaf ABI still need work; array header migration remains queued.
