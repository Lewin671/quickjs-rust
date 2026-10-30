# T077 S1 Array.prototype hook correction

Parent `01d25055`; source commit is bound by T077's resume block and the final
verification receipt. This is an S1 exotic-boundary correction, not S2 array
named-store migration. Array.prototype retains one ObjectRef NamedStore.

## Reason and implementation

An additional differential discriminator found `Array.prototype.length += -1`
returned `-1` instead of throwing RangeError. The intrinsic is represented by
ObjectRef, so it needs array length/index [[DefineOwnProperty]] before ordinary
named storage. The raw setter previously extended length for indices but omitted
length validation, shrink/delete and non-writable length rejection; ordinary
creation and slot-write caches could bypass that partial hook too.

`object/array_prototype.rs` now shares the existing ArraySetLength coercion
helper, validates descriptors, deletes indices in descending order on shrink,
restores failed shrink length to the undeletable index plus one, and applies the
requested read-only bit even on partial failure. Index definitions extend length
and reject extension when length is read-only. The interpreter, Reflect receiver
and descriptor paths dispatch this hook. Ordinary creation, compact constructor
proof and slot-write caches decline this receiver. Raw ObjectRef storage no
longer contains a separate index-extension rule.

Focused tests cover invalid lengths after warmed writes, new index attributes,
successful shrink, nonconfigurable-index partial shrink, read-only extension,
distinct Reflect receivers, reentrant coercion and assignment's read-only check
before coercion. The differential fixture checks the shared NG behavior; the
read-only assignment coercion discriminator is a separate runtime test because
this NG revision coerces its RHS despite returning false.

## Verification

Final working tree, 2026-10-30 UTC: full `check.sh` passes (2,414 default
runtime tests, 753 benchmark-tool tests, 35 script tests and 5,170 subset cases).
`cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet` passes
2,454/2,454; `./scripts/compare-qjs.sh` passes 258/258. The x86_64 cross-check
with agents/perf-counters passes production size assertions. T077 binds these
results to the source revision; the final receipt will bind the exact scan. The abandoned pre-correction exact attempt was stopped during
release compilation, before any case result or complete summary; it establishes
no conformance standing and creates no burndown record.

No new dependency, unsafe code, second index authority, ArrayRef snapshot or
observable fallibility ABI was introduced. No push, merge or iMac job occurred.
