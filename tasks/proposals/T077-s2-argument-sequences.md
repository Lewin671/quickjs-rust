# S2 observer-free descriptor sequences

Freeze before implementation. In the 5f509fb9 non-timing construction cohort
`20261030T202617Z-t077-s2-r2-arguments-constructors`, graphlib still performs
379,914 store-adds, 324,017 schema upgrades and 1,517,688 storage write borrows,
although its arguments layouts hit 50,706 times. Graphlib instructions remain
1.02536 main. Retention fixes schema churn but still walks the transition chain
for every per-call arguments object.

Extend the existing ordered DataSequence builder to accept descriptors and
symbol keys. A prediction checks each next key identity/text and descriptor
flags, then fills owning slots/accessors without interning, transitions or
per-property storage borrows. On mismatch reconstruct only the completed
prefix and resume common NamedStore insertion. Never expose the unfinished
object. JSON remains a data-only caller; arguments creation is observer-free
until it returns, and still creates every mapped cell/accessor and resolves
realm/cross-realm callee/iterator values through existing helpers.

The bounded realm arguments cache predicts a recent matching-width intrinsic
layout. Every individual descriptor is checked; a width prediction is not a
semantic proof. Reject foreign domain/prototype predictions. Retain only the
existing allowed intrinsic symbol metadata, no values or receiver edges. Transfer
the prediction's single layout reference to the finished arguments store; do
not clone another layout per receiver merely to return an unused prediction.
No constructor/class effectful initializer may use this publication scheme.

Test mismatching keys/flags/symbols, partial sequences, accessor identity and
value changes, mapped/strict/rest/duplicate parameters, cross-realm throwers and
iterator identity. Repeat counts/triage and final correctness gates. No unsafe,
new dependency, binding snapshot or S3 API change; no held-out work.
