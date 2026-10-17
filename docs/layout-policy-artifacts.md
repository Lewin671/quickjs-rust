# Layout policy and panel artifacts

`benchmarks/policies/<host>-layout-vN.json` is a measured, hash-checked
`quickjs-layout-policy` schema-1 artifact. A schema-3 unit binds its exact
repository-relative `policy_path` and `policy_sha256`; policy changes require
a new version. `host` records machine/OS/CPU/memory/toolchain/linker, Cargo
configuration, canonical recipe and manifest hashes; `host_node` binds the
specific machine. Analysis verifies the recorded host; collectors also
verify the live host. An unqualified policy is never usable.

`cases` maps all 76 selection cases to `wall_band` B_i and
`instruction_band` I_i; `held_out` maps all nine acceptance programs to their
A/A wall bands. `lanes` holds broad, sentinel, external, equal-case portfolio
and held-out ceilings. Qualification retains joint pilot intervals, extended case intervals and source
hashes. B_i covers A2's two observed layout estimators and global A/A median
bias; I_i covers its rounded A/A instruction envelope. Held-out bands use
rounded reciprocal 95% A/A interval endpoints. These empirical envelopes do
not substitute for confidence intervals. The rule itself is owned by T022.

`layout_gate` freezes `mechanism_class` (`cpu-work` or `instruction-blind`),
`memory_wall_targets` (a subset of targets), `panel_required`, `panel_pairs`
(5), `held_out_pairs` (30) and `formal_blocks` (30 or 60). Instruction-blind
units require nonempty memory targets and `panel_required: true`.

`layout_variants` writes a schema-1 `quickjs-layout-variants` identity, six
immutable executable paths and nm/disassembly/normalized-body audits. The
fixed NOP sizes are natural/16/64/240/512/1008; compiler alignment may round
one shift by less than 32 bytes. Every original text symbol must keep its
order, normalized instruction body and a single uniform displacement.
A whole-page movement of `__text` is recorded separately from the
section-relative padding shift; declared leading NOP bytes are verified.
Relocated branches/RIP references resolve to the same symbols/section offsets; only the last function's trailing section filler may differ.
Any drift refuses timing. Only x86_64 macOS release builds are supported.

`layout_panel` writes `quickjs-layout-panel` schema 1: source revisions,
all binary hashes/audits, exact paths, host, preflight and raw case hashes.
Path lengths are compared as filesystem bytes, including non-ASCII names.
`panels` contains natural only or exactly six ordered matching-layout screens;
held-out has one natural screen of 30 pairs. Raw case packets retain every
sample, separate calibration, per-role costs and paired ratios. Replay
recomputes all ratios, validates complete inventories and binds natural
binary hashes/revisions to the sealed formal bundle. It refuses missing,
short/nonpositive valid increments, altered paths, checksums, samples or hashes.
`panel.json` stays immutable; completion adds `complete.json`.

Decision schema 5 carries `layout_verdict`, raw-replayed formal statistics
and policy/panel/held-out SHA-256 bindings. Case effects are paired-ratio
medians; lanes/portfolio are equal-case geomeans. Their 95% intervals jointly
resample block IDs with 20,000 draws, seed 0, and require >=30 blocks and
<=3% relative half-width. Promotion checks held-out upper intervals against
its program/lane bands as well as point estimates. Legacy decisions retain
their own schema and statistics. Shared formal snapshots use equal-length role/hash
prefixes that preserve the role-dash, and exclusive creation; measurement protocols are
throughput v10, sentinels v3 and resources v3.


Queue schema 3 additionally records the measured host. Schema-3 unit
validation checks that host against the policy and refuses stale inventories
before implementation. New queues require schema-3 plans; older queue schema 2 remains readable
for historical plans.
Instruction-blind decisions require `--resource-report` and `--resource-raw`:
the current resource analyzer replays the report, binds both engines and
host, and retains all reported costs. Missing/incomplete resource evidence
blocks promotion. `layout_replay` is diagnostic-only; it accepts a qualified
policy or explicitly unqualified `--diagnostic-bands` and never reopens an
old unit's budget or changes its standing.

Schema-3 `fast_gate` contains only `target_ids`,
`target_max_candidate_over_base` and `max_attempts`. Its controls are the
policy inventory minus targets; legacy `control_ids` and a single control
ceiling are refused rather than silently ignored.

The A2 band seed also binds its source host/recipe. The qualification
publisher refuses transplanting those bands to a new host or recipe after
only an A/A run; the six-layout envelope must be recalibrated there first.

`panel_required: true` freezes an additional six-layout target aggregate.
Those attempts always complete all six layouts, including a weak natural
screen; both the natural and aggregate target gates remain required. An
optional control-event panel does not change the frozen natural target statistic.

An unqualified artifact may retain null budgets for unmeasured lanes. It is
never usable by `validate-unit`, `screen` or `decide`; no fallback ceiling
is invented. The [recorded iMac artifact](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v1.json)
and its adjacent SHA-256 sidecar preserve the original unqualified outcome.
[Qualified iMac v2](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v2.json)
and its adjacent SHA-256 sidecar are the adopted host policy; v1 remains frozen.
[T055](../tasks/archive/T055-layout-noise-control.md) owns qualification evidence.
Screen/held-out A/A binds the frozen band-source executable; canonical
formal A/A binds its own declared revision and byte-identical two roles.
A different source revision is not required to have the old binary hash.


## Load-only pair recovery

