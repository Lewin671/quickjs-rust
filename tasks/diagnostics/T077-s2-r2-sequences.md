# S2 round-2 sequence repair diagnostic

Timing candidate `e320817e`, base `dcee3c56`: job `20261030T183538Z-t077-s2-r2-sequences-triage-r2`.
Five paired natural-layout samples, diagnostic only; harness verdict **likely loss**.
Development geomean 1.000875; no complete external/selection aggregate or held-out claim.
The login-shell job `20261030T183324Z-t077-s2-r2-sequences-triage` failed host/recipe identity before measurement.

| case | wall | instructions |
| --- | ---: | ---: |
| development/lodash/transform | 1.0408 | 1.0041 |
| development/meriyah/parse | 0.8966 | 0.8835 |
| development/immutable/persistent | 1.0265 | 0.9985 |
| development/moment/dates | 1.0060 | 1.0196 |
| development/diff/compare | 0.8955 | 0.9273 |
| development/papaparse/csv | 1.0437 | 1.0461 |
| development/js-yaml/document | 0.9541 | 0.9238 |
| development/big.js/arith | 1.0023 | 0.9979 |
| development/matter-js/simulate | 1.0183 | 0.8637 |
| development/graphlib/paths | 1.1504 | 1.0979 |
| external/sunspider-1.0/access-binary-trees | 1.0488 | 0.9314 |
| external/kraken-1.1/json-parse-financial | 1.0584 | 0.9182 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.2921 | 1.1836 |
| external/jetstream3-js-subset/cdjs | 1.1549 | 1.0627 |
| broad/string_slice | 1.1143 | 1.1013 |
| broad/top_level_function_call | 1.0567 | 1.0756 |
| sentinel/capturing_closure_call | 0.9500 | 0.9716 |
| sentinel/heterogeneous_property_read | 0.6433 | 0.8000 |
| sentinel/polymorphic_call_site | 0.9829 | 0.9700 |
| sentinel/prototype_method_call | 1.0453 | 0.9109 |
| sentinel/recursive_call_tree | 0.9784 | 1.0000 |
| sentinel/string_key_map_churn | 0.9176 | 0.9157 |
| broad/property_dynamic_read | 0.8124 | 0.7160 |
| broad/object_allocation | 0.9922 | 1.0009 |
| external/kraken-1.1/json-stringify-tinderbox | 1.0475 | 1.0631 |
| external/kraken-1.1/audio-dft | 1.0320 | 1.0211 |

Count/plain-instruction candidate **`b488bff8`**, including direct class fields at `3c1c377a` and typed-loop boundary at `b488bff8`;
job `20261030T184052Z-t077-s2-r2-construction-sequences`. All 24 cases/roles complete, exact stdout equality;
[full raw records](T077-s2-r2-sequences-counts.jsonl) include three plain instruction counts and binary/input hashes.
Same [method](T077-s2-round2-method.md) as the [before state](T077-s2-r2-before.md). Whole-program normalized totals are not causal construction costs.

