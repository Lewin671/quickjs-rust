# Layout policy and panel artifacts

`benchmarks/policies/<host>-layout-vN.json` is a measured, hash-checked
`quickjs-layout-policy` schema-1 artifact. A schema-3 unit binds its exact
repository-relative `policy_path` and `policy_sha256`; policy changes require
a new version. `host` records machine/OS/CPU/memory/toolchain/linker, Cargo
configuration, canonical recipe and manifest hashes; `host_node` binds the
specific machine. Analysis verifies the recorded host; collectors also
verify the live host. An unqualified policy is never usable.

`cases` maps the 76 legacy selection cases (86 with development) to `wall_band` B_i and
`instruction_band` I_i; `held_out` maps all nine acceptance programs to their
wall bands (v4 adds per-program I_i; see [held-out policy v4](#held-out-policy-v4)). `lanes` holds broad, sentinel, external, optional development, equal-case portfolio
and held-out ceilings. Qualification retains joint pilot intervals, extended case intervals and source
hashes. B_i covers A2's two observed layout estimators and global A/A median
bias; I_i covers its rounded A/A instruction envelope. v2/v3 held-out bands use
only rounded reciprocal 95% A/A interval endpoints. These empirical envelopes do
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
held-out has one natural screen of 30 pairs, or (v4) six after `--held-out --complete`. Raw case packets retain every
sample, separate calibration, per-role costs and paired ratios. Replay
recomputes all ratios, validates complete inventories and binds natural
binary hashes/revisions to the sealed formal bundle. It refuses missing,
short/nonpositive valid increments, altered paths, checksums, samples or hashes.
`panel.json` stays immutable; completion adds `complete.json`.

Decision schema 5 carries `layout_verdict`, raw-replayed formal statistics
and policy/panel/held-out SHA-256 bindings. Case effects are paired-ratio
medians; lanes/portfolio are equal-case geomeans. Their 95% intervals jointly
resample block IDs with 20,000 draws, seed 0, and require >=30 blocks and
<=3% relative half-width, except [v6 formal cases](#prospective-formal-precision-v2). Promotion checks held-out upper intervals against
its program/lane bands as well as point estimates. Legacy decisions retain
their own schema and statistics. Shared formal snapshots use equal-length role/hash
prefixes that preserve the role-dash, and exclusive creation; measurement protocols are
throughput v10, sentinels v3 and resources v4.


Queue schema 3 additionally records the measured host. Schema-3 unit
validation checks that host against the policy and refuses stale inventories
before implementation. New queues require schema-3 plans; older queue schema 2 remains readable
for historical plans.
Instruction-blind decisions require `--resource-report` and `--resource-raw`:
the current resource analyzer replays the report against the manifest it
names -- the frozen `benchmarks/resources.json` or the qualified manifest the
policy host derives from the live recipe -- binds both engines and host, and
retains all reported costs. Missing/incomplete resource evidence
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
remains valid for plans that name no development case.
[Qualified iMac v3](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v3.json)
adds the development lane (86 cases) and is required for any plan that names
a development case; v1 remains frozen. [T057](../tasks/archive/T057-development-policy.md)
owns the v3 qualification record. [Qualified iMac v4](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v4.json)
replaces only v3's held-out gate and lane rule; [T059](../tasks/archive/T059-held-out-layout-policy-v4.md) owns its record.
[Qualified iMac v5](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json)
recalibrates every gate for case-sharded workers-4 collection;
[T060](../tasks/archive/T060-parallel-layout-policy-v5.md) owns its evidence and v4 comparison.
[T055](../tasks/archive/T055-layout-noise-control.md) owns qualification evidence.
Screen/held-out A/A binds the frozen band-source executable; canonical
formal A/A binds its own declared revision and byte-identical two roles.
A different source revision is not required to have the old binary hash.


## Case-owned collection schedules

`perf-loop.sh --plan` (schema 3), `layout_iteration`, `layout_panel`, `layout_development_a2`,
`layout_calibration`, and npm lane `run` accept `--workers N` (1..8, default 1).
One is the historical serial schedule; more than one uses deterministic
round-robin case shards. Each fork completes all roles, pairs, load-only
replacement rounds and requested layouts of its cases. Assets/preflight and
final load sealing belong to the parent. Pair eligibility and caps below apply
unchanged, including simultaneous disturbance of several shards.

Artifacts bind `measurement_schedule` (`kind`: `serial` or `case-shards-v1`,
`workers`: N), deterministic `case_shards`, and raw `case_shard`/`worker_pid` ownership.
Panel packets also bind the schedule. Replay conserves the inventory and
rejects missing/changed ownership or a policy/evidence schedule mismatch;
absent historical metadata denotes serial. Completion preserves its natural
screen's schedule. Precision continuations retain the pilot schedule and
case shard. `collection_seconds` times collection including settling/recovery,
excluding preparation/preflight; panel run records preserve natural/completion
durations. npm reports reconstruct their schedule from hashed raw rows.

Policies qualify for one collection schedule; `formal_lane_schedule` separately
binds the canonical formal schedule. A serial policy cannot authorize parallel
screens. [T060](../tasks/archive/T060-parallel-layout-policy-v5.md) owns workers-4
recalibration, qualification, v4 comparison and measured speedup; support for
other N is tooling capability, not a qualification claim.

## Load-only pair recovery

New layout collectors require the serialized timing queue's `BENCH_OUT` and
adjacent `intruders.log`. Raw packets add `attempts` (cumulative alternating
index, epoch window, process samples), `rounds` (append count and sealed load
snapshot), `valid_attempt_indices`, and `complete`. On healthy cases, original
`samples`, `costs` and `ratios` contain exactly the required valid pairs. A JSONL
stream retains all processes, including interrupted/failed rounds and separate
calibration. An invalid-load pair's metrics never choose its replacement;
correctness and positive sample checks still cover every process.

A load-valid short/nonpositive increment is a terminal **case health failure**,
never a replacement trigger. `pair_recording.py` retains every attempt and its
signed wall/user-CPU/instruction increments, load flags, and per-role N/2N
process distributions under `health` (`collection: complete`, `analysis: failed`).
Its `complete: true` means the requested process inventory completed, not that
acceptance evidence is usable. Timeout, nonzero engine exit, or checksum failure
instead records `collection: failed`, `complete: false`, the partial attempt or
calibration, and the failed process result/output when available. These failures
are never waived by load eligibility. The recorder writes the unsealed raw
failure packet before monitor/replay/workload checks, then atomically finalizes
its own packet before returning. A secondary error is retained alongside all
available raw processes and re-raised; replay refuses packets carrying that
infrastructure error. Failed development packets use the healthy path's frozen
manifest, timeout and workload-hash contract, and expected stdout is derived
from the lane contract rather than trusted from the raw file. Infrastructure errors (spawn failure,
missing counters, malformed load history, I/O, caps) still abort; they are not
converted into usable case evidence.

Serial and case-sharded schedulers finish all other already requested cases
and layouts after a terminal case failure. Failed summary rows have empty
`ratios` and the replayed `health`; healthy rows keep their historical shape.
Affected panel packets say `inventory_complete: true`, `readiness: incomplete`.
Incomplete verdicts list every case's collection and analysis state. Calibration,
A2, panel, screen, replay and iteration artifacts also expose these markers at
their top level when health fails; healthy artifact bytes are unchanged. Their
commands exit **4** for case-health incompleteness (ordinary gate failure remains
3). Schedule-calibration estimator/pilot/A2 receipts and speed reports propagate
the same state and exit status; health failures never trigger load replacements.
Statistical consumers raise `CaseHealthFailure`, rather than reading absent
costs/ratios. Other infrastructure errors retain their existing failure paths.

After a natural health failure, automatic iteration and speed collection stop
before requesting variants 1..5, even when K=6 would be mandatory for acceptance:
those samples cannot repair the failed required case. Every case/layout already
requested from the scheduler still finishes. An explicit panel collection or
`--complete` remains available for a separately requested diagnostic inventory;
it cannot change the failed attempt's standing or budget.
A missing/interrupted case is not a terminal health result: replay still refuses
an incomplete inventory. Failed raw packets carry no usable `samples`, `costs`
or `ratios`. Screen and panel replay authenticate health from raw processes,
load selection, identities and hashes; default statistical readers refuse it.
Any failed required case keeps the overall verdict incomplete, with no lane,
portfolio, geomean, formal aggregate, or promotion standing. Healthy evidence
retains the existing statistic and report bytes. Thresholds, attempt accounting,
load-only eligibility and the meaning of a passing screen are unchanged;
historical failed packets and closed decisions are never rewritten.

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

Calibration, formal A/A and pre-v6 candidate plans apply this standing rule.
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
Under v2/v3 a held-out fallback requires all six held-out layouts (`--held-out-panel`
with `--held-out --pairs 30`), each passing; under v4 the persistence rule applies.
No 3% precision limit or adopted A2 envelope is widened.


`layout_formal_continuation --bundle <sealed-dir> --output <new-dir>`
automatically continues all deficient cases of a formal A/A or candidate/base
bundle; `--case <id>` limits it to a subset and `--previous <continuation.json>`
resumes that case. A chain records `comparison: candidate-base` unless both
roles share source and image; replay refuses the other kind, so A/A
qualification never accepts a candidate chain. Three-role argv, per-role
N/startup eligibility, timeout, workload/image hashes and seeded rotation at
absolute block indexes come from the sealed bundle, without re-calibration.
Removed transient assets are restored at the original argv paths only from
matching pinned bytes; existing files are never replaced. Original sealed
files remain immutable. `decide --formal-continuation <continuation.json>`
(latest chain per case) extends that case's formal statistic; lanes keep the
sealed joint blocks, and a still-imprecise case at 240 stays incomplete. Each segment seals raw process windows/results, load,
source/predecessor hashes and exact retained ratios; replay refuses role/order,
count, correctness, counter, timer or identity drift. Formal load invalidates
an entire appended segment; retained failed segments allow at most three whole
load-only replacements. All other errors stop. Qualification accepts latest
`--formal-continuation` chains, uses all case observations and keeps original
joint lane blocks. Formal cap-imprecise cases also set the policy's diagnostic
band/K=6 status; precision alone cannot block qualification. Payoff, stage,
instruction, precise-case, lane/portfolio and correctness gates still bind.

## Prospective formal precision v2

[iMac v6](../benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json)
freezes `formal_precision_rule: threshold-median-v2`. It inherits v5's qualified
measurement schedule, bands and qualification data unchanged; only new plans
frozen after publication may bind it. No automatic successor applies to old
selection gates. Existing plans, T058 attempts 1-2 and closed decisions keep
their rule, timing and budgets. [T062](../tasks/archive/T062-formal-precision-rule.md)
records independent A/A diagnosis, alternatives and CPU-only validation.

For candidate/base **selection cases only**, the two possible looks are the
frozen 30/60-block pilot and 240 blocks. Each uses a central exact binomial
order-statistic median interval at >=97.5% coverage, with <=1.25% per tail.
Union bound over two fixed looks gives >=95% simultaneous per-case coverage
(<=2.5% erroneous favorable-side crossings). This assumes independent paired
blocks from a stationary distribution; no claim of coverage under host drift.
There is no adaptive middle-N decision look. Cases whose interval is precise
(<=3%) or entirely on one side of the relevant threshold need no extension;
other cases append directly to 240. At cap, a wide crossing interval remains
incomplete. The relevant limit is min(payoff, reciprocal B_i) for targets,
B_i for controls, or min(B_i, stage budget) in a migration. Existing diagnostic
bands retain their separate K=6 contract. Clear unfavorable intervals continue
through the existing rejection/persistence gates; they do not become passes.

`layout_formal_continuation --plan <plan>` loads its exact policy and uses this
schedule. Continuation segments record the rule; raw replay refuses old-rule
chains in v6 decisions and v2 chains in historical decisions. The point
estimator remains the paired-ratio median. No sample is discarded or redrawn.
The formal statistics record interval method, coverage and maximum looks.
The pilot or cap may decide a case, never both as independent attempts.

No regression tolerance is widened. Every mandatory formal requirement must
pass along with the unchanged layout, instruction, lane/portfolio, held-out,
resource and correctness gates. Requiring all mandatory gates is an
intersection-union test: one false mandatory directional assertion suffices
to bound an erroneous promotion by its own <=2.5% tail budget; this is not a
95% simultaneous interval claim over the entire inventory or over attempts.
At most two attempts give a <=5% directional union bound for a fixed violating case; no budget reset is granted.
Existing layout-persistence gates are empirical, not new statistical guarantees.
Calibration/A/A, held-out and joint lanes keep their bootstrap precision rule;
their measurement protocol is unchanged, so measurement requalification is
not needed. Increasing in-process work would require it.

## Development policy extension

Development-enabled policies additionally bind `development_manifest_sha256`
and its exact ten-case inventory. Non-target programs follow the same
instruction floor/hard cap and six-layout wall-persistence rules as external
cases. Development and the expanded equal-case portfolio each have a ceiling.
Legacy policies remain usable without development targets. Held-out stays
acceptance-only, with its protocol and bands inherited unchanged.

`layout_development_a2` records natural-versus-each-placement observations;
`layout_calibration --lane development` records the independent 30-pair pilot
and automatic continuation. `layout_development_qualification` replays both
and the four-lane sealed formal A/A, deriving no wider bands from qualification.
The pre-timing contract and publication standing are owned by
[T057 development policy](../tasks/archive/T057-development-policy.md).


## Held-out policy v4

A policy with `held_out_rule: six-layout-persistence-v4` (iMac v4) gives each
held-out program B_i and I_i, and the lane a ceiling, by the development A2
estimators over six placements of the band-source image (five pairs each),
maxed with the parent's 30-pair A/A band and ceiling; a fresh 30-pair A/A
qualifies them (`layout_held_out_policy a2|qualify`). Every selection band,
ceiling and precision record is the parent's. The same `held_out_rule` marker
also selects v4's lane-event rule for every lane and the portfolio. The decision rule is owned by
[T022](../tasks/T022-performance-priority-controller.md#layout-robust-decisions).

Complete a held-out event with `layout_panel --held-out --complete --pairs 30`
on the natural output: it appends variants 1..5 at the natural pair count and
writes `complete.json`; pass that file as `--held-out-panel`. v2/v3 stay valid
for plans frozen against them, but after v4's publication every decision
evaluates held-out with v4's bands, ceiling, precision and rule
(`HELD_OUT_SUCCESSOR` in `layout_policy.py` binds its path, hash and parents);
the decision records `held_out_policy_sha256`. Promotion still requires the
held-out gate.
