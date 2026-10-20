# T063: Instruction regression diagnosis and repair

- Status: historical-campaign; instruction repair complete, not a wall-time acceptance.
- Implementation: present at `d5787373`; original reviewed candidate `41f4ee96`,
  base `cb8ec3ad`; no merge or project-remote push.
- Verified at: `d5787373`, 2026-10-20; instruction replay, full/fast checks,
  252 NG fixtures, 671 focused and 5170 complete allowlist cases.
- Evidence: durable counts, image identities and job ledger below; raw jobs
  are workspace `outputs/imac-bench/<job>/` artifacts, not retained in Git.
- Unresolved: lead wall-time acceptance remains open; exact LLVM heuristic
  behind the original code-generation perturbation is not claimed.
- Next action: [T063](T063-string-canonical.md) owns integration under
  its unchanged lead decision.

## Root cause and final repair

`property_dynamic_read` has three existing string keys and no concatenation
in its loop. Exact-image samples execute the same wide activation/property
read chain in both images. The untouched wide driver grew from 24736 to 26704
bytes after the canonical builders landed; its generated register stores
kept old `Value` payloads on the stack and needed more spill/reload work.
Ordinary VM activation assembly normalized identically. Release images do not
contain the `non-canonical JsString` debug assertion or a canonical validation
scan. No different JS admission or key construction explains this loop.

The repair in `compact_fn/execute.rs` tests the borrowed old tag before moving
anything. Numbers, booleans, null and undefined need no old payload kept alive.
Owned replacement is non-inlined, publishes the incoming value before dropping
the previous value, and exhaustively dispatches each field's ordinary Rust
drop. Explicit field dispatch avoids a second enum drop call and stack copy.
The final wide body is 24640 bytes; its non-owning stores have no old-payload
stack copy. Forwarded-local/temporary read tests and an ownership/refcount/
negative-zero replacement test pin the relevant semantics.

A second concrete cost remained in String-heavy setup. The 192-byte
`string_from_code_units` loop called a 416-byte `push_code_unit` on every unit.
The final loop emits ordinary BMP units inline and calls only the cold
surrogate helper for D800..DFFF. Its body is 512 bytes; ASCII no longer pays a
per-unit general-helper call. The shared seam helper still merges a low
surrogate with a preceding high sentinel, preserving actual U+F0000..F07FF.
`push_js_str` keeps its ordinary append small and puts UTF-8 seam reconciliation
out of line. The lexer's already-validated non-sentinel scalar branch also
uses direct emission. No consumer normalization, unsafe code, new dependency,
frame/binding model or benchmark recognition was added; the constructor's debug
assertion and all original canonical tests remain.

Tagcloud phase probe `20261020T100511Z-t063-tagcloud-phase-instructions`
(exit 0) used one source with a flag selecting `makeTagCloud(tagInfo)` versus
an equally long fixed result. Five alternating instruction pairs: setup
565023975 -> 572107744 (1.012537), full 773102371 -> 779678013 (1.008506),
body difference ratio 0.997558. This localized the residual to initialization,
not the append loop. The probe was diagnostic only, never an acceptance case.

## Exact x86 instruction replay

All counts below are on the iMac, x86_64 macOS, rustc 1.95.0/LLVM 22.1.2.
Images use the existing canonical build recipe: opt-level 3, fat LTO,
16 codegen units, abort, agents, symbol mangling v0 and function alignment 4.
`tools.benchmark.triage_build.build` builds clean isolated committed sources.
Each engine ran through `/usr/bin/time -l`; only `instructions retired` was
used. No wall measurement, compare/loop, or `--timing` job was submitted.
Every before/after pair has identical stdout SHA-256. These are three-pair
instruction diagnostics, not formal intervals or a promotion verdict.

Both columns are candidate/base median paired ratios, each in an independent
cohort. Before: exact `41f4ee96` image replay in
`20261020T092510Z-t063-instruction-diagnosis-r2`; after: `d5787373` in
`20261020T101109Z-t063-unit-inline-instructions`, both exit 0.

| Case | Before | After |
| --- | ---: | ---: |
| broad/property_dynamic_read | 1.027816 | 0.989833 |
| external/raytrace-public-class-fields | 1.025561 | 0.989309 |
| external/cdjs | 1.022672 | 0.991400 |
| external/access-binary-trees | 1.021637 | 0.993415 |
| external/string-base64 | 1.024657 | 0.994817 |
| development/lodash/transform | 1.018467 | 0.988185 |
| external/string-tagcloud | 1.013103 | 0.995202 |
| external/string-unpack-code | 1.013336 | 1.001043 |
| development/js-yaml/document | 1.011699 | 0.996305 |

All nine medians satisfy the requested 1.005 instruction goal; the largest
individual paired ratio is 1.001211. Broad counts use N=2M/4M iterations,
three properties/iteration. Original N-to-2N increments: base 618.7032,
reviewed candidate 636.0159 instructions/property (+17.3127). Final cohort:
base 618.6695, repair 612.3181. Cohorts are not pooled.

All six sentinels were checked at the original triage's recorded iteration
counts: medians range 0.999957..1.000797. A dynamic ASCII append probe uses
`function build(n, part) { var text = ''; for (var i=0; i<n; i++) text += part;
return text.length; } build(+scriptArgs[1], 'a');` with N=1M/2M. Its N-to-2N
increment is 659.9946 -> 656.0170 instructions per declared append (ratio
0.993973): no positive net append instruction cost is observed. This does not
claim that the seam branch itself costs zero independently of other codegen.

Image SHA-256:

