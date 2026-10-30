# T077 S2 round-1 deterministic counters

Candidate `26fee930` versus rebuild base `dd0905b2`; initial S2 diagnostic
artifact `884d3ba2` is an additional before-state, not an accepted milestone.
All ten development checksums pass on both new binaries. No timing/held-out
inputs are included. Each cell below is hit/miss events; cold resolution may
record more than one miss attempt per eventual lookup. Zero is written `0/0`.

Candidate binary SHA-256: `238c030a942fbf24ce117d9c473ec0f3e0b337d434d57f339d7e908127b01045`.
Base binary SHA-256: `4225a79c2c619b77de755ebc502df9cd913c82c2a8c44df96efc3312622274b6`.
Development manifest SHA-256: `2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.

Program bundles reuse the pinned development manifest and hash-verified
bundles from the [initial receipt](T077-s2-counters.md). Review scripts reuse
`/tmp/qjs-s2-review` inputs; stable-shape results below are recorded here so
their conclusions survive temporary-input cleanup. IC counter fields did not
exist at dd0905b2; its zero values are unavailable instrumentation, not zero work.

## Candidate hits/misses by receiver family

| program | object | typed array | Array.prototype | String wrapper | array | function | Map | Set | exotic |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| development/lodash/transform | 1488639/1720 | 0/0 | 0/6 | 0/0 | 1327478/170 | 83389/289 | 0/0 | 2/4 | 0/0 |
| development/meriyah/parse | 5486153/1237164 | 159/2 | 0/0 | 367954/14 | 89690/47 | 109208/11 | 20802/4 | 0/0 | 0/0 |
| development/immutable/persistent | 6008164/200911 | 0/0 | 0/0 | 400914/22 | 543084/62 | 391746/706 | 0/0 | 0/0 | 0/0 |
| development/moment/dates | 15213987/16847705 | 0/0 | 0/8 | 43234/10 | 180041/12 | 3261902/97 | 0/0 | 0/0 | 0/0 |
| development/diff/compare | 13831687/206374 | 0/0 | 0/0 | 78374/32 | 1846800/1162 | 24164/13 | 0/0 | 0/0 | 0/0 |
| development/papaparse/csv | 1761305/615 | 0/0 | 0/0 | 807160/38 | 1255257/145 | 73730/64 | 0/0 | 3/2 | 0/0 |
| development/js-yaml/document | 9496013/1045 | 0/0 | 0/0 | 1508290/90 | 187879/120 | 463537/113 | 0/0 | 0/0 | 0/0 |
| development/big.js/arith | 119084/339 | 0/0 | 0/0 | 30986/32 | 432517/179 | 9265/39 | 0/0 | 0/0 | 0/0 |
| development/matter-js/simulate | 61313212/4126 | 0/0 | 0/0 | 383/2 | 874495/237 | 78037/31 | 0/0 | 0/0 | 0/0 |
| development/graphlib/paths | 4344943/73548 | 0/0 | 0/0 | 0/0 | 285917/87 | 91825/40 | 0/0 | 0/0 | 0/0 |
| review/root_churn | 1999/2 | 0/0 | 0/0 | 0/0 | 0/0 | 0/6 | 0/0 | 0/0 | 0/0 |
| review/root_nochurn | 1999/2 | 0/0 | 0/0 | 0/0 | 0/0 | 0/8 | 0/0 | 0/0 | 0/0 |
| review/sites_class_200 | 3800/600 | 0/0 | 0/0 | 0/0 | 4019/2 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_class_300 | 5700/900 | 0/0 | 0/0 | 0/0 | 6019/2 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_class_600 | 11400/1800 | 0/0 | 0/0 | 0/0 | 12019/2 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_array_300 | 0/0 | 0/0 | 0/0 | 0/0 | 11719/902 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_array_600 | 0/0 | 0/0 | 0/0 | 0/0 | 23419/1802 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/epoch_poly | 9998/6 | 0/0 | 0/0 | 0/0 | 10244/16 | 0/4 | 0/0 | 0/0 | 0/0 |
| review/epoch_poly2 | 9997/9 | 0/0 | 0/0 | 0/0 | 10244/12 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/arrproto | 999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 999/2 | 0/0 | 0/0 | 0/0 |
| review/fobj_missing | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9999/3 | 0/0 | 0/0 | 0/0 |
| review/fcall | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9998/5 | 0/0 | 0/0 | 0/0 |
| review/fcall_compact | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9999/3 | 0/0 | 0/0 | 0/0 |
| review/missing | 9999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/getter | 9999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/arr_mem | 0/0 | 0/0 | 0/0 | 0/0 | 299999/4 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/fn_mem | 0/0 | 0/0 | 0/0 | 0/0 | 0/2 | 299999/2 | 0/0 | 0/0 | 0/0 |

## Before/after development work

IC totals include every receiver family above. The initial S2 binary hashes
and detailed before counters remain frozen in the linked initial receipt.

| program | initial S2 hits/misses | corrected hits/misses | atoms base → corrected | text probes corrected | ID probes corrected |
| --- | ---: | ---: | ---: | ---: | ---: |
| lodash/transform | 1652301/3866059 | 2899508/2189 | 360 → 1124 | 585192 | 587054 |
| meriyah/parse | 5782775/1836310 | 6073966/1237242 | 77 → 1039 | 3375737 | 3383886 |
| immutable/persistent | 6832540/1717055 | 7343908/201701 | 168 → 125667 | 2458816 | 2463548 |
| moment/dates | 18387124/13008441 | 18699164/16847832 | 208 → 339503 | 17845642 | 22894549 |
| diff/compare | 11685881/12276457 | 15781025/207581 | 117 → 105440 | 10190220 | 9071901 |
| papaparse/csv | 2519143/4207702 | 3897455/864 | 119 → 589 | 5171432 | 5183241 |
| js-yaml/document | 11512775/601116 | 11655719/1368 | 97 → 629 | 1816828 | 2179608 |
| big.js/arith | 250955/1108699 | 591852/589 | 55 → 512 | 341761 | 336826 |
| matter-js/simulate | 61393940/2540026 | 62266127/4396 | 332 → 1498 | 63234 | 60577 |
| graphlib/paths | 4444540/930728 | 4722685/73675 | 68 → 131294 | 2289840 | 2417065 |

Missing-name reads do not create atoms: a focused 10,000-read test proves
find-only lookup. Atoms include installed properties and linked operands; the
broader NamedStore authority needs more retained property leases than the base.
Moment's object hit rate remains below the other development programs; this is reported, not hidden by an aggregate.
Packed-array and unrelated function probes retain no cold/value storage; the
300,000-receiver review programs show 299,999 array/function hits respectively.