New layout collectors require the serialized timing queue's `BENCH_OUT` and
adjacent `intruders.log`. Raw packets add `attempts` (cumulative alternating
index, epoch window, process samples), `rounds` (append count and sealed load
snapshot), `valid_attempt_indices`, and `complete`. Original `samples`, `costs`
and `ratios` contain exactly the required valid pairs. A JSONL stream retains
all processes, including interrupted/failed rounds and separate calibration.
An invalid-load pair's metrics never choose its replacement; correctness and
positive sample checks still cover every process. A valid short/nonpositive
increment fails instead of being replaced.

CPU threshold is the existing monitor's >=200% aggregate per UTC second;
process inclusion remains >=30% and cadence 5 seconds. A pair is invalid if
its first-start/last-end window, expanded by 2 seconds, intersects the heavy
sample's one-second timestamp bucket. After each round wait 8 seconds before
sealing load; append exactly the missing number, continuing role alternation.
More than 20% invalid attempts per case or run, or a >=60-second heavy cluster
(sample gaps <=6 seconds), invalidates the whole run. Retain it and replace
whole; independent jobs never pool. Timestamp order, guard completion, original
and appended counts, sample/cost conservation, append-only load history and
caps are replayed. Run summaries seal the final load snapshot and counts;
panels retain separate run bindings for natural and completion collections.
Legacy raw packets retain their frozen whole-run validity rules.

Record user-managed indexing/service state before timing; current qualification
requires Spotlight disabled and photo/media analysis disable flags. Tools do
not change OS services. Canonical multi-worker perf-compare has no sealed
pair-window/retry contract and retains whole-run replacement on disturbance.
Task T055 owns the frozen calibration attempt budget and campaign evidence.

Qualification requires `--band-context` for the recorded A2 host-binding
manifests and `--screen-context` for the committed collector. Only internal
protocol metadata differences are accepted; cases/corpora/recipes stay exact.
`--formal-load-log` supplies the sealed queue `intruders.log`; any heavy sample
refuses qualification, and the accepted log hash is bound in policy evidence.
Host attestation records the run's epoch start, node, full identity and quieting.


## Reference-derived external hang detector

External manifest v3 carries a hash-bound reference timeout policy. Every case
uses max(60 seconds, four times its pinned NG median from the latest valid
canonical reference run), equally for all roles. Reference source revision,
image, run, raw/summary hashes and every median/derived limit are recorded in
the manifest. The loader recomputes limits and refuses missing or changed
cases. Per-run timeout overrides are refused for this protocol; timeout remains
failure. v2 archives retain their original uniform timeout and replay identity.
This bounds hangs without hiding an engine's long tail or waiving precision.


## Precision power and continuation

Every calibration and formal A/A automatically applies this standing rule.
For frozen base_n observations (30 formal blocks) and valid relative CI
half-width h>3%, n=min(240,ceil(base_n*(h/.03)^2)). Retain every pilot
observation and append only the deficit using the original
immutable binary paths, iteration count and cumulative role order. If the
predicted stage still misses 3%, one further stage reaches 240, retaining all
pairs. These decisions use precision alone, never which side wins. Held-out
uses the identical rule and remains acceptance-only.

`layout_continuation --pilot <screen.json> --case <id> --pilot-context <sha>
--output <new-dir>` automatically finishes the predicted stage and cap if needed.
`--previous <continuation.json>` resumes a retained chain. Calibration invokes
this automatically and records latest chains in `power.json`. Segment packets bind pilot/predecessor/raw hashes, binary
identity, the original iteration count and absolute attempt index, with a
separate load log and cap replay for each job. Qualification accepts the latest
`--continuation` per case, recursively replays every prefix and conserves all
ratios. Case statistics use every valid pair; lane statistics keep the original
joint 30-pair cohort because other cases are not remeasured.

At 240, a still-imprecise case is recorded as "imprecise on this host" in
policy `precision` metadata. Its wall band is diagnostic only; every unit
measuring that case requires K=6, even when the natural target fails.
Full-portfolio units therefore always complete the panel if such a case exists.
This band alone cannot reject/promote or tighten target payoff by inverse band.
Target payoff/all-pairs, instruction hard caps, stage budgets, lane/portfolio precision and correctness gates remain binding.
A held-out fallback requires all six held-out layouts (`--held-out-panel`
with `--held-out --pairs 30`); precise program bands and every lane still bind.
No 3% precision limit or adopted A2 envelope is widened.


`layout_formal_continuation --bundle <sealed-dir> --output <new-dir>`
automatically continues all deficient formal A/A cases; `--case <id>` limits
it to a subset and `--previous <continuation.json>` resumes that case. It
requires byte-identical base/candidate source/images. Three-role argv, per-role
N/startup eligibility, timeout, workload/image hashes and seeded rotation at
absolute block indexes come from the sealed bundle, without re-calibration.
Removed transient assets are restored at the original argv paths only from
matching pinned bytes; existing files are never replaced. Original sealed
files remain immutable. Each segment seals raw process windows/results, load,
source/predecessor hashes and exact retained ratios; replay refuses role/order,
count, correctness, counter, timer or identity drift. Formal load invalidates
an entire appended segment; retained failed segments allow at most three whole
load-only replacements. All other errors stop. Qualification accepts latest
`--formal-continuation` chains, uses all case observations and keeps original
joint lane blocks. Formal cap-imprecise cases also set the policy's diagnostic
band/K=6 status; precision alone cannot block qualification. Payoff, stage,
instruction, precise-case, lane/portfolio and correctness gates still bind.
