# T056 pragmatic stage-A qualification and lead handoff

- Status: candidate qualified for the lead diagnostic chain, not acceptance.
- Implementation: runtime8f898dd9 (005b8dd4 plus same-hash weak-cache cleanup),
  isolated on stage-A; ObjectData120B, one retained header change. Not on main.
- Verified at:8f898dd9 full gate/NG255/subset5170,14 layout tests and5 feature
  batch tests; runtime CI39653532878 green. iMac182852Z86/86,183100Z12-image
  K=6 qualification done0; no owner timing or last-attempt consumption.
- Evidence: [bundle](../diagnostics/t056-pragmatic/README.md),
  [canonical instructions](../diagnostics/t056-pragmatic/instruction-qualification.json),
  [K=6 images](../diagnostics/t056-pragmatic/panel/qualification.json),
  [source boundary](../diagnostics/t056-pragmatic/source-scope.json),
  [initial implementation](T056-pragmatic-stageA.md), [PMU closure](T056-pmu-closure.md).
- Unresolved: no new wall/acceptance or full exact Test262 result yet; empty/
  unpack instruction excess reduced, not eliminated. Equal-instruction relative-PC
  sensitivity stays unexplained; M2 kernel isolation is still a separate plan.
- Next action: lead runs fresh correctness→bound86→T061 diagnostic triage→four-case
  K=6 mini with the supplied scripts. No further PMU/helper/sudo/install action;
  preserve original v6 thresholds and the last of2 attempts across A+B.

## Final source and verification

Canonical SHA-256:
`10211f836606e0c14ce5d9a427f818adda1ff2c5b1c8c7cc615860d8b2a4b57e`.
Source commits005b8dd4/8f898dd9 are dfff3bb6 plus this task's own M1 changes;
no later main runtime, parser, lexer, AST, Cargo/dependency, unsafe/FFI, environment
snapshot or tier change. Packed cold flags stay, index_property_count is usize;
header order/types match the previously qualified flags_order120B variant.
Other headers, scalar page alignment and production link-order files are unchanged.

The new batch API evaluates complete descriptors first, canonicalizes/deduplicates
first-slot/last-descriptor, publishes one immutable final layout and retains
incremental owned edges afterwards. Symbol/prototype state and special numeric/
global identity paths remain. Arrays/String/TypedArray aliases resolve after
method batches. Weak final-layout sharing adds no strong retention cycle.
A follow-up regression found same-hash dead Weak allocation metadata could
accumulate below ContentCache's table-wide sweep threshold: a batch followed
by append can drop the batch root while descendants keep its key atoms live.
Insertion now prunes dead entries within that bucket;1000 repeated dead-shape
publications remain bounded. No field/size or performance-admission knob changed.

Local full checks and255 NG comparisons passed on both runtime revisions;
final8f898dd9 full includes5170 curated Test262 cases. Fast gate ran runtime
library tests but mapped no focused Test262 slice for these paths, so the full
subset supplies that coverage.5 perf-counter batch tests additionally cover
ordered duplicates/descriptors, numeric fallback, cache invalidation, domain
sharing/value independence, dictionary boundary and linear field construction.
14 layout tests include transient-vs-repeated probes and weak bucket reclamation.
Runtime CI8f898dd9 is green; a later metadata-only handoff commit is distinct.

## Non-timing instruction evidence

`20261021T182852Z-t056-pragmatic-final-instructions86` done0/260s, same pinned
agents/fat16/abort/align16 recipe, exact baseline b0dfc65d… and final10211f83… .
One original-policy instruction capture per86 cases, no blocked case; highest
is graphlib/paths1.0173695 (cutoff1.03). No instruction pre-gate threshold changed.
The snapshot does not replace fresh full exact correctness or the lead's bound86.

Separately, three warmed whole-process instruction captures per role/case;
use medians from this one final cohort only, not favorable values across jobs:

