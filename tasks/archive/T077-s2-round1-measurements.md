# T077 S2 round-1 iMac diagnostics

Source `e1d8a6a3`; base `dd0905b2`. All results are candidate/base, lower faster.
Five paired natural-layout samples, qualified workers-4 v5 recipe; no held-out
input was read/profiled and no formal acceptance was run. All seven jobs exit 0,
complete within budget and retain their automatic **likely loss** verdict.
Do not select a favorable older cohort or multiply ratios across runs.

## Repair sequence

| source | job | development geomean | heterogeneous | prototype | churn | dynamic |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 26fee930 | 20261030T135556Z-t077-s2-r1-triage | 1.067384 | 1.3525 | 1.2908 | 1.3564 | 1.3165 |
| dbe43671 | 20261030T142043Z-t077-s2-r1-payoff-triage | 1.013157 | 1.0630 | 1.3505 | 0.9425 | 0.7819 |
| 63d30d83 | 20261030T143935Z-t077-s2-r1-borrowed-triage | 0.994917 | 1.0861 | 1.0066 | 0.8794 | 0.7861 |
| 903a5dc0 | 20261030T150125Z-t077-s2-r1-parallel-triage | 1.013005 | 0.9851 | 1.0402 | 0.9125 | 0.7643 |
| 61966610 | 20261030T151732Z-t077-s2-r1-numeric-triage | 1.007384 | 0.6580 | 0.9826 | 0.9040 | 0.7655 |
| 3feb4a1d | 20261030T153711Z-t077-s2-r1-creation-triage | 1.002736 | 0.6504 | 0.9826 | 0.8989 | 0.7918 |
| e1d8a6a3 | 20261030T155647Z-t077-s2-r1-static-triage | 0.998020 | 0.6571 | 0.9954 | 0.8926 | 0.7956 |

Latest development point geomean is 0.998020. The four requested sentinel/
selection point estimates are below 1.0; prototype overlaps 1.0 in its interval.
These diagnostics do not establish a formal aggregate win. External controls,
Moment, Graphlib, string_slice and other listed cases still lose. Formal
development/external/selection/held-out acceptance remains the lead's decision.

## Final cohort, including all controls

Exploratory 95% intervals are unadjusted and omit layout bias.

| case | wall ratio [interval] | instructions ratio [interval] |
| --- | ---: | ---: |
| development/lodash/transform | 1.0314 [1.0170, 1.0659] | 1.0071 [1.0065, 1.0078] |
| development/meriyah/parse | 0.8794 [0.8235, 0.9139] | 0.8830 [0.8827, 0.8831] |
| development/immutable/persistent | 0.9951 [0.9773, 1.0307] | 1.0347 [1.0347, 1.0348] |
| development/moment/dates | 1.1191 [1.0903, 1.1979] | 1.1196 [1.1196, 1.1197] |
| development/diff/compare | 0.8734 [0.8595, 0.9250] | 0.9263 [0.9262, 0.9263] |
| development/papaparse/csv | 1.0492 [1.0384, 1.0810] | 1.0380 [1.0379, 1.0382] |
| development/js-yaml/document | 0.9236 [0.8844, 1.0114] | 0.9221 [0.9220, 0.9223] |
| development/big.js/arith | 1.0109 [1.0069, 1.0235] | 1.0003 [1.0003, 1.0003] |
| development/matter-js/simulate | 1.0115 [1.0090, 1.0159] | 0.8525 [0.8525, 0.8525] |
| development/graphlib/paths | 1.1214 [1.0361, 1.1359] | 1.1078 [1.1073, 1.1079] |
| sentinel/heterogeneous_property_read | 0.6571 [0.6248, 0.6927] | 0.7813 [0.7813, 0.7815] |
| sentinel/prototype_method_call | 0.9954 [0.9307, 1.0468] | 0.8895 [0.8894, 0.8896] |
| sentinel/string_key_map_churn | 0.8926 [0.8474, 0.9132] | 0.8925 [0.8924, 0.8927] |
| broad/property_dynamic_read | 0.7956 [0.7791, 0.9312] | 0.6908 [0.6907, 0.6909] |
| sentinel/capturing_closure_call | 0.9545 [0.9120, 1.0167] | 0.9796 [0.9795, 0.9797] |
| sentinel/polymorphic_call_site | 0.9713 [0.9075, 0.9904] | 0.9759 [0.9759, 0.9760] |
| sentinel/recursive_call_tree | 1.0211 [0.9926, 1.0410] | 1.0000 [1.0000, 1.0002] |
| broad/object_allocation | 1.0016 [0.9839, 1.0140] | 1.0009 [1.0008, 1.0010] |
| broad/string_slice | 1.0960 [1.0696, 1.1639] | 1.0796 [1.0796, 1.0797] |
| broad/top_level_function_call | 1.0223 [0.9742, 1.0274] | 1.0528 [1.0527, 1.0528] |
| external/kraken-1.1/json-stringify-tinderbox | 1.0514 [1.0375, 1.0607] | 1.0636 [1.0632, 1.0641] |
| external/kraken-1.1/json-parse-financial | 1.3335 [1.3207, 1.3482] | 1.3722 [1.3718, 1.3724] |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.4192 [1.4040, 1.4403] | 1.3295 [1.3295, 1.3295] |
| external/sunspider-1.0/access-binary-trees | 1.4507 [1.4041, 1.4545] | 1.4083 [1.4080, 1.4085] |
| external/kraken-1.1/audio-dft | 1.0378 [1.0236, 1.0481] | 1.0072 [1.0060, 1.0074] |
| external/jetstream3-js-subset/cdjs | 1.2359 [1.2277, 1.2553] | 1.1730 [1.1730, 1.1738] |