| case | role | objects | allocator requests/object | schema acquire/clone/drop | leases clone/drop/upgrade | storage R/W | slot alloc/realloc | transition hit/miss | IC store probes/object | plain instruction median |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| development/big.js/arith | base | 14191 | 57.419 | 0.000/0.000/0.000 | 0.000/0.004/0.000 | 13.326/6.082 | 0.750/0.004 | 0.000/0.000 | 4.364 | 24029436457 |
| development/big.js/arith | candidate | 13647 | 58.314 | 8.472/4.051/8.259 | 0.079/14.754/14.712 | 103.352/5.034 | 0.998/0.007 | 1.350/0.053 | 4.898 | 23937859180 |
| development/diff/compare | base | 1614971 | 13.202 | 0.882/0.882/0.882 | 0.000/0.000/0.000 | 14.136/0.000 | 0.011/0.004 | 0.000/0.000 | 0.453 | 43892043789 |
| development/diff/compare | candidate | 1599510 | 10.003 | 1.969/1.255/1.967 | 0.146/5.587/5.374 | 34.540/1.132 | 0.093/0.007 | 0.520/0.062 | 0.472 | 40526437060 |
| development/graphlib/paths | base | 166658 | 17.569 | 0.689/0.688/0.558 | 0.000/0.000/0.000 | 26.949/0.019 | 0.359/0.130 | 0.000/0.000 | 0.209 | 8989222854 |
| development/graphlib/paths | candidate | 165491 | 23.224 | 5.965/3.646/5.615 | 5.733/18.390/12.209 | 87.505/12.175 | 0.312/0.149 | 0.386/1.448 | 0.353 | 9887849940 |
| development/immutable/persistent | base | 497802 | 10.608 | 0.537/0.537/0.486 | 0.000/0.001/0.000 | 13.738/4.825 | 0.412/0.151 | 0.000/0.000 | 2.480 | 20944820349 |
| development/immutable/persistent | candidate | 497129 | 10.394 | 4.246/2.727/4.172 | 0.760/3.107/2.097 | 31.052/4.992 | 0.351/0.202 | 0.001/0.455 | 3.626 | 20876454980 |
| development/js-yaml/document | base | 87316 | 109.917 | 0.132/0.132/0.131 | 0.000/0.001/0.000 | 66.851/6.987 | 0.098/0.033 | 0.000/0.000 | 21.291 | 25555156860 |
| development/js-yaml/document | candidate | 86713 | 106.895 | 2.924/1.080/2.763 | 0.698/25.575/24.882 | 226.223/24.432 | 0.078/0.034 | 0.626/0.011 | 22.767 | 23548501485 |
| development/lodash/transform | base | 82554 | 20.943 | 0.974/0.971/0.489 | 0.000/0.000/0.000 | 17.555/0.015 | 0.008/0.006 | 0.000/0.000 | 0.010 | 13007578834 |
| development/lodash/transform | candidate | 80890 | 20.321 | 3.185/1.051/2.639 | 4.068/7.320/3.760 | 50.390/1.114 | 0.010/0.007 | 0.051/0.019 | 0.012 | 13001569453 |
| development/matter-js/simulate | base | 8877 | 105.145 | 0.985/0.914/0.698 | 0.000/0.005/0.000 | 6851.021/93.826 | 0.145/0.040 | 0.000/0.000 | 590.805 | 30624770349 |
| development/matter-js/simulate | candidate | 7459 | 126.066 | 3.736/2.188/2.978 | 5.814/13.057/7.242 | 8315.534/1019.034 | 0.073/0.062 | 0.159/0.385 | 1017.433 | 26436093185 |
| development/meriyah/parse | base | 232658 | 15.909 | 0.958/0.957/0.952 | 0.000/0.000/0.000 | 14.561/0.001 | 0.040/0.604 | 0.000/0.000 | 9.075 | 16909856454 |
| development/meriyah/parse | candidate | 232069 | 16.109 | 4.216/2.209/3.393 | 9.043/21.230/12.188 | 88.824/13.156 | 0.264/0.387 | 1.110/0.055 | 10.846 | 14905705392 |
| development/moment/dates | base | 572143 | 22.454 | 0.420/0.420/0.420 | 0.000/0.000/0.000 | 49.932/2.670 | 0.978/0.462 | 0.000/0.000 | 3.062 | 50366968774 |
| development/moment/dates | candidate | 571542 | 21.755 | 9.378/4.876/9.372 | 7.389/26.349/18.809 | 171.511/7.332 | 0.517/0.462 | 0.338/0.621 | 4.603 | 51089371321 |
| development/papaparse/csv | base | 30612 | 286.787 | 0.983/0.979/0.980 | 0.000/0.000/0.000 | 39.426/0.013 | 0.752/0.226 | 0.000/0.000 | 0.225 | 19603389719 |
| development/papaparse/csv | candidate | 30059 | 259.564 | 8.356/1.318/5.750 | 0.042/171.945/171.920 | 845.446/11.965 | 0.540/0.232 | 2.485/0.026 | 0.236 | 20462022107 |
| external/sunspider-1.0/access-binary-trees | base | 42726 | 2.207 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 3.919/0.000 | 0.988/0.001 | 0.000/0.000 | 2.954 | 316110285 |
| external/sunspider-1.0/access-binary-trees | candidate | 42198 | 2.266 | 4.085/3.044/4.014 | 0.023/1.056/1.044 | 8.164/3.067 | 0.999/0.002 | 0.010/0.016 | 2.991 | 292733465 |
| external/kraken-1.1/json-parse-financial | base | 52650 | 9.998 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 0.019/0.000 | 0.990/1.046 | 0.000/0.000 | 0.000 | 409857530 |
| external/kraken-1.1/json-parse-financial | candidate | 52125 | 9.435 | 2.304/1.080/2.249 | 0.019/0.827/0.817 | 7.567/0.076 | 0.999/0.098 | 0.736/0.014 | 0.000 | 376749857 |
| external/jetstream3-js-subset/raytrace-public-class-fields | base | 799970 | 2.126 | 0.000/0.000/0.000 | 0.000/0.000/0.000 | 10.366/3.798 | 0.999/0.224 | 0.000/0.000 | 2.799 | 6580313006 |
| external/jetstream3-js-subset/raytrace-public-class-fields | candidate | 799290 | 2.082 | 4.837/3.834/4.834 | 0.002/1.004/1.003 | 14.678/4.252 | 0.937/0.224 | 0.001/0.001 | 2.802 | 7420656585 |
| external/jetstream3-js-subset/cdjs | base | 379686 | 6.120 | 0.231/0.231/0.231 | 0.000/0.000/0.000 | 24.805/1.292 | 0.768/0.106 | 0.000/0.000 | 3.371 | 8085761008 |
| external/jetstream3-js-subset/cdjs | candidate | 379118 | 5.785 | 3.406/2.320/2.949 | 0.003/0.773/0.772 | 36.996/3.383 | 0.277/0.106 | 0.001/0.002 | 3.377 | 8531158351 |
| broad/string_slice | base | 656 | 3081.870 | 0.040/0.015/0.026 | 0.000/0.008/0.000 | 0.002/0.008 | 0.189/0.081 | 0.000/0.000 | 0.000 | 1784593384 |
| broad/string_slice | candidate | 128 | 15805.883 | 33.211/18.109/10.992 | 7.914/10.055/5.688 | 7861.469/16.055 | 0.734/0.672 | 3.125/5.539 | 0.000 | 1959396353 |
| broad/top_level_function_call | base | 655 | 31.589 | 0.040/0.015/0.026 | 0.000/0.008/0.000 | 0.002/3053.440 | 0.189/0.081 | 0.000/0.000 | 0.000 | 1161757028 |
| broad/top_level_function_call | candidate | 128 | 172.945 | 33.211/18.109/10.992 | 7.914/10.125/5.758 | 31299.180/15641.070 | 0.734/0.672 | 3.125/5.539 | 0.000 | 1249531632 |
| control/constructor/1000 | base | 1654 | 6.598 | 0.001/0.001/0.001 | 0.000/0.003/0.000 | 0.604/1.211 | 0.681/0.032 | 0.000/0.000 | 1.209 | 25598609 |
| control/constructor/1000 | candidate | 1127 | 9.941 | 6.272/3.744/2.972 | 0.866/2.806/2.348 | 16.532/6.145 | 0.084/0.076 | 0.361/0.599 | 1.776 | 26483986 |
| control/constructor/2000 | base | 2654 | 4.865 | 0.001/0.000/0.001 | 0.000/0.002/0.000 | 0.753/1.508 | 0.801/0.020 | 0.000/0.000 | 1.507 | 36931515 |
| control/constructor/2000 | candidate | 2127 | 5.738 | 4.734/2.924/2.515 | 0.459/2.427/2.184 | 14.872/5.606 | 0.045/0.040 | 0.191/0.317 | 1.882 | 37055112 |
| control/literal/1000 | base | 1652 | 5.967 | 0.608/0.607/0.002 | 0.000/0.003/0.000 | 0.605/1.212 | 0.076/0.032 | 0.000/0.000 | 0.000 | 22473389 |
| control/literal/1000 | candidate | 1126 | 9.883 | 4.480/2.846/1.190 | 0.863/1.908/1.448 | 13.805/4.367 | 0.083/0.076 | 0.358/0.597 | 0.000 | 24008901 |
| control/literal/2000 | base | 2652 | 4.094 | 0.756/0.755/0.002 | 0.000/0.002/0.000 | 0.754/1.509 | 0.047/0.020 | 0.000/0.000 | 0.000 | 30790260 |
| control/literal/2000 | candidate | 2126 | 5.705 | 2.843/1.978/0.630 | 0.457/1.481/1.238 | 12.015/3.724 | 0.044/0.040 | 0.190/0.316 | 0.000 | 32190207 |
| control/class/1000 | base | 1652 | 6.602 | 0.001/0.001/0.001 | 0.000/0.003/0.000 | 1.813/1.819 | 0.681/0.032 | 0.000/0.000 | 0.000 | 28963018 |
| control/class/1000 | candidate | 1126 | 10.825 | 7.150/4.627/3.853 | 0.865/2.798/2.339 | 18.265/6.146 | 0.972/0.076 | 0.358/0.598 | 0.000 | 30780880 |
| control/class/2000 | base | 2652 | 4.867 | 0.001/0.000/0.001 | 0.000/0.002/0.000 | 2.261/2.264 | 0.801/0.020 | 0.000/0.000 | 0.000 | 43660882 |
| control/class/2000 | candidate | 2126 | 6.674 | 5.668/3.862/3.452 | 0.458/2.422/2.180 | 16.729/5.607 | 0.985/0.040 | 0.190/0.317 | 0.000 | 45685866 |
| control/json/1000 | base | 1652 | 8.993 | 0.001/0.001/0.001 | 0.000/0.003/0.000 | 2.418/1.212 | 0.681/0.032 | 0.000/0.000 | 0.000 | 28782409 |
| control/json/1000 | candidate | 1126 | 11.658 | 7.141/2.845/3.851 | 0.863/3.682/3.224 | 20.022/4.366 | 0.083/0.076 | 2.132/0.597 | 0.000 | 30003607 |
| control/json/2000 | base | 2652 | 7.864 | 0.001/0.000/0.001 | 0.000/0.002/0.000 | 3.014/1.509 | 0.801/0.020 | 0.000/0.000 | 0.000 | 43369634 |
| control/json/2000 | candidate | 2126 | 7.586 | 5.664/1.977/3.451 | 0.457/3.361/3.119 | 18.601/3.723 | 0.044/0.040 | 2.070/0.316 | 0.000 | 44222505 |

Construction target remains open. Subsequent global-write fusion/root/hint/dynamic-map repairs require a new cohort; these records stay frozen.
