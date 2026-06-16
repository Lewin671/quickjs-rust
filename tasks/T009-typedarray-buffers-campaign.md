# T009: TypedArray and Buffers Campaign

## Goal

Implement ArrayBuffer, the TypedArray family, and DataView. At commit
`3e7feb0` (2026-06-09 full scan) this cluster carries roughly 2,500 actionable
gaps: `TypedArray` 2,027, `ArrayBuffer` 263, `DataView` 188 — concentrated in
`test/built-ins/TypedArray` (1,356), `test/built-ins/DataView` (561), and
`test/built-ins/TypedArrayConstructors` (445).

## Evidence

- Top actionable areas from the full scan; no structural not-run involvement,
  so progress is measurable with focused gap runs alone.
- Later extensions tagged separately: `resizable-arraybuffer` 449,
  `SharedArrayBuffer` 375, `Atomics` 283.

## Slices

- [x] S1 ArrayBuffer core: constructor, `byteLength`, `slice`, species
      handling, detach semantics used by the rest of the family.
      `ArrayBuffer.isView`, `ArrayBuffer[Symbol.species]`, brand checks, a
      detached internal flag, and clean rejection of `maxByteLength`
      (resizable/growable buffers) are in place. `test/built-ins/ArrayBuffer`
      moved from 46 to 63 passing.
- [x] S2 %TypedArray% constructors: the eleven concrete constructors
      (nine numeric plus BigInt64/BigUint64, since BigInt support made them
      cheap) share the `%TypedArray%` intrinsic as their `[[Prototype]]`, with
      `%TypedArray.prototype%` as the shared prototype of each concrete
      `.prototype`. `%TypedArray%` is not directly callable or constructable.
      Construction supports no-args / length / typed array / buffer +
      byteOffset + length (with alignment and bounds validation) / iterable /
      array-like, backed by a real `ArrayBuffer`. Instance accessors
      `buffer`/`byteLength`/`byteOffset`/`length` and the prototype
      `Symbol.toStringTag` getter brand-check their receiver; detached buffers
      report zero and throw on construction. Per-type element conversion
      (integer wrapping, float rounding, Uint8Clamped round-half-even, BigInt
      wrapping) runs at construction. Indexed element reads stay materialized
      as own properties; indexed *writes* through `array[i] = v` do not yet
      route per-type conversion through the buffer (needs a VM exotic-index
      hook, out of this slice's path boundary) — tracked for S3.
- [x] S3 %TypedArray%.prototype methods, batched by behavior family
      (iteration, copy/fill/set, sort/search, view accessors); one reviewable
      unit per batch. Two commits on `%TypedArray.prototype%`: batch 1
      (iteration/read) — `at`, `indexOf`, `lastIndexOf`, `includes`, `join`,
      `keys`/`values`/`entries` (`Symbol.iterator` aliased to `values`),
      `forEach`, `map`, `filter`, `reduce`, `reduceRight`, `some`, `every`,
      `find`, `findIndex`, `findLast`, `findLastIndex`, `slice`, `subarray`,
      `toString`, `toLocaleString`; batch 2 (write/order) — `set` (array-like
      and typed-array sources with offset/range checks), `fill`, `copyWithin`,
      `reverse`, `sort` (default numeric ordering, NaN last, `-0` before `+0`,
      stable; comparator supported), `toReversed`, `toSorted`, `with`. All
      methods are brand-checked, reject detached buffers, and route reads/writes
      through the backing buffer; writes also refresh the materialized index
      property (indexed `array[i] = v` still bypasses conversion — a VM hook out
      of scope). `map`/`filter`/`slice`/`subarray`/`toReversed`/`toSorted`/
      `with` build a new typed array of the receiver's concrete kind (species
      via the concrete constructor, since per-instance `Symbol.species`
      machinery is not yet wired for typed arrays). `subarray` currently copies
      its range rather than aliasing the shared buffer (a shared-view slot would
      replace the copy). BigInt arrays reject `Number` in `set`/`fill`/`with`,
      and mixed BigInt/Number `set` throws. `test/built-ins/TypedArray/prototype`
      moved from 44 to 196 passing (limit 800 after-scan).
