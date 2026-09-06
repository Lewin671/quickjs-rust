# T031: Realm-owned single-code-unit String values

## Status

Candidate under measurement. Base: `a8e9d253b3ddf8e63e5963e37a5b6ac11cfba632`.
The frozen plan is `performance-units/realm-single-code-unit-strings.json`.
No retained or promotion claim is made before its gates pass.

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
Formal thirty-block measurement and exact-commit Test262 coverage are next.
