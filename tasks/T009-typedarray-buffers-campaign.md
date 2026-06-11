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
- [ ] S3 %TypedArray%.prototype methods, batched by behavior family
      (iteration, copy/fill/set, sort/search, view accessors); one reviewable
      unit per batch.
- [ ] S4 DataView: constructor and get/set accessors with endianness and
      bounds checks.
- [ ] S5 Re-cluster remaining gaps; resizable/growable buffers,
      SharedArrayBuffer, and Atomics stay out of scope until this point and
      get their own slices only if the burndown trend justifies them.

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
