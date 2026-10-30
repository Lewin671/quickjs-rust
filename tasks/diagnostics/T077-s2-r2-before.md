# S2 round-2 before-state construction work

Candidate `28206401` versus main `dcee3c56`, before runtime repairs.
Jobs `20261030T174905Z-t077-s2-r2-before-triage` (timing) and
`20261030T180431Z-t077-s2-r2-construction-before` (counts/plain instructions).
All 24 input/role collections complete with matching stdout; plain instructions
have three samples per role. No instrumented timing or held-out input.

[Method](T077-s2-round2-method.md) defines coverage and normalization.
[Raw counts](T077-s2-r2-before-counts.jsonl) retain input/binary hashes and
all fields. Whole-process allocations/hashes are normalized, not causally
attributed to object creation. Base stored names are Rc strings, not atom leases.

| case | role | objects | alloc requests/object | schema acquire/clone/drop | lease clone/drop/upgrade | storage borrow read/write | slot alloc/realloc | transition hit/miss | store IC probes/object | instruction median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| development/big.js/arith | base | 14191 | 57.419 | 0.000/0.000/0.000 | 0.000/0.004/0.000 | 13.326/6.082 | 0.750/0.004 | 0.000/0.000 | 4.364 | 24029550955 |
| development/big.js/arith | candidate | 13647 | 58.318 | 8.470/0.966/8.257 | 3.165/27.097/23.969 | 148.082/11.186 | 1.004/0.007 | 4.432/0.053 | 7.973 | 23983093207 |
| development/diff/compare | base | 1614971 | 13.202 | 0.882/0.882/0.882 | 0.000/0.000/0.000 | 14.136/0.000 | 0.011/0.004 | 0.000/0.000 | 0.453 | 43892118815 |
| development/diff/compare | candidate | 1599510 | 10.470 | 1.957/1.240/1.955 | 0.157/5.614/5.393 | 34.584/1.119 | 0.150/0.007 | 0.525/0.059 | 0.472 | 40446301806 |
| development/graphlib/paths | base | 166658 | 17.569 | 0.689/0.688/0.558 | 0.000/0.000/0.000 | 26.949/0.019 | 0.359/0.130 | 0.000/0.000 | 0.209 | 8988570278 |
| development/graphlib/paths | candidate | 165491 | 23.905 | 5.968/3.647/5.618 | 5.737/18.416/12.231 | 87.798/12.181 | 0.362/0.149 | 0.386/1.450 | 0.355 | 9768392346 |
| development/immutable/persistent | base | 497802 | 10.608 | 0.537/0.537/0.486 | 0.000/0.001/0.000 | 13.738/4.825 | 0.412/0.151 | 0.000/0.000 | 2.480 | 20945208134 |
| development/immutable/persistent | candidate | 497129 | 10.455 | 4.245/1.450/4.171 | 2.037/6.937/4.650 | 43.813/7.543 | 0.413/0.202 | 1.277/0.455 | 4.902 | 21505903855 |
| development/js-yaml/document | base | 87316 | 109.917 | 0.132/0.132/0.131 | 0.000/0.001/0.000 | 66.851/6.987 | 0.098/0.033 | 0.000/0.000 | 21.291 | 25554069033 |
| development/js-yaml/document | candidate | 86713 | 106.960 | 2.923/1.077/2.762 | 0.702/25.588/24.891 | 232.109/24.433 | 0.144/0.034 | 0.629/0.011 | 22.768 | 23346422680 |
| development/lodash/transform | base | 82554 | 20.943 | 0.974/0.971/0.489 | 0.000/0.000/0.000 | 17.555/0.015 | 0.008/0.006 | 0.000/0.000 | 0.010 | 13008257069 |
| development/lodash/transform | candidate | 80890 | 20.323 | 3.185/1.050/2.640 | 4.069/7.323/3.761 | 50.397/1.114 | 0.011/0.007 | 0.051/0.019 | 0.012 | 12958960502 |
| development/matter-js/simulate | base | 8877 | 105.145 | 0.985/0.914/0.698 | 0.000/0.005/0.000 | 6851.021/93.826 | 0.145/0.040 | 0.000/0.000 | 590.805 | 30624511261 |
| development/matter-js/simulate | candidate | 7459 | 126.599 | 3.690/1.920/2.978 | 6.101/13.852/7.751 | 8317.974/1019.370 | 0.200/0.062 | 0.381/0.385 | 1017.655 | 25916795110 |
| development/meriyah/parse | base | 232658 | 15.909 | 0.958/0.957/0.952 | 0.000/0.000/0.000 | 14.561/0.001 | 0.040/0.604 | 0.000/0.000 | 9.075 | 16909043537 |
| development/meriyah/parse | candidate | 232069 | 15.221 | 3.325/1.133/2.503 | 10.119/23.566/13.449 | 88.523/11.374 | 0.048/0.606 | 1.294/0.055 | 11.031 | 14756189234 |
| development/moment/dates | base | 572143 | 22.454 | 0.420/0.420/0.420 | 0.000/0.000/0.000 | 49.932/2.670 | 0.978/0.462 | 0.000/0.000 | 3.062 | 50367830512 |
| development/moment/dates | candidate | 571542 | 30.183 | 13.976/6.451/13.970 | 13.121/43.251/29.538 | 251.168/14.869 | 0.979/0.462 | 0.968/3.014 | 7.479 | 57242681639 |
| development/papaparse/csv | base | 30612 | 286.787 | 0.983/0.979/0.980 | 0.000/0.000/0.000 | 39.426/0.013 | 0.752/0.226 | 0.000/0.000 | 0.225 | 19610113077 |
| development/papaparse/csv | candidate | 30059 | 259.793 | 8.354/1.084/5.749 | 0.277/172.651/172.391 | 847.790/12.422 | 0.772/0.232 | 2.716/0.026 | 0.465 | 20371632824 |
| external/sunspider-1.0/access-binary-trees | base | 42726 | 2.207 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 3.919/0.000 | 0.988/0.001 | 0.000/0.000 | 2.954 | 316107104 |
| external/sunspider-1.0/access-binary-trees | candidate | 42198 | 2.268 | 4.085/0.053/4.014 | 3.014/10.030/7.026 | 38.074/9.049 | 1.001/0.002 | 3.001/0.016 | 5.982 | 443449679 |
| external/kraken-1.1/json-parse-financial | base | 52650 | 9.998 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 0.019/0.000 | 0.990/1.046 | 0.000/0.000 | 0.000 | 409844889 |
| external/kraken-1.1/json-parse-financial | candidate | 52125 | 10.127 | 7.139/0.044/7.084 | 0.019/5.758/5.747 | 17.390/0.076 | 1.001/1.057 | 5.666/0.014 | 0.000 | 566279956 |
| external/jetstream3-js-subset/raytrace-public-class-fields | base | 799970 | 2.126 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 10.366/3.798 | 0.999/0.224 | 0.000/0.000 | 2.799 | 6580234405 |
| external/jetstream3-js-subset/raytrace-public-class-fields | candidate | 799290 | 2.145 | 4.837/0.003/4.834 | 0.002/5.282/5.281 | 32.344/6.636 | 1.000/0.224 | 3.832/0.001 | 2.802 | 8714354453 |
| external/jetstream3-js-subset/cdjs | base | 379686 | 6.120 | 0.231/0.231/0.231 | 0.000/0.000/0.000 | 24.805/1.292 | 0.768/0.106 | 0.000/0.000 | 3.371 | 8083246972 |
| external/jetstream3-js-subset/cdjs | candidate | 379118 | 6.508 | 3.406/0.238/2.949 | 2.085/7.020/4.936 | 57.815/7.546 | 0.769/0.106 | 2.083/0.002 | 5.459 | 9406092906 |
| broad/string_slice | base | 656 | 3081.870 | 0.040/0.015/0.026 | 0.000/0.008/0.000 | 0.002/0.008 | 0.189/0.081 | 0.000/0.000 | 0.000 | 1784593663 |
| broad/string_slice | candidate | 128 | 15806.555 | 33.211/18.109/10.992 | 7.914/10.055/5.688 | 7861.469/16.055 | 1.398/0.672 | 3.125/5.539 | 0.000 | 1925335839 |
| broad/top_level_function_call | base | 655 | 31.589 | 0.040/0.015/0.026 | 0.000/0.008/0.000 | 0.002/3053.440 | 0.189/0.081 | 0.000/0.000 | 0.000 | 1161799286 |
| broad/top_level_function_call | candidate | 128 | 173.617 | 33.211/18.109/10.992 | 7.914/10.125/5.758 | 31299.180/15641.070 | 1.398/0.672 | 3.125/5.539 | 0.000 | 1223499621 |

Retaining N/2N controls isolate incremental object construction from bootstrap.
The two-field constructor adds 1 header and 1 slot buffer in both versions,
but S2 adds schema ownership/lease/transition work. Two-field literals use one
allocation in base and two in S2. Class constants lost base's single storage
reservation; S2 repeatedly defines each key. JSON repeats interning/lookup and
transition handling per key. These observations select the
[construction plan](../proposals/T077-s2-round2-construction.md).
