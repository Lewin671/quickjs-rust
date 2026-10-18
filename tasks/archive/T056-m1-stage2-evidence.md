# T056 M1 stage 2: layout authority

Owner `agent/t056-m1-stage2/codex`, base `ab9c29b6`; no public push or merge.
The frozen plan advances only `migration.current_stage` to 2 (base remains
`351004f3`, cumulative cap 1.025, one screen attempt, governing v4 successor).

`property/layout.rs` owns shared immutable shapes keyed by exact atom-entry
identities, descriptor kind/attributes, slot order and payload encoding. The
per-domain shape cache holds weak references with bounded sweeping; each shape
holds at most 16 weak transition hints. After 32 structural publications a
receiver uses a private dictionary identity/version; rollover replaces its
identity rather than wrapping. Dictionary inserts, descriptor edits and swap
removal update the atom index in place. Order tombstones/slot positions compact
above `2 * live + 64`, giving bounded, amortized constant-time churn without
scanning every key on insertion/deletion. Value writes keep the layout unchanged.
Both atom-slot and weak-shape-cache indices use the existing internal hasher.

`value/object/storage.rs` retains Small/Dynamic/Shaped/ShapedPair payloads under
one authority; Dynamic's old string index and duplicate order list are removed. Define/delete/reorder,
seal/freeze, class-field installation and lazy String index installation
publish metadata and slots under one storage borrow. Old revisions and cache
policies remain live for M1.3. Function/array installed named descriptors use
`property/named_map.rs`, whose mutation guard tracks edited slots (a cold set
only for multiple distinct slots) and publishes before releasing its borrow;
bulk iteration publishes once; implicit/exotic hooks remain receiver-owned. Prototype-derived objects
and the installed realm prototypes/global retain their realm domain; host/null
roots retain owned domains and foreign inputs resolve by exact content.

Storage needs owned key identities, not finite relocation integers. `AtomKey`
and numbered `Atom` share one interned entry; only bytecode linking requests a
checked monotonic ID. This avoids adding limit panics to infallible storage
APIs. A forced-exhaustion test proves storage keys stay valid while a new
numbered request returns the existing explicit error. No unsafe, dependencies,
public APIs or binding/cell changes. ObjectData's test ceiling changes from
112 to 144 bytes for its uniform layout header; dictionaries remain boxed.
Module namespace live aliases normalize the exported property name too (not
its binding name), so canonical lookup still reaches the live cell. A focused
module test first reproduced `undefined:undefined:undefined` instead of
`1:2:2` before that fix. The array adapter also exposes its stored string
insertion order instead of sorting named properties alphabetically; the
focused `b,a` / delete-reinsert test reproduced `a,b` before this correction. The ordinary Get/Set/cache migration remains M1.3.

## Correctness verification

Initial implementation `c5dfa1c6` passed the full gate (2,316 runtime tests),
NG 249/249 and its pre-commit fast gate (2,316 tests and 52 selected Test262
cases). The final incremental-dictionary source tree based on that commit,
2026-10-18 UTC (M1 Pro, correctness only; iMac occupied by another timed job):

- `PYTHONDONTWRITEBYTECODE=1 ./scripts/check.sh`: pass. Runtime 2,320 tests
  (21 added over the branch base), CLI 13, lexer 63, parser 274, Unicode 59;
  agents 21 and perf-counters 24. Benchmark tools 636, scripts 35; context,
  size, format, all three clippy configurations, plans and hook isolation pass.
- Full curated Test262 subset: 5,170/5,170. No allowlist, expected-failure,
  Test262 configuration or submodule-pin changes.
- `compare-qjs.sh`: 249/249 on the fixed agents debug image below, including
  large lazy String indices/freeze and sparse-array shrink/moved named slots.
- Focused coverage: `property/{atom,layout}/tests.rs`, `property/named_map.rs`,
  `value/object/layout_tests.rs`, `tests/objects/named_keys.rs`, `module/tests.rs`.
  All payloads, constructor/literal/native wrappers, canonical/lone-surrogate
  keys, descriptor kinds/attributes, delete/reinsert, prototype edits,
  accessor/Proxy re-entry, array/string/function/typed-array/namespace hooks,
  foreign-domain lookup, escaping domains and 10,000-key/object churn are covered.
  Tests verify shared/weak shape retention, in-place reserved dictionary buffers,
  slot remapping, version/identity rollover, order compaction and mutation batches.
- Fixed local exact-scan image SHA-256:
  `0f8a1925bc4137c0a4e60bbe4652791c3758f16de4bd14754e41062786715b85`
  (`target/t056-m1s2-exact-incremental-qjs`, debug + agents). The complete
  `--exact --all --no-recommend` scan uses it with `QJS_AGENTS=1` and four workers.

The added sparse-array shrink discriminator finds a pinned-reference mismatch:
after 80 configurable indices at 1,000,000..1,000,079, `length = 0`, NG's own-name
list retains 1,000,041..1,000,079 before `length,b,a`; Rust returns `length,b,a`.
The strict assertion stays in the Rust regression; the NG fixture checks the
common indexed reads, enumerable keys and moved named slots. No reference source,
configuration or existing fixture is excluded/changed to conceal this difference.

Follow-up fast gate, complete agents exact scan and sole iMac screen are recorded
below once complete. Earlier partial exact scans stopped during corpus metadata
collection for implementation refinements are not conformance records; no screen
has run. No M1.3 routing or cache retirement is included.
