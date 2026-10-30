# T077 S2 round-1 final counters

Engine `e1d8a6a3`; deterministic work counts, no timing/held-out inputs.
All ten pinned development checksums pass on candidate and dd0905b2.
Binary/input pins: [source checkpoint](T077-s2-round1-source.md).
Initial round-1 counts remain [frozen separately](T077-s2-round1-counters.md).

Cells are hit/miss **attempts**, not semantic lookup counts; cold fallbacks may
record several misses before one resolution. Stable 600-site/20-round class
reads have 11,400 hits out of 12,000 semantic reads (95%); the event misses
include those repeated cold attempts. No universal hit-rate claim is made.

| case | object | typed array | Array.prototype | String wrapper | array | function | Map | Set | exotic |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| development/lodash/transform | 1488639/880 | 0/0 | 0/9 | 0/0 | 1327478/188 | 83389/396 | 0/0 | 2/6 | 0/0 |
| development/meriyah/parse | 5486153/413149 | 159/3 | 0/0 | 367954/21 | 89690/67 | 109208/15 | 20802/6 | 0/0 | 0/0 |
| development/immutable/persistent | 6008164/226075 | 0/0 | 0/0 | 400914/33 | 543084/66 | 391746/894 | 0/0 | 0/0 | 0/0 |
| development/moment/dates | 15213987/11400298 | 0/0 | 0/12 | 43234/15 | 180041/15 | 3261902/117 | 0/0 | 0/0 | 0/0 |
| development/diff/compare | 13831687/107649 | 0/0 | 0/0 | 78374/48 | 1846800/1237 | 24164/18 | 0/0 | 0/0 | 0/0 |
| development/papaparse/csv | 1761305/789 | 0/0 | 0/0 | 807160/57 | 1255257/180 | 73730/90 | 0/0 | 3/3 | 0/0 |
| development/js-yaml/document | 9496013/1188 | 0/0 | 0/0 | 1508290/135 | 187879/151 | 463537/138 | 0/0 | 0/0 | 0/0 |
| development/big.js/arith | 119084/374 | 0/0 | 0/0 | 30986/48 | 432517/203 | 9265/39 | 0/0 | 0/0 | 0/0 |
| development/matter-js/simulate | 61280165/37668 | 0/0 | 0/0 | 383/3 | 874495/263 | 78037/36 | 0/0 | 0/0 | 0/0 |
| development/graphlib/paths | 4344943/107484 | 0/0 | 0/0 | 0/0 | 285917/115 | 91825/54 | 0/0 | 0/0 | 0/0 |
| review/root_churn | 1999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/9 | 0/0 | 0/0 | 0/0 |
| review/root_nochurn | 1999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/12 | 0/0 | 0/0 | 0/0 |
| review/sites_class_200 | 3800/600 | 0/0 | 0/0 | 0/0 | 4019/3 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_class_300 | 5700/900 | 0/0 | 0/0 | 0/0 | 6019/3 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_class_600 | 11400/1800 | 0/0 | 0/0 | 0/0 | 12019/3 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_array_300 | 0/0 | 0/0 | 0/0 | 0/0 | 11719/903 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/sites_array_600 | 0/0 | 0/0 | 0/0 | 0/0 | 23419/1803 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/epoch_poly | 9998/6 | 0/0 | 0/0 | 0/0 | 10244/24 | 0/6 | 0/0 | 0/0 | 0/0 |
| review/epoch_poly2 | 9997/9 | 0/0 | 0/0 | 0/0 | 10244/18 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/arrproto | 999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 999/3 | 0/0 | 0/0 | 0/0 |
| review/fobj_missing | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9999/3 | 0/0 | 0/0 | 0/0 |
| review/fcall | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9998/7 | 0/0 | 0/0 | 0/0 |
| review/fcall_compact | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 9999/3 | 0/0 | 0/0 | 0/0 |
| review/missing | 9999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/getter | 9999/3 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/arr_mem | 0/0 | 0/0 | 0/0 | 0/0 | 299999/6 | 0/0 | 0/0 | 0/0 | 0/0 |
| review/fn_mem | 0/0 | 0/0 | 0/0 | 0/0 | 0/3 | 299999/3 | 0/0 | 0/0 | 0/0 |

## Recovery and construction work

First column is `26fee930`, then final `e1d8a6a3`. Counters include runtime
installation/linking/creation; they are not estimates of eliminated CPU time.

| program | text probes first → final | ID probes first → final | final interns | final atoms created |
| --- | ---: | ---: | ---: | ---: |
| lodash/transform | 585192 → 303874 | 587054 → 588092 | 1313 | 1124 |
| meriyah/parse | 3375737 → 2456551 | 3383886 → 3412488 | 1343 | 1034 |
| immutable/persistent | 2458816 → 1774343 | 2463548 → 2613686 | 126092 | 125667 |
| moment/dates | 17845642 → 10973682 | 22894549 → 24123349 | 339666 | 339503 |
| diff/compare | 10190220 → 10163183 | 9071901 → 9071901 | 105519 | 105330 |
| papaparse/csv | 5171432 → 5164401 | 5183241 → 5183241 | 730 | 589 |
| js-yaml/document | 1816828 → 1808037 | 2179608 → 2199268 | 885 | 629 |
| big.js/arith | 341761 → 299700 | 336826 → 336826 | 578 | 512 |
| matter-js/simulate | 63234 → 61439 | 60577 → 60577 | 1915 | 1498 |
| graphlib/paths | 2289840 → 2272328 | 2417065 → 2431631 | 131418 | 131294 |

- Weak static-name links remove repeated text recovery at megamorphic sites.
  A focused 1,000-read test records zero text probes/atoms and 1,000 ID probes.
- Shared-own poly and monomorphic numeric hits load live slots with zero key
  or descriptor probes; dictionary/type changes retain guards and bailout.
- Ten thousand absent reads create no key; empty native contexts share per
  domain and detach on mutation. Neither weak site links nor IC entries pin keys,
  receivers or domains. JsString memo ownership is a deliberate live key lease.
- Moment still has many object misses; the table preserves those counts.
- The 300,000-receiver array/function programs each retain 299,999 family hits,
  while probes leave array cold storage and unrelated function defaults absent.

Source/corpus fingerprints (SHA-256 over sorted case + source hash lines):
`7d219976af0c575f5334fc1fe4cb79ea9c44e5683f5ea9b9d9a360e3d1b5d5bc`.
Manifest SHA-256: `2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.
