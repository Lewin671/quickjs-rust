# S2 header and descriptor-sequence checkpoints

These are completed intermediate cohorts, **not final acceptance**. Base is
accepted main dcee3c56. All natural-layout public86 timing is diagnostic only:
five pairs, exploratory intervals, no held-out/formal precision claim.

The 061e7f28 job `20261030T221440Z-t077-s2-r2-date-number-public86` is
[incomplete](T077-s2-r2-public86-061e-incomplete.json): health rejected
`attempt 2 candidate: invalid wall_ns increment -65652493`. It supplies no
usable aggregate verdict; the invalid cohort is preserved.

The completed jobs are `20261030T222542Z-t077-s2-r2-header-public86`
([14abe3f3](T077-s2-r2-public86-header.json)) and
`20261030T223426Z-t077-s2-r2-sequences-hoists-public86`
([f380089d](T077-s2-r2-public86-f380.json)). Both automatic verdicts are
**likely loss**. The latter adds validated arguments sequences and entry-proved
native-call hoisting. Immutable monomorphic guards are later source.

| lane | count | 14abe wall / instructions | f380 wall / instructions |
| --- | ---: | ---: | ---: |
| development | 10 | 0.983161 / 0.954967 | 0.984720 / 0.956724 |
| external | 45 | 1.035770 / 1.018318 | 1.039737 / 1.020494 |
| broad | 25 | 0.997591 / 0.990000 | 0.997287 / 0.983069 |
| sentinel | 6 | 0.932205 / 0.936486 | 0.933117 / 0.936479 |
| all | 86 | 1.010875 / 0.996646 | 1.013065 / 0.995936 |

The string-slice target is repaired at f380: .903757 wall / .927737 instructions,
versus 1.064608 / 1.065389 at 14abe. External payoff and some construction table
walls remain open; instruction improvement alone does not satisfy wall payoff.

Non-timing job `20261030T222951Z-t077-s2-r2-argument-sequences-counts`
measures f5a83119: [all 24 paired records](T077-s2-r2-arguments-counts.jsonl),
three canonical instruction samples/role, exact stdout equality. Instrumented
copies supply work counts only; the [method](T077-s2-round2-method.md) defines
normalization and limits. This cohort predates the new IC-metadata counters.

