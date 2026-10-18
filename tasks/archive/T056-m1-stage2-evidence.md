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
cases). The final incremental-dictionary source at `873c9bc5`,
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
common indexed reads, enumerable keys and moved named slots. Reference sources/configuration and the 248 pre-existing baseline fixtures
are unchanged.

Follow-up fast gate, complete agents exact scan and sole iMac screen are recorded
below once complete. Earlier partial exact scans stopped during corpus metadata
collection for implementation refinements are not conformance records; no screen
has run. No M1.3 routing or cache retirement is included.

## Candidate and queued screen

Final runtime candidate: `873c9bc54545dc7488e309cb7712cce5b1a303c0` (incremental
dictionary edits, sole ordering authority and slot-tracked named-map mutation).
Its pre-commit fast gate passes 2,320 runtime tests, context/fmt/clippy/size;
no focused Test262 filter matched those follow-up paths. The full gate provides
the 5,170-case curated subset; a fresh full-gate replay at this commit passed, including all 2,320 runtime tests.
Rebuilding its agents debug executable is byte-identical to the fixed local
exact-scan image named above. The final NG fixture set passes 249/249.

Queued job `20261018T055933Z-t056-m1s2-screen1`, submitted with `run --timing`
after checking host status; no other job was cancelled. It waits for a
`correctness-ready.json` receipt before running `perf-loop.sh`: receipt source
must equal this candidate, full gate/NG/subset must pass, and the complete
agents exact scan must pass all 42,672 configured cases. The receipt has
been released after the complete exact scan passed (see below). Output is the queue job's `out/stage2/` directory.

The fixed base remains `351004f3`; current stage is 2/3, cumulative cap 1.025,
one valid screen, K=6. The collector loads the governing v4 successor from the
frozen v3 plan. No formal continuation, promotion or M1.3 is started here.
There is no performance verdict yet.

## Complete exact scan and receipt

At `873c9bc5`, `QJS_AGENTS=1`, four workers, fixed debug + agents image:
`find-qjsng-gaps.sh --exact --all --no-recommend` completed with exit 0.
Total 53,572; configured 42,672; NG-config skipped 10,900. Rust pass 42,672,
fail 0, timeout 0, not-run 0. NG pass 42,602, fail 70, timeout 0; actionable
gap 0. The complete `local-exact-agents` burndown entry is in the ledger.

- Summary SHA-256: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
- Cases SHA-256: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
- Full-gate log SHA-256: `f2a7b9be49849b55c798374b6cbb5e7cb021328b0bb3fa93447eb95d3a31682d`.
- Correctness receipt SHA-256: `c0bb2a22c40feedf00e713453db4cb7708004893020d43da7c79e5ace4c6192a`.

The receipt was atomically uploaded to this job's `out/correctness-ready.json`
on 2026-10-18 UTC, after source-to-candidate diff and exact-image identity checks.
The job is allowed to start its sole screen when the host queue reaches it.
No source change follows the candidate; later commits record evidence only.
