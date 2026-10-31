# S2 round-2 construction target and migration

Base is accepted main `dcee3c56` (runtime `71294a20`); pre-repair candidate
`28206401` merges S3a. Before jobs: `20261030T174905Z-t077-s2-r2-before-triage`
and `20261030T180431Z-t077-s2-r2-construction-before`. No held-out input.

## Evidence-selected target

- Restore allocation-free one/two-value storage inside the sole NamedStore.
  Base has inline two-value literals; S2 allocates a separate value buffer.
  Larger stores use the same heap vector, never a second descriptor authority.
- Store-add plans guard old shared layout, receiver semantics/extensibility and
  the prototype validity generation for Set. A hit appends the live value and
  adopts the next layout, without key recovery, ID probe or transition lookup.
- Construction plans may retain shared **string-only** schema metadata, like
  existing literal links. Prototype identities remain weak; symbols, dictionary
  receivers and exotic semantics decline. Read ICs remain weak. This bounded
  code-owned metadata prevents weak transition recipes disappearing when the
  shared retention budget is consumed by live metadata; it pins no JS value.
- Reuse class definition sequences without publishing future fields. Primitive
  observer-free segments reserve capacity once; effectful initializers still
  execute before each individual DefineOwnProperty. Deletes/reconfigurations,
  Proxy/exotic receivers and existing descriptors use the complete fallback.
- JSON parse predicts the preceding schema at each nesting level, compares keys
  in source order and reuses the corresponding shape/slots. Key mismatch or
  duplicates preserve generic first-order/last-value behavior. No reviver or
  user callback observes intermediate construction; reviver traversal remains
  unchanged. Avoid owned key allocation when the input token can be borrowed.

No parser shortcut, dependency, unsafe, FFI, Rust async or binding snapshot.
Shared layouts/validity cells, lease-safe IDs, live edges and byte-bounded shared
retention remain intact. Define each new typed API beside its invariant.

## Stages and verification

1. Inline slots/direct pair construction; focused descriptor/order/type tests.
2. Guarded store-add and class recipes; mutation/observer/Proxy/prototype tests.
3. JSON sequence reuse; duplicates, escapes, reviver source and order tests.
4. Recollect the same counters/instructions and triage inventory against main.
   Every lead-table case must be <=1.00 and development clearly below 1.00.
5. Only then shrink ArrayData to base size (56 bytes) or below, preserving pure
   probes and explicit realm/prototype identity. Full default/off/verify/NG,
   configured exact/burndown, fuzz and RSS complete the owner handoff.

T077 migration rules govern; these are not T022 leaf attempts. Keep every
measurement cohort and its rejection label. Formal four-aggregate/held-out and
resource acceptance remain lead-owned; no timing in the iMac quiet window.
