# T056 M1 stage 2: layout authority

Owner `agent/t056-m1-stage2/codex`, base `ab9c29b6`; no public push or merge.
The frozen plan advances only `migration.current_stage` to 2 (base remains
`351004f3`, cumulative cap 1.025, one screen attempt, governing v4 successor).

`property/layout.rs` owns shared immutable shapes keyed by exact atom-entry
identities, descriptor kind/attributes, slot order and payload encoding. The
per-domain shape cache holds weak references with bounded sweeping; each shape
holds at most 16 weak transition hints. After 32 structural publications a
receiver uses a private dictionary identity/version; rollover replaces its
identity rather than wrapping. Value writes keep the layout unchanged.

`value/object/storage.rs` retains Small/Dynamic/Shaped/ShapedPair payloads under
one authority; Dynamic's old string index is removed. Define/delete/reorder,
seal/freeze, class-field installation and lazy String index installation
publish metadata and slots under one storage borrow. Old revisions and cache
policies remain live for M1.3. Function/array installed named descriptors use
`property/named_map.rs`, whose mutation guard publishes before releasing its
borrow; implicit/exotic hooks remain receiver-owned. Prototype-derived objects
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

Pre-commit source tree based on `ab9c29b6`, 2026-10-18 (M1 Pro, correctness
only; iMac occupied by another timed queue job):

- `PYTHONDONTWRITEBYTECODE=1 ./scripts/check.sh`: pass. Runtime 2,316 tests
  (17 added), CLI 13, lexer 63, parser 274, Unicode 59; agents 21 and
  perf-counters 24. Benchmark tools 636 tests, scripts 35; context, size,
  format, all three clippy configurations, plan checks and hook isolation pass.
- Full curated Test262 subset: 5,170/5,170. No allowlist, expected-failure,
  Test262 configuration or submodule-pin changes.
- `compare-qjs.sh`: 249/249, including the assertion-bearing
  `property-layout-mutations.js`. The final fixture was checked with the same
  prebuilt agents debug executable used by the exact scan.
- Focused coverage lives in `property/{atom,layout}/tests.rs`,
  `property/named_map.rs`, `value/object/layout_tests.rs`,
  `tests/objects/named_keys.rs` and `module/tests.rs`. It covers all payloads,
  constructor/literal/native wrappers, canonical/lone-surrogate keys,
  descriptor kind and every attribute, delete/reinsert order, prototype edits,
  accessor/Proxy re-entry, array/string/function/typed-array/namespace hooks,
  foreign-domain lookup, retained-domain lifetime and 10,000-key churn.
  The churn tests bound atom-table retention by 65--68 entries with fixed live
  keys and verify dictionary versions, shape sharing and identity rollover.

The pre-commit fast gate, complete agents exact scan and sole iMac screen are
recorded below once complete. No M1.3 routing or cache retirement is included.
