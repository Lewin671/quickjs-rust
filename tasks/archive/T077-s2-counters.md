# T077 S2 deterministic development counters

Candidate `fc5da912bdd646af2ae05f991fe0618afc961195`; predecessor `b2daec46`. Native aarch64 macOS,
`cargo build --release -p qjs-cli --features perf-counters`; each of the ten
hash-verified development bundles executed once with its pinned driver and
validated checksum. No time, external, selection or held-out samples collected.

Manifest SHA-256: `2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.
Candidate diagnostic binary: `f2db9964520bc026e86baaa6019f6909fa05ccedef6258c45fb20b19b89d313e`.
Predecessor diagnostic binary: `b2388f1ae211dff0d018252fda08910f87b2641ca83aea9ae1cfce3dc1e54a3e`.

## Authority work before and after

These are the existing AtomDomain/store counters, not all hashes in the old
runtime: S1 function/array maps were outside those counters. Added family IC
counters have no predecessor equivalent; absent instrumentation is unavailable,
not zero. In particular, this table cannot establish a speedup or a total-name-
hash reduction. Initial key/shape materialization is included in both runs.

| Counter | S1 | S2 | S2 / S1 |
| --- | ---: | ---: | ---: |
| named_property_reads | 54,553,596 | 54,553,592 | 1.0000 |
| property_text_probes | 71,290,034 | 56,250,510 | 0.7890 |
| property_id_probes | 76,273,957 | 58,405,806 | 0.7657 |
| property_key_interns | 1,037,872 | 1,160,194 | 1.1179 |
| property_atoms_created | 1,035,713 | 1,049,956 | 1.0138 |
| property_shape_field_copies | 856,221 | 977,767 | 1.1420 |
| property_transition_hits | 4,364,214 | 4,615,283 | 1.0575 |
| property_transition_misses | 44,253 | 46,773 | 1.0569 |

Text probes are content-index queries at host/domain boundaries; ID probes
count the authority index. The four-read difference is execution-path counting,
not lost checksum work. Higher intern/materialization/field-copy counts are an
open construction-cost risk requiring timing, especially for programs whose
domain text probes increased. No claimed percentage of logical JS access hits.

| Program | Text probes S1 → S2 | ID probes S1 → S2 | Checksum |
| --- | ---: | ---: | --- |
| lodash/transform | 621,965 → 1,916,617 | 621,818 → 671,416 | dd804f67 |
| meriyah/parse | 3,870,436 → 3,971,165 | 4,143,522 → 3,652,860 | 6482be |
| immutable/persistent | 4,598,959 → 5,510,678 | 4,765,188 → 4,596,747 | 2fd9ba00 |
| moment/dates | 25,838,861 → 13,300,229 | 28,994,078 → 22,085,795 | 77a28ccb |
| diff/compare | 22,867,780 → 15,121,466 | 23,530,488 → 15,308,357 | 9cf0f84a |
| papaparse/csv | 8,822,674 → 7,884,546 | 8,834,153 → 6,493,631 | 74046808 |
| js-yaml/document | 2,062,840 → 4,425,847 | 2,421,165 → 2,747,780 | 7f1866a1 |
| big.js/arith | 168,640 → 681,996 | 186,002 → 266,755 | d4940ea1 |
| matter-js/simulate | 93,885 → 993,801 | 92,077 → 60,766 | 8e0e844 |
| graphlib/paths | 2,343,994 → 2,444,165 | 2,685,466 → 2,521,699 | e845d023 |

## S2 family IC events

A hit/miss is an actual IC probe, including separate hot/cold attempts. A probe
is a generic own-property attempt, including a hook decline; it need not enter
the ID index. Hashes count text-index queries used to link a cache miss, not
hasher instructions or all builtin/property hashing. The global authority
counters above cover broader work. The receiver category is the carrier node
consulted, so prototype hits may be counted on intrinsic Object/String carriers.
Typed loops and leaf executors also consult ICs; `named_property_reads` counts
different dispatch events and is not a denominator for these event counts.

| Family | Hits | Misses | Own probes | Text hash queries |
| --- | ---: | ---: | ---: | ---: |
| object | 115,713,720 | 23,035,300 | 17,344,958 | 7,366,178 |
| typed_array | 159 | 1 | 1 | 0 |
| array_prototype | 0 | 7 | 301 | 0 |
| string_wrapper | 3,166,603 | 141,624 | 70,812 | 70,812 |
| array | 973,972 | 18,360,725 | 6,050,010 | 5,934,583 |
| function | 4,586,715 | 554,930 | 830 | 144 |
| map | 20,802 | 2 | 2 | 0 |
| set | 3 | 4 | 4 | 0 |
| exotic | 0 | 0 | 0 | 0 |

Per-program tuple below is **hits / misses / own probes / text hash queries**.
Zero means no such event in these programs, not missing engine support.

| Program | Family | Counts |
| --- | --- | --- |
| lodash/transform | object | 1,488,662 / 1,598 / 1,082 / 514 |
| lodash/transform | typed_array | 0 / 0 / 0 / 0 |
| lodash/transform | array_prototype | 0 / 3 / 20 / 0 |
| lodash/transform | string_wrapper | 0 / 0 / 0 / 0 |
| lodash/transform | array | 80,316 / 3,781,823 / 1,247,226 / 1,247,140 |
| lodash/transform | function | 83,323 / 82,632 / 222 / 42 |
| lodash/transform | map | 0 / 0 / 0 / 0 |
| lodash/transform | set | 0 / 3 / 3 / 0 |
| lodash/transform | exotic | 0 / 0 / 0 / 0 |
| meriyah/parse | object | 5,258,663 / 1,678,947 / 801,303 / 626,761 |
| meriyah/parse | typed_array | 159 / 1 / 1 / 0 |
| meriyah/parse | array_prototype | 0 / 0 / 14 / 0 |
| meriyah/parse | string_wrapper | 367,954 / 14 / 7 / 7 |
| meriyah/parse | array | 25,989 / 157,339 / 63,723 / 44,209 |
| meriyah/parse | function | 109,208 / 7 / 5 / 1 |
| meriyah/parse | map | 20,802 / 2 / 2 / 0 |
| meriyah/parse | set | 0 / 0 / 0 / 0 |
| meriyah/parse | exotic | 0 / 0 / 0 / 0 |
| immutable/persistent | object | 6,058,158 / 50,774 / 121,271 / 25,227 |
| immutable/persistent | typed_array | 0 / 0 / 0 / 0 |
| immutable/persistent | array_prototype | 0 / 0 / 7 / 0 |
| immutable/persistent | string_wrapper | 330,222 / 141,406 / 70,703 / 70,703 |
| immutable/persistent | array | 52,375 / 1,524,548 / 490,731 / 490,727 |
| immutable/persistent | function | 391,785 / 327 / 292 / 22 |
| immutable/persistent | map | 0 / 0 / 0 / 0 |
| immutable/persistent | set | 0 / 0 / 0 / 0 |
| immutable/persistent | exotic | 0 / 0 / 0 / 0 |
| moment/dates | object | 15,082,004 / 12,192,095 / 10,271,826 / 2,940,138 |
| moment/dates | typed_array | 0 / 0 / 0 / 0 |
| moment/dates | array_prototype | 0 / 4 / 4 / 0 |
| moment/dates | string_wrapper | 43,234 / 10 / 5 / 5 |
| moment/dates | array | 0 / 516,048 / 180,046 / 168,001 |
| moment/dates | function | 3,261,886 / 300,284 / 81 / 29 |
| moment/dates | map | 0 / 0 / 0 / 0 |
| moment/dates | set | 0 / 0 / 0 / 0 |
| moment/dates | exotic | 0 / 0 / 0 / 0 |
| diff/compare | object | 11,332,053 / 7,401,782 / 5,089,458 / 3,207,629 |
| diff/compare | typed_array | 0 / 0 / 0 / 0 |
| diff/compare | array_prototype | 0 / 0 / 48 / 0 |
| diff/compare | string_wrapper | 78,374 / 32 / 16 / 16 |
| diff/compare | array | 251,290 / 4,874,635 / 1,595,922 / 1,545,486 |
| diff/compare | function | 24,164 / 8 / 6 / 1 |
| diff/compare | map | 0 / 0 / 0 / 0 |
| diff/compare | set | 0 / 0 / 0 / 0 |
| diff/compare | exotic | 0 / 0 / 0 / 0 |
| papaparse/csv | object | 1,301,192 / 1,380,606 / 920,608 / 460,121 |
| papaparse/csv | typed_array | 0 / 0 / 0 / 0 |
| papaparse/csv | array_prototype | 0 / 0 / 18 / 0 |
| papaparse/csv | string_wrapper | 807,160 / 38 / 19 / 19 |
| papaparse/csv | array | 337,060 / 2,827,017 / 918,257 / 916,572 |
| papaparse/csv | function | 73,728 / 40 / 36 / 4 |
| papaparse/csv | map | 0 / 0 / 0 / 0 |
| papaparse/csv | set | 3 / 1 / 1 / 0 |
| papaparse/csv | exotic | 0 / 0 / 0 / 0 |
| js-yaml/document | object | 9,496,007 / 925 / 430 / 270 |
| js-yaml/document | typed_array | 0 / 0 / 0 / 0 |
| js-yaml/document | array_prototype | 0 / 0 / 31 / 0 |
| js-yaml/document | string_wrapper | 1,508,290 / 90 / 45 / 45 |
| js-yaml/document | array | 44,941 / 429,106 / 142,988 / 137,427 |
| js-yaml/document | function | 463,537 / 170,995 / 55 / 21 |
| js-yaml/document | map | 0 / 0 / 0 / 0 |
| js-yaml/document | set | 0 / 0 / 0 / 0 |
| js-yaml/document | exotic | 0 / 0 / 0 / 0 |
| big.js/arith | object | 119,084 / 320 / 156 / 103 |
| big.js/arith | typed_array | 0 / 0 / 0 / 0 |
| big.js/arith | array_prototype | 0 / 0 / 40 / 0 |
| big.js/arith | string_wrapper | 30,986 / 32 / 16 / 16 |
| big.js/arith | array | 91,620 / 1,108,308 / 340,961 / 340,961 |
| big.js/arith | function | 9,265 / 39 / 13 / 13 |
| big.js/arith | map | 0 / 0 / 0 / 0 |
| big.js/arith | set | 0 / 0 / 0 / 0 |
| big.js/arith | exotic | 0 / 0 / 0 / 0 |
| matter-js/simulate | object | 61,312,729 / 4,507 / 2,404 / 1,311 |
| matter-js/simulate | typed_array | 0 / 0 / 0 / 0 |
| matter-js/simulate | array_prototype | 0 / 0 / 99 / 0 |
| matter-js/simulate | string_wrapper | 383 / 2 / 1 / 1 |
| matter-js/simulate | array | 2,834 / 2,534,945 / 871,748 / 865,318 |
| matter-js/simulate | function | 77,994 / 572 / 101 / 7 |
| matter-js/simulate | map | 0 / 0 / 0 / 0 |
| matter-js/simulate | set | 0 / 0 / 0 / 0 |
| matter-js/simulate | exotic | 0 / 0 / 0 / 0 |
| graphlib/paths | object | 4,265,168 / 323,746 / 136,420 / 104,104 |
| graphlib/paths | typed_array | 0 / 0 / 0 / 0 |
| graphlib/paths | array_prototype | 0 / 0 / 20 / 0 |
| graphlib/paths | string_wrapper | 0 / 0 / 0 / 0 |
| graphlib/paths | array | 87,547 / 606,956 / 198,408 / 178,742 |
| graphlib/paths | function | 91,825 / 26 / 19 / 4 |
| graphlib/paths | map | 0 / 0 / 0 / 0 |
| graphlib/paths | set | 0 / 0 / 0 / 0 |
| graphlib/paths | exotic | 0 / 0 / 0 / 0 |

The focused `own_hit_has_no_id_probe_text_hash_or_descriptor_path` test
independently proves one guarded own hit performs zero authority ID or text
queries. Counters compile out of timing binaries. The frozen candidate still
needs iMac triage against `dd0905b2` and lead-owned four-aggregate acceptance.
The runner sends temporary git refs; no-push clarification is pending, and
this owner submitted no timing job.