Request SHA-256: `6e33431af1fa7270bbd8bcd19bf0ae796f5e033b56b8008bc1fbca7f41caf289`.
Raw-record hash-map fingerprint (SHA-256 over canonical JSON): `c51f30821e2f549ce97ac4742ab49dcc0645af2e4ff2bc0b7af3f131999f482f`.
Host: iMac20,1, Intel i7-10700, macOS 12.6, Rust 1.95.0.
Canonical release: agents, opt-level 3, fat LTO, 16 CGUs, abort, v0 symbols,
LLVM function alignment 4. No new dependency, unsafe, relink or order-file change.

## Memory and target qualification

Final x86 tests/RSS job: `20261030T160533Z-t077-s2-r1-final-x86-rss`, exit 0.
All 2,486 runtime tests with agents/perf-counters pass on x86_64; arm64 full/
verify gates pass separately. Header assertions execute on both targets.
Final x86 candidate SHA-256: `51434239bc843eb04645f95fc3887a76b40079b4da6437bf843f5c209d4cbacb`.
Base SHA-256: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.

One base/candidate pair per 300,000-retained-receiver script via time -l.
Peak RSS below is bytes; this is diagnostic, not the >=15-pair resource protocol.

| script | base RSS | candidate RSS | ratio |
| --- | ---: | ---: | ---: |
| arr_mem | 44175360 | 52027392 | 1.1777 |
| arr_mem_nocall | 41992192 | 52011008 | 1.2386 |
| fn_mem | 120274944 | 111804416 | 0.9296 |
| fn_mem_nocall | 111480832 | 112607232 | 1.0101 |

Arrays retain a larger explicit-prototype header than the rebuild base. The
final called/no-call array peaks differ by only 16 KiB; called functions add no
extra peak over no-call here. Unit proofs establish that IC probes allocate no
array cold/value storage and do not materialize unrelated function defaults.
The array RSS regression remains a reportable cost, not a waived acceptance.

| payload | bytes, excluding RC header |
| --- | ---: |
| NamedStore / RefCell<NamedStore> | 56 / 64 |
| ObjectData | 80 |
| ArrayData / ArrayColdData | 88 / 32 |
| FunctionData | 304 |
| IC entry / handle | 32 / 8 |
| debug site state | 88 |
| Value / JsString handle | 16 / 8 |
| production StringData on arm64 | 80 |

StringData includes an 8-byte computed-key memo in existing metadata; arm64
DWARF type inspection verifies 80 bytes. IC entries/cold poly budget are unchanged.

## Diagnosis provenance

- `20261030T140601Z-t077-s2-r1-property-profiles`: both roles, four named/
  computed sentinel/selection programs, uninstrumented sample at 1 ms; all eight
  process/sampler exits zero. Shows descriptor-size cache frames and atom recovery.
- `20261030T143230Z-t077-s2-r1-payoff-profiles`: same four programs/roles;
  identifies remaining untyped descriptor route and out-of-line authority closure.
- `20261030T155953Z-t077-s2-r1-development-profiles`: Moment/Graphlib only,
  legal development inputs, source/binary hashes and checksum output retained in
  the job. Sampling is diagnosis, not a timing observation or an acceptance run.

Final per-family [counters](T077-s2-round1-final-counters.md) and
[source pins](T077-s2-round1-source.md) own work counts and artifact identities.
