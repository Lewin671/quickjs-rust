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

- [ ] S1 ArrayBuffer core: constructor, `byteLength`, `slice`, species
      handling, detach semantics used by the rest of the family.
- [ ] S2 %TypedArray% constructors: the nine concrete constructors, from
      buffer/length/array/iterable, indexed element access with canonical
      numeric index semantics.
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
