# T056 M1 stage A: implementation record

Stage A of plan [`interpreter-core-v2-layout-edges`](../performance-units/interpreter-core-v2-layout-edges.json)
(plan commit `50b42547`), implemented on `agent/t056-m1-stage-a/claude` per
[property layouts](../../docs/design/property-layouts.md). Owner of status and next action:
[T056](../T056-interpreter-core-v2.md). Non-timing instruction counts diagnose cost; the frozen pre-gate only admits or blocks a timed screen.
No timing acceptance is claimed.

## Branch content

Runtime commits over `50b42547`, oldest first: `3506f4db` (layout authority for ordinary stores), `b1686896`
(class-field keys interned at definition, lazy native default prototypes), `1979fcb7` (function and array named
tables), `83601f83` and its revert `a31e87fa` (see the disclosure below), `9d5f96fb` (slot writes through the
payload descriptor), `0abd7976` (one probe per named-table add, layout-owned function key order, shared fallback
domain), `7d8f18db` (slot accesses without reading the layout node), `9049dfc6` (a missed named write is the
creation's only probe), then tests and documentation. `ff850af8` removes that fallback domain and threads the realm domain through
all creation paths; its WIP formatting and temporary probe test are cleaned up in the continuation.

At `72c2e5db`, deviations from the design, with their reasons, were stated in the design itself: the shared-slot dictionary
token (a `Weak` of the node plus the owner's layout revision), edit edges capped at 8 with content-cache hits
recording no edge, the lazy large-shape index, explicit realm domains on all creation paths, and unpacked flags (packing
saves no bytes at 120). The domain-less lexical marker never accepts named properties. Built-in constructor key
order follows installation order, locked by `builtin_constructor_keys_enumerate_in_installation_order`. Counter
tests no longer warm a fallback domain because that one-time production initialization no longer exists.

## Disclosure: write-site add replay (removed from stage A)

`83601f83` remembered, per named write site with a creation proof and per public class field, the layout
transition of the last plain creation, and replayed it by layout identity on the next receiver. The review ruled it
outside stage A (a route, stage B's accelerated Set, or an L4 leaf for the class-field part, which conflicts with
"class fields and constructor stores cache no final shape"), so `a31e87fa` reverts it; it is kept on
`agent/t056-add-transition-replay/claude`. It was implemented and measured during stage A. Local arm64 instruction
ratios against the base image of `50b42547` (min of 3, whole process):

| Workload | `1979fcb7` (before) | `83601f83` (with replay) |
| --- | ---: | ---: |
| binary-trees, 400 passes | 1.0536 | 0.8558 |
| constructor `new Node(l, r, v)`, 3M | 1.0344 | 0.9437 |
| class fields `new P()`, 4 fields, 2M | 1.0194 | 0.9762 |
| raytrace-class-fields bundle | 1.0203 | 0.9572 |
| 3d-raytrace bundle | 0.9825 | 0.9819 |
| `new Object()`-style empty construction | 1.0022 | 1.0022 |

These ratios include the replay; the stage A numbers below do not.

## Local instruction ratios at `9049dfc6` (arm64, min of 3, against the base image)

| Workload | Ratio |
| --- | ---: |
| raytrace-class-fields bundle | 1.0234 |
| binary-trees, 400 passes | 1.0177 |
| class fields, 4 fields | 1.0155 |
| constructor, 3 fields | 0.9911 |
| moment/dates, diff/compare, meriyah/parse (single passes) | 0.9880, 0.9764, 0.9764 |
| 3d-raytrace, JSON.parse loop, empty construction | 0.9888, 0.9915, 0.9951 |

Broad screen at `7d8f18db` (local, increment-measured): `top_level_function_call` 0.9919 (1.0251 before the
store kept its encoding and the dictionary token), `array_index_of` 0.9953, `captured_write` 0.9858,
`global_read` 1.0001. `array_index_of` had no hot-path source change; its per-iteration perf-counter events
equal the base's (none counted), and arm64 `wide::activation::run` is 4596 instructions on the base and 4520
here with the same callee set except two slot readers that changed inlining, so its earlier 1.028 was code
placement. The x86 symbol comparison rides on the pre-gate image.

Startup (`0;`, whole process): 21.05M base, 21.29M here (+1.1%); atom domains at startup fell from 10 to 2 (the
realm's and the shared fallback). The remaining cost is the append chains of large install objects: 658 shapes and
11,098 field copies per realm.

## Gates

At `ccbc2378` plus the documentation commit: `./scripts/check.sh` passed (format, clippy for default, agents and
perf-counters, all three test configurations, benchmark tool tests, plans, context, file sizes, Test262 subset of
5170 cases) and `./scripts/compare-qjs.sh` passed all 255 fixtures, including the three new layout fixtures.

Continuation of `ff850af8`: removed the temporary prototype-table probe, formatted the realm-domain WIP,
and split the native Test262 helpers from `global.rs` into `global/test262.rs` after the domain additions took
the parent past its 2000-line limit. The cleanup working tree passed `check.sh` (including 5170 Test262 cases),
`compare-qjs.sh` (255 fixtures), and `check-touched.sh --staged --explain` (2340 runtime tests and the selected
function/eval slices). Final runtime changes will receive fresh gates.

## Pre-gate 1: diagnostic image, BLOCK

Job `20261020T171937Z-t056-m1a-pregate1`, image `38af1bc3` versus base `f58c696e`, x86_64 canonical
release builds, one instruction pair over all 86 policy cases, cutoff `max(1.03, I_i)`. This image still used
the removed thread-local fallback and is diagnostic only, never the screen image. No timed attempt was spent.
Raw job files: workspace `outputs/imac-bench/20261020T171937Z-t056-m1a-pregate1/out/`.
Binary SHA-256: base `b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e`,
candidate `b558c7be06fd5bd0b08a55f39c8223bf2e60bd93573c7330a83467a6ebe75a3f`.
The script's `cases 172` metadata counts argument words (`--case` plus each id), not cases; JSON/verdict covers 86.

| Case | Candidate/base instructions |
| --- | ---: |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.0387 **BLOCK** |
| broad/captured_write | 1.0310 **BLOCK** |
| development/graphlib/paths | 1.0291 |
| external/sunspider-1.0/string-unpack-code | 1.0275 |
| external/jetstream3-js-subset/hash-map | 1.0196 |
| external/sunspider-1.0/string-base64 | 1.0176 |
| external/sunspider-1.0/string-validate-input | 1.0170 |
| development/js-yaml/document | 1.0160 |
| development/moment/dates (target) | 0.9811 |
| development/diff/compare (target) | 0.9746 |
| development/meriyah/parse (target) | 0.9819 |

Final committed-image results are recorded at the end of this file.


## Explicit-domain continuation diagnostics

`20261020T180640Z-t056-m1a-codegen-diagnose` built canonical `ff850af8` and the base and captured x86
symbols/assembly plus agents/perf-counters binaries. Its collection failed because the temporary bundle directory
was absent; no timing or pre-gate verdict resulted. `20261020T181103Z-t056-m1a-codegen-counts` reused those exact
binaries and completed the counters and an eight-case diagnostic. This is not a complete pre-gate.

- `captured_write`: 1.0311; subtracting the 1000-iteration events from 2000 iterations gives identical base and
  candidate events. The added layout startup events do not scale with iterations. Wide `run` is 25,264 bytes
  on the base and 23,984 on `ff850af8`; a smaller body alone does not rule out register spills.
- The existing execution events of string-unpack-code, string-base64 and string-validate-input match exactly;
  instruction ratios are 1.0329, 1.0173 and 1.0174. New layout hashes are 108,954 / 16,453 / 36,039, respectively;
  these counters did not exist in the base and cannot by themselves establish extra work versus its old maps.
  No tier-coverage shift is observed. Added atoms/shape work and code generation both require accounting.
- Raytrace fields: 1.0460; graphlib: 1.0326; hash-map: 1.0196; js-yaml: 1.0161.

`bd5c340b` makes the constructor's realm-domain lookup lazy and splits property insertion into a small edge-hit
path and an out-of-line replacement/new-edge path. The hit keeps the payload encoding already known: append edges
cannot re-encode. Test coverage checks inherited-domain creation and null-prototype fallback; every field still
follows one edge, with no site replay or final-shape cache.

Job `20261020T181515Z-t056-m1a-edge-instructions` (13 diagnostic cases, not a complete pre-gate) at `bd5c340b`:
raytrace 1.0359, captured-write 1.0310, unpack-code 1.0307, graphlib 1.0341, hash-map 1.0198, base64 1.0169,
validate-input 1.0164, js-yaml 1.0158; controls dynamic-read 0.9995 and recursive-call-tree 1.0000; targets
moment 0.9811, diff 0.9751, meriyah 0.9766. Wide `run` remains 23,984 bytes. None of these jobs spends a
screen attempt or establishes timing performance.


At `55ac897e`, insertion's edge-hit wrapper is forced inline and guarded prototype-slot payload reads are
out of line. Job `20261020T181842Z-t056-m1a-inline-instructions` still gave captured-write 1.0310 and
raytrace 1.0361, so that boundary did not fix either blocked case. Unpack-code 1.0360, graphlib 1.0313,
hash-map 1.0245, base64 1.0168, validate-input 1.0167, js-yaml 1.0131; targets moment/diff/meriyah
0.9805 / 0.9752 / 0.9764. Wide `run` shrank to 23,328 bytes with no captured-write instruction change.

`20261020T182153Z-t056-m1a-callee-assembly` compared the exact base and `55ac897e` admission helpers.
The hot `inline_entry` grew from 368 to 400 bytes, saving two more registers/stack words and retaining the
program-cache pointer through its cold initializer. `1244c8dd` separates cached wide-program reads from
an out-of-line `initialize_program`, leaving compilation, cached declines and admission unchanged. It also
forces the existing per-field append-edge probe inline: x86 still called that 320-byte helper from the constant
field loop after the insertion wrapper was inlined. These are general cache-hit/code-generation boundaries;
no operation, receiver, field sequence, function or benchmark is newly admitted.


At `1244c8dd`, job `20261020T182416Z-t056-m1a-program-instructions` reduced captured-write to 1.0124,
raytrace to 1.0262, graphlib to 1.0279, base64 to 1.0012, validate-input to 1.0126, hash-map to 1.0132 and
js-yaml to 1.0078. Unpack-code remained above the cutoff at 1.0349. Dynamic-read was 0.9802, recursive tree
1.0000; targets moment/diff/meriyah 0.9768 / 0.9721 / 0.9725. Wide `run`: 22,928 bytes. The cached-program
boundary fixes real executed admission/code-generation overhead, rather than relying on body size alone.


`36570fbf` carries the existing admitted-call facts word through `InlineCallee` as `NonZeroU64`, instead of
expanding it into a register count and four booleans and returning a separate class flag. It uses exactly the
same cached facts, caller-dependent checks, exit-admission counter and class/call distinction. The change removes
aggregate construction/return work for every eligible wide call and constructor, without adding a cache or a
recognizer. The 78 focused wide-tier tests and the fast gate (2341 runtime tests plus 65 selected Test262 cases)
passed. Arm64 diagnostic instructions: captured-write 0.9929, raytrace 1.0263, unpack-code 1.0235 against the
existing base binary; only instructions are used under concurrent build load.


Job `20261020T182911Z-t056-m1a-facts-instructions` at `36570fbf`: captured-write 0.9903, raytrace 1.0231,
base64 0.9992, validate-input 1.0080, hash-map 0.9950, js-yaml 1.0044; dynamic-read 0.9973, recursive tree
1.0001. Unpack-code 1.0322 and graphlib 1.0308 remained just above the pre-gate cutoff. Targets moment/diff/
meriyah 0.9730 / 0.9733 / 0.9721. This remains a 13-case diagnostic, not the complete pre-gate.

The continuation then applies the ordinary store's existing edge-as-absence proof to `NamedTable::insert_in`
(function/array named descriptors): a matching append edge inserts directly, instead of searching the parent's
fields before following that same edge. Replacement/new-edge logic stays out of line; the table still takes its
realm domain only at its first named install. The four perf-counter layout tests pass, including named-table
probe/retention obligations. It adds no per-site replay, cached final shape or specialized property name.


`5d3e04bd`, job `20261020T183455Z-t056-m1a-table-instructions`: unpack-code 1.0337 and graphlib 1.0325
still blocked. Captured-write 0.9903, raytrace 1.0235, base64 0.9992, validate-input 1.0079, hash-map 0.9959,
js-yaml 1.0060; targets moment/diff/meriyah 0.9759 / 0.9745 / 0.9734; dynamic-read 0.9972, recursive tree
1.0000. Wide `run` was 23,504 bytes. The edge-first named table did not alone improve those instruction costs.
Its complete insertion boundary is kept out of line in the following revision to contain caller code growth.

The next probe change compares short layout keys with bounded, overlapping head/tail byte slices (2, 4 or 8 bytes)
after the existing length/pointer/first-byte tests. These safe fixed-width comparisons avoid general byte-comparison
calls for keys shorter than 16 bytes; longer keys retain the old comparison. All bytes participate, with no hash,
new key identity or semantic shortcut. The focused boundary test changes each byte at lengths 0–40 and covers
multibyte UTF-8. All 11 layout tests passed; the complete gates follow below.


## Final runtime candidate

`72c2e5db` is the final runtime image of this continuation. Job
`20261020T183933Z-t056-m1a-keys-instructions` passed all 13 diagnostic cases below 1.03: raytrace 1.0230,
unpack-code 1.0210, graphlib 1.0205, captured-write 0.9903, base64 0.9955, validate-input 1.0030,
hash-map 0.9999, js-yaml 1.0021, dynamic-read 0.9876, recursive tree 0.9999; targets moment/diff/meriyah
0.9784 / 0.9782 / 0.9716. This is still diagnostic, not the mandatory 86-case pre-gate.

Wide `activation::run`: base **25,264 bytes**, candidate **23,504 bytes**. Named direct Rust callees (compiler
suffixes removed) change from 61 to 64. `inline_entry` and the OnceCell `try_init` closure leave the direct-call
set; `initialize_program` and `compute_fixed_inline_facts` enter it. The original literal-prototype reader is
replaced by `prototype_data_slot_value`; `shared_data_slot_value`, `prototype_is_weak`, `Upvalue::get` and
`DynamicBindings::cell` become out-of-line calls, while the old dynamic-binding HashMap `get` is no longer a
direct callee. `shared_data_slot_write` remains a callee under a changed impl symbol. This records named-call
boundaries, not an attribution of executed cost from size or static call counts alone.

Final arm64 diagnostics against the handed-off base binary: captured-write 0.9907, unpack-code 1.0288,
raytrace 1.0374. The class workload still has an arm64 instruction regression; x86 is this unit's frozen gate,
and these local counts establish no timing acceptance.

Candidate isolation: `git merge-base 50b42547 72c2e5db` is `50b42547`; `git rev-list --merges
50b42547..72c2e5db` is empty. `git diff f58c696e 72c2e5db -- crates` changes only **122 files in
`crates/qjs-runtime`**, covering this M1 layout/domain migration and its code-generation repairs. No main merge,
diagnostic-branch cherry-pick, parser/AST change or saved-out add replay was introduced. The exact diff is
reproducible from these repository revisions.

At `72c2e5db`, the pre-commit fast gate passed (2342 runtime tests; no focused Test262 filter matches these
property-module paths), all 255 `compare-qjs.sh` fixtures passed, and the pre-push `check.sh` passed including
default/agents/perf-counters checks and 5170 Test262 subset cases. Concurrent background pushes caused the first
push to lose the remote-ref compare-and-swap race; a serialized normal push succeeded after the full gate.
Runtime source is frozen here; final configured Test262/counter verification and the complete pre-gate follow.


Final event job `20261020T184243Z-t056-m1a-final-correctness` built agents/perf-counters at `72c2e5db` and
compared captured-write (1000 and 2000 iterations), raytrace fields, graphlib, unpack-code, base64 and
validate-input with the exact base's counter build. Every original non-`property_*` event matches, including
compact/direct/native calls, opcode counts and tier entries. Each final script records exactly one production
atom domain; the removed fallback no longer adds another startup domain. The new layout counters have no matching base instrumentation;
atom-creation counts grow because layouts now intern their keys. This rules out a counted tier/route shift; it does not
separately attribute byte-comparison and register costs that these counters do not count.

The job's unsharded exact scan was cancelled during serial metadata enumeration (no complete scan and no
burndown entry). Replacement `20261020T184826Z-t056-m1a-final-test262-shards` runs all eight disjoint `--all
--engine both --shard I/8` slices with one worker each, agents enabled, the canonical candidate, 20-second timeouts
and one timeout retry, then the standard burndown aggregation. No case exclusion or acceptance rule changes.
The final pre-gate job `20261020T184945Z-t056-m1a-pregate2-final` runs after that scan and checks its
complete passing entry for the same commit before executing the unchanged pre-gate-1 script.

Remote CI for `72c2e5db`: run `39477362030` **success** (2026-10-20), in addition to the local gates above.


The complete eight-shard scan passed at `72c2e5db`: **42,672 / 42,672 configured Rust cases**, no failures,
timeouts or not-run cases; 53,572 total, 10,900 skipped by the pinned QuickJS-NG configuration. QuickJS-NG:
42,602 pass, 70 fail, zero timeouts; actionable gaps zero. The union of the eight per-case JSONL files equals
the pinned corpus's 53,572 paths exactly once, with no duplicates or omissions. The standard complete-scan entry
was appended to `docs/conformance/burndown.jsonl` as `local-imac`; no partial scan was recorded.


## Final pre-gate 2: PASS

Job **`20261020T184945Z-t056-m1a-pregate2-final`**, candidate **`72c2e5db2f041dcbc4e798cbfa63c36c7f3ec6f9`**
versus base **`f58c696e8379261ed726f0cea1051ed6d34f431a`**, x86_64 canonical uninstrumented agents/release
images. It ran the unchanged pre-gate-1 script after verifying the same commit's complete passing conformance
entry. **86/86 present, all pass** against `max(1.03, I_i)`; every current policy band is below 1.03, so every
cutoff is 1.03. Worst ratio 1.02425836 (unpack-code), instruction geomean 0.99080427. The <=1.02 engineering aim
was not reached on unpack-code, prototype-method-call and class-fields, but all have margin to the frozen cutoff.
No timed screen or attempt was submitted; both frozen screen attempts remain available.

Binary SHA-256:
- Base: `b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e`.
- Candidate: **`bb422adc4f733ae29565f37dbe36c4ff22a95fd2b86167bcbf8fee2591200e33`**.

Raw receipt: workspace `outputs/imac-bench/20261020T184945Z-t056-m1a-pregate2-final/out/` (`images.txt`,
`pregate.json`, `pregate.txt`, `verdict.txt`). As in pre-gate 1, `cases 172` counts argument words; the JSON and
verdict cover exactly 86 unique case ids. The complete instruction ratios below preserve the result in the
repository rather than depending on workspace artifacts. Ratios are candidate/base, lower is fewer instructions.

| Case | Instruction ratio |
| --- | ---: |
| external/sunspider-1.0/string-unpack-code | 1.02425836 |
| sentinel/prototype_method_call | 1.02273262 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.02124580 |
| development/graphlib/paths | 1.01879811 |
| external/sunspider-1.0/controlflow-recursive | 1.01114342 |
| external/sunspider-1.0/bitops-bitwise-and | 1.00905352 |
| external/sunspider-1.0/access-binary-trees | 1.00792380 |
| external/jetstream3-js-subset/gaussian-blur | 1.00643163 |
| external/sunspider-1.0/bitops-bits-in-byte | 1.00542755 |
| external/sunspider-1.0/bitops-3bit-bits-in-byte | 1.00522402 |
| development/js-yaml/document | 1.00488434 |
| external/sunspider-1.0/crypto-sha1 | 1.00426088 |
| external/jetstream3-js-subset/cdjs | 1.00330205 |
| external/kraken-1.1/audio-dft | 1.00291469 |
| external/sunspider-1.0/string-validate-input | 1.00288399 |
| external/sunspider-1.0/bitops-nsieve-bits | 1.00251468 |
| external/kraken-1.1/imaging-desaturate | 1.00197793 |
| external/kraken-1.1/audio-beat-detection | 1.00175171 |
| external/sunspider-1.0/access-nbody | 1.00161606 |
| broad/array_allocation | 1.00149944 |
| broad/object_allocation | 1.00091602 |
| external/sunspider-1.0/3d-raytrace | 1.00090950 |
| external/sunspider-1.0/regexp-dna | 1.00088530 |
| external/kraken-1.1/audio-oscillator | 1.00048685 |
| broad/array_dynamic_read | 1.00005021 |
| broad/string_slice | 0.99998983 |
| external/jetstream3-js-subset/hash-map | 0.99998724 |
| sentinel/recursive_call_tree | 0.99985767 |
| external/sunspider-1.0/crypto-md5 | 0.99972905 |
| broad/array_read | 0.99766308 |
| broad/property_read | 0.99752206 |
| external/sunspider-1.0/math-spectral-norm | 0.99751942 |
| broad/many_locals_call | 0.99749158 |
| broad/closure_allocation_call | 0.99734040 |
| external/kraken-1.1/audio-fft | 0.99606761 |
| external/kraken-1.1/stanford-crypto-ccm | 0.99549549 |
| external/sunspider-1.0/string-base64 | 0.99530641 |
| broad/captured_read | 0.99481792 |
| broad/function_call_two_args | 0.99468649 |
| broad/plain_function_call | 0.99466345 |
| broad/function_call_reordered | 0.99457557 |
| broad/property_write | 0.99450810 |
| external/sunspider-1.0/math-partial-sums | 0.99445462 |
| broad/array_write | 0.99411920 |
| external/sunspider-1.0/access-nsieve | 0.99330115 |
| broad/dynamic_method_call | 0.99307369 |
| external/sunspider-1.0/date-format-tofte | 0.99242772 |
| development/immutable/persistent | 0.99152115 |
| development/lodash/transform | 0.99092803 |
| sentinel/string_key_map_churn | 0.99091787 |
| development/papaparse/csv | 0.99043750 |
| broad/captured_write | 0.99035041 |
| external/kraken-1.1/ai-astar | 0.98901898 |
| broad/math_abs | 0.98826949 |
| broad/property_dynamic_read | 0.98745809 |
| external/sunspider-1.0/3d-morph | 0.98660699 |
| development/big.js/arith | 0.98599823 |
| broad/top_level_function_call | 0.98591604 |
| external/sunspider-1.0/access-fannkuch | 0.98508870 |
| external/sunspider-1.0/string-fasta | 0.98452742 |
| external/kraken-1.1/imaging-darkroom | 0.98415429 |
| external/sunspider-1.0/crypto-aes | 0.98389663 |
| sentinel/polymorphic_call_site | 0.98325264 |
| external/sunspider-1.0/date-format-xparb | 0.98292501 |
| sentinel/capturing_closure_call | 0.98192615 |
| external/jetstream3-js-subset/stanford-crypto-aes | 0.98166170 |
| external/kraken-1.1/json-stringify-tinderbox | 0.98151136 |
| development/matter-js/simulate | 0.98137924 |
| broad/empty_loop | 0.98128497 |
| broad/method_call | 0.98025541 |
| external/sunspider-1.0/math-cordic | 0.97950273 |
| external/kraken-1.1/stanford-crypto-aes | 0.97897667 |
| external/kraken-1.1/imaging-gaussian-blur | 0.97883913 |
| development/diff/compare | 0.97814189 |
| development/moment/dates | 0.97794027 |
| external/kraken-1.1/stanford-crypto-sha256-iterative | 0.97664189 |
| broad/branch_arithmetic | 0.97382774 |
| external/kraken-1.1/stanford-crypto-pbkdf2 | 0.97382548 |
| broad/global_read | 0.97374395 |
| development/meriyah/parse | 0.97220056 |
| broad/local_read | 0.96948555 |
| external/sunspider-1.0/3d-cube | 0.96879410 |
| broad/array_index_of | 0.96815281 |
| external/sunspider-1.0/string-tagcloud | 0.95842634 |
| external/kraken-1.1/json-parse-financial | 0.95102842 |
| sentinel/heterogeneous_property_read | 0.89533304 |

This pre-gate admitted that earlier image only; the review corrections below supersede it for timing.
Its documentary successor changed no runtime source and established no timing acceptance or integration.


## Independent review corrections after `6f5eb651`

All four findings were reproduced before the fixes: four focused tests failed on default-key reordering,
prototype placement, dictionary copy-on-edit and two root sets per independent realm. The review correction
`0a9af81b` makes these changes:

- Materializing implicit `length`/`name` moves an already installed descriptor into its leading slot and
  counts it in `leading_defaults`, preserving its value and attributes. Error and eight native error subtypes,
  other built-in constructors and a direct-table default replacement test cover freeze, seal and defineProperty.
- Shared dictionary slots hold a `Weak<()>` of the independent dictionary identity plus the owner's layout
  revision, not a weak layout-node reference. Structural edits retain a uniquely owned layout (strong=1, weak=0).
  A work-count test holds a cached slot across append/delete/reconfigure/seal/freeze and checks zero dictionary
  field copies and stable node identity; true copy-on-write is counted separately.
- The earlier native constructor install-order change is **reverted**: the lazy default reserves `prototype`
  immediately after `length`/`name`, including when installers replace it directly after statics. No unused
  prototype object is created. The order-locking test now asserts prototype before statics; it no longer claims
  the changed order is implementation-defined.
- Script and module creation hold one DomainRoots across global-object construction and realm creation. A
  counter test checks one domain and one root set for each independent entry path, rather than two root sets.

These fixes change runtime source, so `72c2e5db`'s exact receipt/pre-gate cannot admit the corrected candidate.
Fresh verification and the new final candidate are recorded below when complete. No timing job is authorized
by this correction; the lead retains timing submission.

The correction working tree passed all five focused regression tests with `perf-counters` (the four initial
regressions were first observed failing), `./scripts/check.sh` (2345 runtime tests, agents, 29 diagnostic tests,
662 benchmark-tool tests, 35 script tests, context/file guards and 5170 Test262 subset cases), and all
255 `./scripts/compare-qjs.sh` fixtures. The pre-commit fast gate also verifies the staged correction; the
committed-image exact and instruction pre-gate follow. New diagnostic counters count dictionary field copies
and domain root-set creation; neither exists as executed counter work in the canonical uninstrumented build.

Runtime correction commit: **`0a9af81bebcef80e9239954f8db191a71f10316e`**. Its pre-commit fast gate passed
2345 runtime tests and the 117 selected Test262 function/eval slices. The full gate above ran manually before
push, so the duplicate pre-push invocation was skipped via the hook's `QJS_SKIP_PRE_PUSH=1` switch. Remote CI
run **39487411821** is **success** for this exact commit. Candidate isolation remains unchanged: the plan commit
`50b42547` is an ancestor, no merges occur in `50b42547..0a9af81b`, and
`git diff f58c696e 0a9af81b -- crates` touches only 122 M1 files in `crates/qjs-runtime`.

Fresh non-timing jobs are `20261020T195419Z-t056-m1a-review-test262-shards` (eight disjoint complete shards) and
`20261020T195429Z-t056-m1a-pregate3-review-final` (dependent 86-case instruction gate). The canonical
candidate SHA-256 is `1970bb690fd9445e94036df51eb29ff7580c10ef16f0f298d0b0cee2112b7491`; the pre-gate
checks its physical hash against the exact-scan image before collecting. The complete results follow.

The new complete exact job finished **done 0** in 1112 seconds including its canonical release build.
Rust: **42,672/42,672 configured pass**, zero fail/timeout/not-run. QuickJS-NG: 42,602 pass, 70 fail,
zero timeout; actionable gaps zero. All eight shard summaries are `--all --engine both`, with no filter;
the 53,572 raw case paths occur exactly once and equal the pinned corpus. The decision tool's
`_test262_zero_gap` accepted the new commit's schema-1 entry, which was appended through the standard
`test262-burndown.sh --entry` path to `docs/conformance/burndown.jsonl`. The acceptance chain must reuse
this new receipt, never the superseded `72c2e5db` entry.


## Corrected-image pre-gate 3: PASS

Job **`20261020T195429Z-t056-m1a-pregate3-review-final`**, candidate **`0a9af81b`** versus the original
base **`f58c696e`**, completed **done 0** in 187 seconds, after checking the new complete exact entry.
The candidate's physical SHA-256 matches the exact-scan image above; base SHA-256 remains
`b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e`. All **86/86** policy cases are present
and pass the unchanged `max(1.03, I_i)` instruction cutoff (all current cutoffs are 1.03). The worst case is
string-unpack-code at **1.02717746**; the engineering aim <=1.02 is not met by unpack-code, prototype-method-call,
raytrace fields and binary-trees. There is no wall-time acceptance claim. Both jobs are non-timing; no timed
screen or attempt was submitted, and the two frozen attempts across stages A/B remain available.

Instruction geomean **0.99197342**. Targets moment/dates **0.97775942**, diff/compare **0.97853817**,
meriyah/parse **0.97096351**. Raw receipt: workspace
`outputs/imac-bench/20261020T195429Z-t056-m1a-pregate3-review-final/out/` (`images.txt`, `pregate.json`,
`verdict.txt`); images.txt now reports `cases 86` rather than counting shell argument words.

| Case | Instruction ratio |
| --- | ---: |
| external/sunspider-1.0/string-unpack-code | 1.02717746 |
| sentinel/prototype_method_call | 1.02583470 |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.02155102 |
| external/sunspider-1.0/access-binary-trees | 1.02048402 |
| development/graphlib/paths | 1.01990818 |
| external/sunspider-1.0/controlflow-recursive | 1.01177744 |
| external/sunspider-1.0/bitops-bitwise-and | 1.00965338 |
| external/sunspider-1.0/bitops-3bit-bits-in-byte | 1.00845475 |
| external/sunspider-1.0/crypto-sha1 | 1.00733624 |
| external/sunspider-1.0/bitops-bits-in-byte | 1.00727731 |
| external/jetstream3-js-subset/gaussian-blur | 1.00668174 |
| external/jetstream3-js-subset/cdjs | 1.00596863 |
| development/js-yaml/document | 1.00408113 |
| external/kraken-1.1/audio-dft | 1.00365246 |
| broad/function_call_reordered | 1.00356000 |
| broad/plain_function_call | 1.00344681 |
| broad/captured_read | 1.00327565 |
| external/sunspider-1.0/string-validate-input | 1.00300185 |
| external/sunspider-1.0/bitops-nsieve-bits | 1.00283842 |
| external/sunspider-1.0/access-nbody | 1.00232232 |
| external/sunspider-1.0/math-spectral-norm | 1.00188262 |
| broad/function_call_two_args | 1.00177708 |
| broad/many_locals_call | 1.00171991 |
| broad/string_slice | 1.00157426 |
| external/sunspider-1.0/3d-raytrace | 1.00132530 |
| external/jetstream3-js-subset/hash-map | 1.00128216 |
| external/kraken-1.1/imaging-desaturate | 1.00110974 |
| broad/object_allocation | 1.00090495 |
| broad/array_allocation | 1.00081469 |
| external/sunspider-1.0/crypto-md5 | 1.00068272 |
| external/kraken-1.1/audio-oscillator | 1.00056597 |
| sentinel/recursive_call_tree | 1.00000933 |
| broad/array_dynamic_read | 1.00000254 |
| external/sunspider-1.0/regexp-dna | 0.99875713 |
| broad/property_read | 0.99777406 |
| broad/array_read | 0.99758903 |
| broad/closure_allocation_call | 0.99711023 |
| external/kraken-1.1/stanford-crypto-ccm | 0.99607122 |
| external/kraken-1.1/audio-fft | 0.99598513 |
| external/kraken-1.1/audio-beat-detection | 0.99562510 |
| external/sunspider-1.0/string-base64 | 0.99555671 |
| external/sunspider-1.0/math-partial-sums | 0.99490257 |
| broad/property_write | 0.99449549 |
| broad/array_write | 0.99415112 |
| external/sunspider-1.0/access-nsieve | 0.99352340 |
| external/sunspider-1.0/date-format-tofte | 0.99331170 |
| broad/dynamic_method_call | 0.99313380 |
| development/lodash/transform | 0.99136899 |
| development/papaparse/csv | 0.99112354 |
| sentinel/string_key_map_churn | 0.99086239 |
| broad/top_level_function_call | 0.99037263 |
| broad/captured_write | 0.99034174 |
| sentinel/polymorphic_call_site | 0.98915141 |
| external/kraken-1.1/ai-astar | 0.98911946 |
| development/immutable/persistent | 0.98907397 |
| broad/math_abs | 0.98824118 |
| broad/property_dynamic_read | 0.98753713 |
| sentinel/capturing_closure_call | 0.98750339 |
| external/kraken-1.1/imaging-darkroom | 0.98703315 |
| external/sunspider-1.0/3d-morph | 0.98684095 |
| broad/method_call | 0.98677638 |
| development/big.js/arith | 0.98602714 |
| external/sunspider-1.0/access-fannkuch | 0.98540047 |
| external/sunspider-1.0/crypto-aes | 0.98497359 |
| external/sunspider-1.0/string-fasta | 0.98459238 |
| external/jetstream3-js-subset/stanford-crypto-aes | 0.98208674 |
| external/kraken-1.1/json-stringify-tinderbox | 0.98136138 |
| broad/empty_loop | 0.98128013 |
| development/matter-js/simulate | 0.98098562 |
| external/kraken-1.1/stanford-crypto-aes | 0.97986987 |
| external/sunspider-1.0/math-cordic | 0.97966222 |
| external/kraken-1.1/imaging-gaussian-blur | 0.97859344 |
| development/diff/compare | 0.97853817 |
| development/moment/dates | 0.97775942 |
| external/kraken-1.1/stanford-crypto-sha256-iterative | 0.97729749 |
| external/sunspider-1.0/date-format-xparb | 0.97709932 |
| broad/branch_arithmetic | 0.97410777 |
| external/kraken-1.1/stanford-crypto-pbkdf2 | 0.97405876 |
| broad/global_read | 0.97376811 |
| broad/array_index_of | 0.97209003 |
| development/meriyah/parse | 0.97096351 |
| broad/local_read | 0.96951403 |
| external/sunspider-1.0/3d-cube | 0.96917801 |
| external/sunspider-1.0/string-tagcloud | 0.96113000 |
| external/kraken-1.1/json-parse-financial | 0.95212415 |
| sentinel/heterogeneous_property_read | 0.89532748 |

The regenerated acceptance chain lives outside the repository in workspace `tmp/t056-m1a-chain/`, bound to
this candidate and its new exact/pre-gate jobs. It preserves the lead's ZoneInfo-from-/etc/localtime window
implementation (no time.tzset call; compatible with the iMac uv Python), checks `done 0` and no DISTURBED
between jobs, and rechecks variant-0 hashes before any timing. The old READY.id belongs to the superseded
candidate and is not reusable. The lead starts with a fresh correctness-ready job, then K=6 selection,
formal30, stage-mode continuation and `decide --mode stage`; held-out remains a final-promotion requirement.
No acceptance-chain job has been submitted for the new candidate.


## Attempt 1 of 2: valid K=6 rejection

Job **`20261020T210106Z-t056-m1a-chain-selection-k6`**, candidate `0a9af81b` against frozen base
`f58c696e`, completed **done 3**, `label=reject`, complete=true, missing=[], with no DISTURBED marker.
This is a **valid failed screen**: attempt **1 of 2 spent**, **one attempt remains across A+B combined**.
The implementation shape at this image is rejected; this does not close the mechanism family. Downstream
formal/continuation/decision jobs stop at the failed-prerequisite check. Do not rerun this image as a new
acceptance or reuse its passing instruction pre-gate as timing acceptance.

Natural image SHA-256: base `b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e`,
candidate `1970bb690fd9445e94036df51eb29ff7580c10ef16f0f298d0b0cee2112b7491`.
All six lane/portfolio gates pass; actual per-layout portfolio geomeans span **0.99253276–0.99900745**.
The five failures are all `persistent six-layout wall cost`, wall band **1.025**. Ratios below are
candidate/base; the first two columns are natural placement, followed by wall medians for variants 0–5.

| Case | Natural wall | Natural instructions | Six layout wall medians |
| --- | ---: | ---: | --- |
| broad/local_read | 1.07108483 | 0.96955200 | 1.071085, 1.027881, 1.054727, 1.144202, 1.086894, 1.096163 |
| broad/property_dynamic_read | 1.07853161 | 0.98752372 | 1.078532, 1.045418, 1.059201, 1.057602, 1.052468, 1.043657 |
| broad/branch_arithmetic | 1.05693929 | 0.97386144 | 1.056939, 1.039396, 1.072086, 1.131012, 1.036699, 1.135331 |
| external/sunspider-1.0/access-binary-trees | 1.05264355 | 1.02048312 | 1.052644, 1.057757, 1.045871, 1.040363, 1.047475, 1.045481 |
| external/sunspider-1.0/string-unpack-code | 1.04080178 | 1.02691898 | 1.040802, 1.042394, 1.051884, 1.048312, 1.039864, 1.047991 |

Raw evidence: workspace `outputs/imac-bench/20261020T210106Z-t056-m1a-chain-selection-k6/out/`
(`verdict.json`, immutable `screen/panel.json`, appended `screen/complete.json` and per-variant raw records).
Next: diagnose executor/codegen placement and real tree/unpack instruction work, then have the lead run
prospectively declared diagnostic triage before deciding whether to spend the sole remaining attempt.
No new timing job is authorized for the implementation owner.