- [x] S4 DataView: `new DataView(buffer [, byteOffset [, byteLength]])` over an
      `ArrayBuffer` (TypeError for non-buffers; SharedArrayBuffer absent),
      `ToIndex` coercion with RangeError on OOB offset/length and detach
      re-checks ordered per spec. Prototype `buffer`/`byteLength`/`byteOffset`
      accessors brand-check and throw on detached (for the two byte accessors).
      All ten element families have `get*`/`set*` with a `littleEndian` flag
      (default big-endian), `ToIndex` offset, RangeError on OOB, and per-spec
      coercion order — `set*` coerces the value (`ToNumber`/`ToBigInt`) before
      the detach/bounds checks. `Symbol.toStringTag` is a `"DataView"` data
      property (writable false, configurable true). Byte encode/decode is local
      via big-endian `to_be_bytes` with a reversal for little-endian.
      `test/built-ins/DataView` moved from 0 to 369 passing (--all scan). The
      remaining failures are out of this slice: `getFloat16`/`setFloat16` (38,
      Float16 proposal), `*-sab.js` (SharedArrayBuffer), `resizable-array-buffer`
      cases, and `detached-buffer` cases that need a JS-facing
      `$DETACHBUFFER`/detach hook in `ArrayBuffer` (tracked for S1/S5); the
      DataView-side detach guards are already in place.
- [ ] S5 Re-cluster remaining gaps; resizable/growable buffers,
      SharedArrayBuffer, and Atomics stay out of scope until this point and
      get their own slices only if the burndown trend justifies them.
      SharedArrayBuffer has been re-clustered through growable construction,
      `byteLength`/`maxByteLength`/`growable`, `grow`, and `slice`;
      `test/built-ins/SharedArrayBuffer --all` now reports 0 actionable gaps.
      Atomics now has the non-blocking object surface plus `load`/`store`,
      read-modify-write operations, `compareExchange`, `isLockFree`, `pause`,
      and no-waiter `notify`; `test/built-ins/Atomics --all` moved from 289
      to 80 actionable gaps, leaving wait queue/agent behavior.
      `%TypedArray%.prototype.map`/`filter` now allocate their result through
      `TypedArraySpeciesCreate` (the `@@species` constructor / default), read
      source elements live each step, and `Set` mapped/kept values through the
      result's element coercion: `filter` moved 40 -> 4 and `map` 32 -> 5
      actionable gaps, leaving only resizable/immutable-buffer variants.
      `subarray` now allocates through `TypedArraySpeciesCreate(O, « buffer,
      beginByteOffset, newLength »)` and only brand-checks O up front (a
      detached/out-of-bounds view yields srcLength 0, still coerces start/end,
      then construction throws): 24 -> 2 actionable gaps, leaving
      resizable-buffer variants.
      `toLocaleString` now Invokes each element's own `toLocaleString` (resolved
      through `Number.prototype`, so an overridden one is honored) and
      ToString-coerces the result, matching `Array.prototype.toLocaleString`:
      19 -> 5 actionable gaps. The remaining `calls-tostring`/`calls-valueof`
      variants fail on a separate closure-binding bug (a function that assigns a
      free `var` desyncs a previously-captured closure's view of it, tracked in
      T014), not the typed-array surface; the last is a resizable-buffer variant.
      `some`/`every`/`find`/`findIndex`/`findLast`/`findLastIndex`/`reduce`/
      `reduceRight` now read each element live via `get_view_element` instead of
      a one-shot `ViewSnapshot`, so callbacks that mutate a not-yet-visited index
      are observed (`values-are-not-cached`, `predicate-call-changes-value`):
      -2 actionable gaps per method (16 total). `ViewSnapshot` is now unused and
      removed. Remaining per-method gaps are resizable-buffer mid-iteration
      variants.

## Scope

- Allowed paths: `crates/qjs-runtime/**` (group builtins under
  `typed_array/` and `array_buffer/` behavior-family modules).
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; S1 integrates before S2-S4, which can
  proceed in parallel worktrees.

## References

- `docs/architecture.md`
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` (typed array sections).
- Test262: `test/built-ins/TypedArray/**`, `test/built-ins/ArrayBuffer/**`,
  `test/built-ins/DataView/**`, `harness/testTypedArray.js`,
  `harness/detachArrayBuffer.js`.

## Acceptance Criteria

- Harness includes used by these suites (`testTypedArray.js`,
  `detachArrayBuffer.js`) run under the baseline so the cases execute rather
  than skip.
- Each slice reduces the focused gap count for its subtree.
- Campaign exit: `TypedArray` leaves the top-three feature clusters in the
  actionable gap report.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/built-ins/TypedArray --all
./scripts/find-qjsng-gaps.sh --filter test/built-ins/DataView --all
./scripts/check.sh
```

## Notes

No `unsafe`: back buffers with plain byte vectors and explicit ownership in
runtime data structures. Many TypedArray cases are stress-sized; keep the
default stress-timeout exclusion in mind when reading focused gap reports.
