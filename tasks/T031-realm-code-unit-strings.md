# T031: Realm-owned single-code-unit String values

## Status

Retained after formal same-host promotion. Code commit:
`5702c78900d20317e765711a06e2659d774d2014`. Base: `a8e9d253b3ddf8e63e5963e37a5b6ac11cfba632`.
The frozen plan is `performance-units/realm-single-code-unit-strings.json`.
The frozen promotion gate passed for all 76 watched comparisons. This is a
unit-level result on the recorded macOS series, not whole-engine superiority.

## Evidence and selection

The exact base's Linux queue ranked `string-tagcloud` third. A new complete
same-host macOS candidate/base/NG baseline ranks it fourth at 2.994x NG; its
queue SHA-256 is
`b49679df5916ffe237b2c94243f26317db8415523bf2de6f98b54758f429497e`.
The standard-recipe base executable SHA-256 is
`53ec042ecc48239355c4a2644ae2ec0dc7bc6104e4ea8b8e45396e36eafc2fbd`.
All baseline lanes, profiles and content-verified profile receipts are under
`target/performance-high-roi-a8e9d253/` and remain outside commits.

Current profiles place 1,120/7,166 main-thread samples in tagcloud's ordinary
String boxing and 653/7,929 in date-format-xparb's indexed String-data
installation. These inclusive counts are upper bounds, not entirely removable
cost. The common implementation allocates a new immutable string value and
byte buffer for each index even when thousands of indices contain the same
code unit. HashMap's current profile instead spreads cost over dispatch,
frames and property access; it is an independent regression control here.

## Scope and mechanism

Add a lazy, realm-owned cache of at most 256 immutable Latin-1 code-unit
strings. Both `new String(value)` and `Object(string)`/sloppy receiver boxing
reuse those values while installing exactly the same ordinary indexed
properties. Wider code units keep the existing canonical conversion.

The cache creates no global mutable state and holds no realm/object references.
Its table and entries are allocated only on use. Copy-on-write protects cached
values if a program later appends to an extracted character.

This is distinct from the rejected wrapper-source-buffer sharing unit and the
rejected virtual-index representation: the source buffer is still copied,
every indexed descriptor is still materialized, and ordinary ObjectData and
PropertyStorage representations and property semantics are preserved.

Owned paths: the string value factory, realm cache plumbing, two existing
String boxing installers, focused String tests, this task and its frozen plan.
Parser/AST, VM dispatch, slot/upvalue binding representation, dependencies,
allocator choice, and third-party source are outside this unit.

## Verification and decision

- Cover all 256 cached values, wide/surrogate fallback, lazy/bounded retention,
  independent caches, and copy-on-write.
- Cover explicit and implicit boxing, subclasses, indexed descriptors,
  own-key order and Proxy write/delete invariants through runtime tests.
- Run focused String/Object Test262 coverage, touched/full checks, and
  QuickJS-NG comparison tests.
- A diagnostic paired screen may reject an obviously unhelpful candidate; it
  cannot retain it. Formal decisions use at least 30 paired blocks and the
  frozen 5% target improvement / 3% control regression ceilings. All six
  generic sentinels are required; promotion also checks every external/broad
  case and exact-commit complete Test262 parity.
- At most two candidate implementations. Preserve negative evidence and
  revert rather than retuning until a noisy result crosses the threshold.

## Initial diagnostic screen

The first implementation's eleven alternating pairs over the original
hash-verified external bundles report tagcloud 0.8663x candidate/base
(diagnostic 95% interval 0.8563–0.8845), date-format-xparb 0.9834x,
date-format-tofte 0.9910x, HashMap 1.0013x, ordinary object allocation 0.9985x,
and closure allocation 0.9966x. A* is 1.0138x with an interval reaching 1.0372,
so its neutrality still needs the formal cohort. No observation is removed.
The screen remains insufficient for acceptance because it has only eleven
pairs. Raw data and executable hashes are in
`target/performance-high-roi-a8e9d253/screen-r1/`.

All focused String/cache tests, the full local check (including the Test262
subset), and QuickJS-NG comparison fixtures passed before the candidate commit.
Formal thirty-block measurement and exact-commit Test262 coverage followed;
the results are recorded below.


## Formal promotion result

The first implementation passed the unchanged plan on a complete thirty-block
candidate/base/NG run. All 25 broad cases, six generic sentinels and 45 external
cases were present; internal linearity and block health passed with no invalid
blocks. The sealed bundle was replayed before the decision.

| Case | Candidate/base | 95% interval |
| --- | ---: | ---: |
| string-tagcloud | 0.884008 | 0.880853–0.887117 |
| date-format-xparb | 0.973280 | 0.970259–0.975980 |
| date-format-tofte | 0.997173 | 0.995624–0.998673 |
| hash-map | 0.997609 | 0.996017–0.998823 |
| ai-astar | 1.005213 | 1.004098–1.006681 |
| object_allocation | 1.001624 | 0.999283–1.003228 |
| closure_allocation_call | 0.997442 | 0.995372–0.999488 |

The target saves 11.6% wall time; date-format-xparb saves 2.7%. Small costs are
retained transparently: the six generic sentinels increase by 0.3–1.6%, with
upper bounds no higher than 1.019185. The largest non-target point regression
is access-nsieve at 1.021055 (upper 1.023815), within the frozen 1.03 ceiling.
Per-suite external candidate/base geometric means are JetStream subset
0.999320, Kraken 1.001797 and SunSpider 0.995643. This is a targeted allocation
improvement, not an 11.6% whole-engine gain.

The candidate profile's four principal malloc/free stack-top groups total
972/7,574 samples (12.8%), against 1,610/7,166 (22.5%) in the base. These are
diagnostic allocation-cost shares; they are not a resource/RSS benchmark.
The runtime/cache tests cover bounded retention and independent copy-on-write.

Promotion decision: `retained`, no reasons, 76 comparisons. SHA-256:
`443c853b8d93b82e4dd9a988c04972b8fd45bbdcae25c1e2d7f9edfc72e534f1`.
Evidence: `target/performance-high-roi-a8e9d253/candidate-r1-30/` and
`target/performance-high-roi-a8e9d253/promotion-r1.json`.

Candidate [CI](https://github.com/Lewin671/quickjs-rust/actions/runs/34018239503)
and [Test262 coverage](https://github.com/Lewin671/quickjs-rust/actions/runs/34018425750)
passed. The burndown artifact explicitly binds code commit 5702c789: all 42,672
configured cases passed with zero actionable NG gaps, from a 53,572-case pinned
inventory with 10,900 configuration exclusions. The workflow-run page's default
branch SHA is not substituted for the artifact's candidate SHA.