| case | instruction ratio | alloc requests / object | schema acquire / clone / drop | leases clone / drop / upgrade | storage R / W | slot alloc / realloc | transition hit / miss | store IC probes / object |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| development/big.js/arith | 0.997751 | 57.963 | 6.505/4.821/6.291 | 0.079/13.982/13.940 | 101.036/5.034 | 0.998/0.007 | 1.350/0.053 | 4.898 |
| development/diff/compare | 0.926592 | 10.007 | 1.890/1.274/1.888 | 0.158/5.636/5.406 | 34.533/1.121 | 0.093/0.006 | 0.478/0.075 | 0.472 |
| development/graphlib/paths | 1.001581 | 15.467 | 1.138/1.040/0.919 | 9.201/12.490/3.695 | 73.925/7.694 | 0.312/0.014 | 0.025/0.005 | 0.353 |
| development/immutable/persistent | 0.958158 | 8.370 | 2.576/2.183/2.510 | 0.307/1.840/1.534 | 26.997/3.835 | 0.351/0.151 | 0.001/0.003 | 3.626 |
| development/js-yaml/document | 0.915266 | 107.002 | 2.600/1.259/2.475 | 1.500/25.311/24.170 | 221.987/24.166 | 0.078/0.034 | 0.180/0.100 | 22.767 |
| development/lodash/transform | 0.991674 | 18.783 | 2.129/1.046/1.596 | 4.567/7.272/3.213 | 50.188/1.065 | 0.010/0.002 | 0.020/0.015 | 0.012 |
| development/matter-js/simulate | 0.859793 | 125.403 | 2.607/1.906/1.889 | 5.739/12.703/6.982 | 8314.055/1018.615 | 0.073/0.034 | 0.105/0.230 | 1017.435 |
| development/meriyah/parse | 0.872311 | 15.941 | 3.239/2.108/2.865 | 9.009/21.073/12.065 | 88.414/13.027 | 0.264/0.376 | 1.076/0.022 | 10.846 |
| development/moment/dates | 0.977242 | 19.530 | 7.523/4.408/7.518 | 7.611/21.515/13.905 | 152.478/5.651 | 0.517/0.168 | 0.107/0.107 | 4.603 |
| development/papaparse/csv | 1.036829 | 258.990 | 2.187/1.319/1.127 | 7.400/169.493/164.559 | 827.335/9.511 | 0.540/0.232 | 0.032/0.026 | 0.236 |
| external/sunspider-1.0/access-binary-trees | 0.837260 | 2.266 | 4.085/4.041/4.014 | 0.023/0.060/0.047 | 5.173/3.067 | 0.999/0.002 | 0.010/0.016 | 2.991 |
| external/kraken-1.1/json-parse-financial | 0.924165 | 9.435 | 1.902/1.080/1.847 | 0.019/0.827/0.817 | 7.567/0.076 | 0.999/0.098 | 0.736/0.014 | 0.000 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 0.985341 | 2.082 | 2.454/2.450/2.450 | 0.002/0.004/0.003 | 11.678/4.252 | 0.937/0.000 | 0.001/0.001 | 2.802 |
| external/jetstream3-js-subset/cdjs | 1.035113 | 5.785 | 3.379/3.088/2.945 | 0.003/0.005/0.003 | 34.690/3.383 | 0.277/0.000 | 0.001/0.002 | 3.377 |
| broad/string_slice | 1.065378 | 15805.891 | 33.203/18.109/10.992 | 7.914/10.055/5.688 | 7861.469/16.055 | 0.734/0.672 | 3.125/5.539 | 0.000 |
| broad/top_level_function_call | 1.009165 | 172.953 | 33.203/18.109/10.992 | 7.914/10.125/5.758 | 49.211/15641.070 | 0.734/0.672 | 3.125/5.539 | 0.000 |
| control/constructor/1000 | 1.003584 | 9.944 | 6.272/4.631/2.972 | 0.866/1.919/1.461 | 10.329/6.145 | 0.084/0.076 | 0.361/0.599 | 1.776 |
| control/constructor/2000 | 0.960786 | 5.739 | 4.733/3.864/2.515 | 0.459/1.487/1.244 | 8.294/5.606 | 0.045/0.040 | 0.191/0.317 | 1.882 |
| control/literal/1000 | 1.057553 | 9.884 | 4.479/2.846/1.190 | 0.863/1.907/1.448 | 10.255/4.367 | 0.083/0.076 | 0.358/0.597 | 0.000 |
| control/literal/2000 | 1.032064 | 5.705 | 2.842/1.978/0.630 | 0.457/1.480/1.237 | 8.254/3.724 | 0.044/0.040 | 0.190/0.316 | 0.000 |
| control/class/1000 | 1.027419 | 10.826 | 6.263/4.629/2.965 | 0.865/1.910/1.452 | 12.057/6.146 | 0.972/0.076 | 0.358/0.598 | 0.000 |
| control/class/2000 | 1.000150 | 6.675 | 4.728/3.863/2.511 | 0.458/1.482/1.239 | 10.149/5.607 | 0.985/0.040 | 0.190/0.317 | 0.000 |
| control/json/1000 | 1.038512 | 11.659 | 7.139/2.845/3.851 | 0.863/3.681/3.223 | 16.472/4.366 | 0.083/0.076 | 2.132/0.597 | 0.000 |
| control/json/2000 | 1.013990 | 7.586 | 5.663/1.977/3.451 | 0.457/3.361/3.118 | 14.839/3.723 | 0.044/0.040 | 2.070/0.316 | 0.000 |

Graphlib schema upgrades fall 324,017 (5f509fb9 retained-layout cohort) to 5,749;
its instruction ratio is 1.001581 at f5. Per-object entries above are whole-process
totals divided by actual ordinary ObjectData allocations, not causal construction
cost. Array allocations, live values and non-atom stored-name Rc costs differ.
Native-call/S3 entry and binding-cell authority are unchanged. No dependency or
unsafe was introduced. Main versus candidate RSS after class-name boxing is
recorded separately; final source still needs its own gates and measurements.
