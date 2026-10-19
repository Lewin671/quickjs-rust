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
The screen result and verdict follow in "Sole screen: reject" below.

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

## Sole screen: reject (job `20261018T055933Z-t056-m1s2-screen1`)

The frozen plan (`tasks/performance-units/interpreter-core-v2-atoms-layouts.json`,
SHA-256 `5f951b8e46b17ff2eaa855ad7e3025bb94e018e205cfb5f19200416e5acf679d`,
matching the verdict's `unit_sha256`) was screened through
`perf-loop.sh --plan ... --output "$BENCH_OUT/stage2"` on the iMac queue, candidate
`873c9bc54545dc7488e309cb7712cce5b1a303c0` against base `351004f3`. The job first
waited for and accepted `out/correctness-ready.json` (candidate, full gate, 249 NG
fixtures, 5,170 subset, 42,672/42,672 exact), then measured. `layout_iteration`
applied the bound v4 successor (`HELD_OUT_SUCCESSOR`) before evaluating, so this
verdict is already the governing v4 result; unlike stage 1, no replay is needed.

- Exit `3` after 12,385 s (`status` `done 3`). Exit 3 is the screen's measured
  fail verdict, not a tool error: `tools/benchmark/screen.py` returns `3` only
  when a verdict exists and `not verdict["passed"]`, while tool/preview/value/OS
  errors return `1`; `layout_iteration.main` likewise returns `3` for a failed
  layout rule. The log ends with the complete verdict JSON, not a traceback.
- Not `DISTURBED`: the log's closing note is "light background activity in 29
  samples (intruders.log), below the disturbance threshold", and the six panel
  packets record 0 heavy samples and 0 invalid pairs (430 + 2,150 attempts).
  The intruders were single daemons (`mediaanalysisd`, `photoanalysisd`) below
  105% each, never two cores at once.
- Evidence is complete: `label` `reject`, `passed` false, `complete` true,
  `missing` `[]`, `panel_required` true, `event_resolution` null. Six matching
  layouts x 86 cases x 5 pairs; 42 of 86 cases reject, 44 pass, 116 failure lines.
- Workspace copy: `outputs/imac-bench/20261018T055933Z-t056-m1s2-screen1/`.
  `verdict.json` SHA-256 `f3fbf9d76842600f6016cd2ad4cdc03193ba09205a80d57a6c34f3dc342ad713`;
  `screen/complete.json` `5cf250373abaf3f1bc7109e0f51fd44f39691fd6756ce37484651bf37879acd5`;
  `screen/panel.json` `8d92098fea5eb693fe2f85c162d04368fccd7b7195134cafb220df8fa08a47a4`;
  `log` `877286e77f258f7b81718dcd96dda465bb027b4c877d6fe4141a7180f7fba4c7`.

### Deciding lanes and rule

Frozen plan: migration `current_stage` 2/3, cumulative
`stage_max_candidate_over_base` 1.025, one screen attempt, `panel_required` true.
The governing v4 lane rule (`tools/benchmark/layout_rule.py::lane_event`: "lane
rejects only if its six-layout geomean or every layout exceeds it"; T022: "Under
v4 a natural lane excess is a lane event needing K=6; it rejects only if the
six-layout geomean or every layout exceeds the ceiling") decides as follows.

| Lane | Ceiling | Layouts 0..5 | Six-layout geomean | Above ceiling |
| --- | ---: | --- | ---: | --- |
| broad | 1.012949 | 1.017445, 1.010724, 1.012763, 0.993484, 1.014494, 0.996732 | 1.007566 | 2/6, event nonpersistent |
| sentinel | 1.015687 | 1.023974, 1.023736, 1.017195, 1.023511, 1.018930, 1.032280 | 1.023260 | 6/6 |
| external | 1.007380 | 1.178263, 1.176930, 1.175771, 1.182171, 1.180231, 1.178769 | 1.178687 | 6/6 |
| development | 1.009630 | 1.138085, 1.139866, 1.150726, 1.145285, 1.141198, 1.141656 | 1.142795 | 6/6 |
| portfolio | 1.007249 | 1.113554, 1.110936, 1.111743, 1.108561, 1.113556, 1.108192 | 1.111088 | 6/6 |

Four lanes fail both branches. The broad natural excess is the only lane event
the panel dissolves (2 of 6 above, geomean under the ceiling). In addition,
42 cases fail their case rule: 38 carry "persistent six-layout wall cost" (all
six wall medians above B_i; T022: "Six medians above B_i reject, even with flat
instructions"), 37 carry "consistent instruction excess exceeds median hard cap"
(min above the band and median above 1.03; T022: "A consistent excess with median
above 1.03 rejects at the hard cap"), and 37 carry "panel instruction hard cap
exceeded".

Verdict: the K=6 panel was mandatory and this job completed it, so the rule calls
for no follow-up measurement or panel command. `event_resolution` is null because
the panel confirmed the events instead of dissolving them; the screen is a final
`reject`. It consumes the plan's single attempt. Under the migration rule (T022:
"an aborted stage closes an implementation rather than a mechanism family")
stage 2's implementation shape is closed: `migration.current_stage` stays 2, the
plan JSON is unchanged, and M1.3 is not started. No formal three-engine run
exists: "Only a passing screen spends a formal run", so no `decide --mode stage`
bundle could be produced.

### Where the cost is (top 5 by wall ratio)

Natural wall median (candidate/base) and six-layout wall geomean; both rankings
select the same five programs. The instruction median shows whether the cost is
executed work or placement.

| Program | Wall | Six-layout wall | Instructions |
| --- | ---: | ---: | ---: |
| external/kraken-1.1/json-parse-financial | 5.2138 | 5.2313 | 5.6513 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 3.5125 | 3.5266 | 3.7667 |
| external/sunspider-1.0/access-binary-trees | 3.0675 | 3.0790 | 3.3218 |
| external/sunspider-1.0/date-format-xparb | 2.2308 | 2.2071 | 2.3245 |
| external/jetstream3-js-subset/cdjs | 1.6238 | 1.6285 | 1.5652 |

All five are external workloads, and instruction ratios track wall ratios, so
the excess is additional executed work in the layout path, not a placement
accident. The next tier is hash-map (1.4502 wall / 1.3556 instructions),
broad/property_dynamic_read (1.3309 / 1.1966) and
sentinel/string_key_map_churn (1.2455 / 1.2098). Static broad/property_read
passes at 1.0162, while both dynamic keys and key churn fail, so the next
attempt should start from a fresh profile of per-access layout work on bulk
object/key churn, not from another variation of this shape. The three predeclared
development targets all regress: moment/dates 1.2444 wall / 1.3715 instructions,
diff/compare 1.3031 / 1.2856, meriyah/parse 1.2208 / 1.2442.