| Whole case | f58 base | dfff previous | final8f898 | previous/base | final/base |
| --- | ---: | ---: | ---: | ---: | ---: |
| empty |13,865,417|15,325,020|14,864,654|1.105269|1.072067|
| unpack |1,032,582,327|1,043,471,751|1,041,968,153|1.010546|1.009090|
| CLI help |8,649,881|8,659,837|8,679,453|1.001151|1.003419|

Thus empty/unpack work fell, but+7.21%/+0.91% remains; CLI-only is slightly
higher. These are instruction point estimates, not new wall conclusions or
confidence-qualified effects. All samples and all86 ratios are retained.

Initial005b8dd4 instruction job180725Z passed86/86; its three-capture cohort
and old K=6 qualification181036Z remain explicitly initial, not final-image
receipts. That job also ran an isolated feature/event build:
[events at005b](../diagnostics/t056-pragmatic/events-005b.json). Relative to
previous recorded dfff, empty field copies11,845→6,981; unpack45,656→40,792;
unpack linear text_matches comparisons830,096→759,646. Index builds return
from4/182 indexed fields to468/32,894: the464-index allocation saving is **not
retained**; repeated scans are amortized instead. Do not call these all possible
hash/key comparisons or convert them into an exact instruction/time saving.
These events precede the metadata-only weak fix and are not rerun canonical
instruction evidence for8f898dd9.

The construction mechanism originates in3506f4db's immutable per-add prefix
layout authority, not the later112B header patch. Earlier attribution placed
nearly all new shape/field work in initialize_builtins;175023Z identified the
large method-table installers. This identifies added construction work, not an
exclusive numerical attribution of every startup instruction to one commit.
Exploratory180208Z merely sampled already-cached historical images once; early
commits were absent. Its [inventory](../diagnostics/t056-pragmatic/startup-history-inventory.json)
is not a qualified instruction bisect and is not used to select this candidate.

## Lead bundle and preserved risk

`20261021T183100Z-t056-pragmatic-final-k6-qualification` done0/394s: all12 images,
all offsets0/16/64/240/512/1008, zero body/address drift modulo verified relocations,
uniform declared shifts, NG255 each and identical global/local/unpack/empty
smokes. No image selection, padding search, re-signing or timing run. The
executed producer and current reproducer differ only in the module docstring;
[AST/hash proof](../diagnostics/t056-pragmatic/panel/producer-proof.json) records both.

lead/ contains the fresh exact42672/53572-path correctness→same-image86→original
T06128-case triage chain. panel/ binds all three future receipts, exact helper/
image/unit/host/file seals and final prebuilt images. It runs four cases,
6 layouts×5 pairs×4 roles=480 units,720 measured executions+480 launch preflights.
Same active path and separate-inode A/A clones, original numeric AA and5/6
routing criteria; no ordinary counter/metric retry or case/layout/pair deletion.
Thirty-minute guard, local03–05/15min-before03 exclusion and35min admission
remain. Budget679.052s projection<1560 collect/1680 body/1800 whole run including
quieting. Lead triage remains the other diagnostic, never a last-attempt screen.

Validators simulate the whole mini, all routing boundaries and night/DST/timeout
rules; future correctness/86/triage receipts are mocked in that simulation,
with their real rejection/binding guards separately tested. No JS engine or
helper is executed by validators. They write future reports under target/;
committed validation snapshots remain frozen.

Known risk is unchanged: count+flags jointly triggered56 relative-PC changes
in the equal-instruction typed numeric loop. Ops/bank/stack/zone, uniform code
shifts and JCC boundary counts did not explain it in the recorded experiments;
there is no per-iteration ObjectData flag/count access. Full evidence and limits
remain in the [initial record](T056-pragmatic-stageA.md) and its linked archives.
The120B choice removes the observed combination, not the unknown hardware cause.
Kernel isolation remains M2 follow-up under a separate architecture/unit plan.
Neither these instruction snapshots nor the mini authorizes acceptance, resets
an old rejected attempt or integrates stage A into main.