- Base: `c2ba98499c5416f6f9ee950e724b6e0ff3a6fbf893f478b25abb6d370bf382be`.
- Reviewed candidate: `55f9fe6a567ada5925ced80ae5bbdc447c1f7ac49248304f38aefa4929d75edb`.
- Repair: `6dac1d262b83bb69bf8c73f009269e37f287a155ccd575e68851d663ffa673ad`.
- Final `instructions.json`: `e1fbf7a9962932c9fea9d27270677e67b77617efe99dbc2770678828dd2bd613`.
- Final `extra-instructions.json`: `0c6d80fc17fbf0aa8a3cdb73036f4548231705b5a38bec22a2fc0773c5861409`.

External/development sources are the original triage's generated bundles,
reproducible with `python3 -m tools.benchmark.bundles` and the corresponding
`benchmarks/external-preview.json` / `benchmarks/development-preview.json`.
Broad uses `benchmarks/workloads/broad-micro.js property_dynamic_read N`.
All counter cohorts run each role once per pair, alternating order across
three pairs. Jobs retain the precise commands, all raw counts and build specs.

## Diagnostic sequence retained

Each job below is prefixed `20261020T`; all were non-timing. Rejected choices
were reverted; no acceptance threshold or attempt budget was changed.

| Revision / job | Observation and disposition |
| --- | --- |
| `092410Z-t063-instruction-diagnosis` | Cancelled; no evidence used. |
| `41f4ee96`, `092510Z-t063-instruction-diagnosis-r2` | Original exact-image samples and instruction reproduction; exit 0. |
| `cc68ea0d`, `092901Z-t063-cold-seam-instructions` | Cold seam alone: property 1.01498, wide 25856 bytes; insufficient, boundary retained. Exit 0. |
| `5a8147c4`, `093457Z-t063-shared-append-instructions` | Whole append outlining: property 1.01493; reverted. Nine case counts complete, exit 1 in extra probe because it called unavailable console; no extra-probe result retained. |
| `073cc2d1`, `094003Z-t063-operand-boundary-instructions` | Operand-call boundary: property 1.02030, dynamic append 1.06683; call cost outweighed smaller driver. Reverted, semantic test retained. Exit 0. |
| `623cf4d2`, `094516Z-t063-store-lifetime-instructions` | Borrowed-tag store: property 0.98979; tree 1.00783/cloud 1.01453 remained. Exit 0. |
| `afa9c0c7`, `095500Z-t063-scalar-producer-instructions` | Avoid redundant lexer scalar helper: cloud 1.01275; retained, insufficient alone. Exit 0. |
| `095916Z-t063-residual-short-samples` | Exact short-process samples, exit 0; too sparse for percentages, helped identify the owned-store wrapper. |
| `be8edc27`, `100202Z-t063-direct-drop-instructions` | Direct owned-field drops: tree 1.004916/cloud 1.00857; retained, cloud still open. Exit 0. |
| `100511Z-t063-tagcloud-phase-instructions` | Setup/body phase localization described above; exit 0. |
| `44c2d904`, `100819Z-t063-code-unit-instructions` | Low-unit guard alone: cloud 1.00849, per-unit call remained; superseded by inline/cold split. Exit 0. |
| `d5787373`, `101109Z-t063-unit-inline-instructions` | All nine goals and six sentinels meet instruction target; exit 0. |

## Correctness rerun

At `d5787373`, local pre-commit fast checks pass; all final runtime tests pass
(2325). Mapped bytecode changes selected 65 Test262 cases, lexer changes 98;
string.rs alone matched no automatic filter, so the full/focused gates below
supply its semantic coverage. Nine canonical tests still include the debug
assertion panic and exhaustive real sentinel-range scalars.

No-timing job `20261020T101409Z-t063-instruction-repair-gates` (334 s):

| Command | Exit / result |
| --- | --- |
| `./scripts/check.sh` | 0; all feature clippy/tests, workspace runtime 2325, 659 tool tests, 35 script tests, 5170 Test262 allowlist cases |
| `./scripts/compare-qjs.sh` | 0; 252/252 fixtures match |
| `./scripts/test262-subset.sh` with String/Object/JSON/Proxy filters | 0; 671/671 |
| `test262-baseline.sh --all --filter test/built-ins/String --engine quickjs-rust` | 1; 1221 pass, 2 fail, 0 timeout |
| Same upstream command, Object | 0; 3411 pass |
| Same upstream command, JSON | 0; 165 pass |
| Same upstream command, Proxy | 1; 310 pass, 1 fail, 0 timeout |

Upstream scans use `QJS_CLI_BIN` pointing to the exact final release image.
The Rust-only mode includes three cases excluded from the pinned NG
configuration (`regexp-duplicate-named-groups=skip`, `tail-call-optimization=skip`).
All 5107 previously configured cases pass; all three extra failures were
replayed on both the exact base and repair images, with the same errors:

- `String/prototype/match/duplicate-named-groups-properties.js` and
  `duplicate-named-indices-groups-properties.js`: parse SyntaxError,
  `invalid regular expression group name`.
- `Proxy/revocable/tco-fn-realm.js`: runtime Error, `value is not callable`.

The paired failure replays were functional-only local executions of the exact
x86 images, not local hardware counter data. Job exit 1 and these failures
are retained, not hidden by expectation/config edits. Initial reference-only
NG failures are documented in [the initial correctness record](T063-correctness-verification.md).
The lead's frozen section stays byte-for-byte unchanged (SHA-256
`372e267e65d847e52905742b92481686aed03725d0c805584f235174ad66174f`).
