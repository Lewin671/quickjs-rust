# T062: Prospective threshold-aware formal precision

- Status: closed-landed; branch-only tooling/policy change, no integration.
- Implementation: present; v6 opts into two-look exact median case intervals.
- Verified at: this T062 tree on base 89a13699, 2026-10-19; full check.sh passed; pre-commit enforces the staged fast gate.
- Evidence: T060 qualification and v5 policy; CPU-only validation recorded below.
- Unresolved: no fresh timing or runtime candidate tested; not a new qualification run.
- Next action: integrate the policy change after review; future plans bind v6. T058 is not replanned here.

## Scope and diagnosis

Owner: `agent/precision-rule/codex`, base `89a13699`; benchmark tooling,
policies and performance documents only. No runtime/source benchmark edits.
No jobs submitted, no iMac timing launched. Copied the existing sealed T060
formal bundle and 44-block continuation read-only; recomputed the seal hashes.
The v5 policy's qualification evidence already binds their hashes.

The premise "cannot reach 3% at 240" is not established by qualification:
**0/86** formal cases project above 3% at 240 under the old square-root rule;
all 86 formal, 86 screen and 9 held-out qualification records actually passed.
Only formal `external/sunspider-1.0/bitops-3bit-bits-in-byte` extended: pilot
1.005460 [0.988195, 1.041582], h=3.5926%, predicted n=44; at 44 its
h=1.4285%. Square-root projection at 240 is 1.2702%, not an impossibility.
Next-largest formal pilot widths: audio-dft 1.6996%, imaging-darkroom 1.5578%,
regexp-dna 1.5230%. These are A/A results, not T058 candidate effects.

Short SunSpider whole-process durations (60 base/candidate observations):

| Case | Median ms | Min/max ms | Paired log-ratio SD |
| --- | ---: | ---: | ---: |
| controlflow-recursive | 11.8219 | 11.3248 / 12.5514 | .02041 |
| bitops-bitwise-and | 13.8201 | 13.3981 / 14.5918 | not used |
| math-spectral-norm | 15.8328 | 15.5746 / 17.2755 | not used |
| bitops-3bit-bits-in-byte | 17.3983 | 16.2629 / 20.4504 | .07424 |

`process.run_process` times launch through child exit with `perf_counter_ns`;
external programs have no in-process amplification. Read-only iMac clock
metadata on 2026-10-19 reports monotonic `mach_absolute_time`, resolution 1 ns.
That is clock resolution, not a measured historical jitter bound. Even two
1 ns rounding errors are only .0000115% of 17.4 ms, orders below the spread.
Startup/scheduling/execution variation is plausible; these files do not
separate their contributions. The shorter, quieter recursive case disproves
"short duration alone determines noise". No irreducible floor or nonstationary
component can be estimated reliably from one 30-block cohort.

For iid lognormal ratios, asymptotic median h is about
1.96*sqrt(pi/2)*sigma/sqrt(n): at n=240, sigma=.1892 already predicts h=3%.
The observed .07424 projects near 1.18% by that approximation; sigma=.20 is a
**prospective stress model**, not inferred from T058, and predicts 3.17%.
Even when expected precision fits, random intervals can miss it. Width is an
uncertainty summary, not the question "does the evidence resolve this limit?"

## Decision and alternatives

The normative contract is [formal precision v2](../../docs/layout-policy-artifacts.md#prospective-formal-precision-v2).
V6 inherits v5 measurement/qualification unchanged and hash-binds a new
analysis rule; it is not a claim that fresh v6 A/A timing was collected.
Two fixed looks (frozen pilot and 240), central exact median intervals with
97.5% per-look coverage, union bound >=95% over both. Each directional error
is <=2.5% across both looks under iid stationary paired blocks. Exact ranks
(zero-based): 30 -> [8,21], 60 -> [20,39], 240 -> [102,137]. No bootstrap
approximation or uncorrected adaptive middle look supplies the new exemption.
The point statistic, host schedule, samples and decision thresholds stay fixed.
Diagnostic-only application to the A/A pilot with v5 control limits leaves 85
cases ready and the short bitops case needing 240; this is not a re-decision
or a new qualification verdict.

Precision is required only while the interval has not resolved the actual
case threshold; crossing intervals at cap stay incomplete. A favorable wide
interval is evidence against regression, not a tolerance waiver. Unfavorable
intervals still meet the existing payoff/stage/persistence checks. Mandatory
layout/instruction/correctness/held-out/lane gates remain. Statistical coverage
is per-case across looks, not a guarantee for all inventory intervals,
nonstationary observations, empirical layout persistence or repeated units.

Rejected alternatives:

- Amplification changes what whole-process startup-inclusive cases measure,
  and can change scope/tier behavior. It requires new manifests/protocol
  binding, A2 bands, independent A/A and formal qualification. T060's measured
  A2/estimator/formal/power jobs alone total 10,232 s (>2.84 h) on the iMac,
  excluding independent screen pilots, build/audit, replay and queue delay.
  This is a historical lower-cost reference, not an estimate of amplified
  runtime. Qualification does not justify that disruption or cost now.
- Wider predeclared control tolerances would hide real costs between the old
  and new limits, and would prevent such cases paying for a target. No band
  or tolerance is widened here.
- Three nominal 95% looks, including a width-selected middle N, lack a 95%
  sequential coverage argument. Fixing only early exit with unadjusted
  intervals would retain that problem; fixed looks and alpha spending avoid it.

## Validation

CPU-only, local Mac, stdlib Python; no benchmark timing on either Mac.
Reproduction: `python3 -m tools.benchmark.precision_validation --bundle
<copied-T060-formal-bundle> --trials 10000`. It checks every sealed file hash,
reconstructs all 86 pilot ratios and matches the original 20,000 seed-0
bootstrap draws via cached median rank pairs. Seed 62019 generates 10,000
independent 240-block streams per scenario, iid lognormal paired ratios.
Two noise scales: the qualification case's log SD and predeclared .20 stress.
Limits are 1.05, chosen independently of T058 candidate data and close to
v5's common B_i=1.050127. This isolates a mandatory formal directional gate;
it is not a simulated whole-engine promotion probability.

Full original rule replay includes its width-selected predicted N, pilot
retention, cap fallback and imprecision veto. New replay uses exactly its two
fixed looks. Each table cell is old -> new, percent; "incomplete" counts
unresolved directional truth away from the boundary, not missing inputs.

| Log SD | True ratio | Incomplete % | False pass % |
| ---: | ---: | ---: | ---: |
| 0.07424 | 1.00 | 1.31 -> 0.00 | n/a -> n/a |
| 0.07424 | 1.05 | n/a -> n/a | 4.19 -> 1.98 |
| 0.07424 | 1.06 | 77.80 -> 74.31 | 0.62 -> 0.15 |
| 0.07424 | 1.10 | 2.02 -> 0.00 | 0.00 -> 0.00 |
| 0.07424 | 0.90 | 0.00 -> 0.00 | n/a -> n/a |
| 0.20000 | 1.00 | 83.90 -> 21.90 | n/a -> n/a |
| 0.20000 | 1.05 | n/a -> n/a | 0.97 -> 1.98 |
| 0.20000 | 1.06 | 96.92 -> 93.78 | 0.27 -> 0.66 |
| 0.20000 | 1.10 | 84.08 -> 26.94 | 0.00 -> 0.04 |
| 0.20000 | 0.90 | 82.71 -> 0.00 | n/a -> n/a |

At the boundary, passes are counted as false assertions of favorable side
(the truth equals the limit). The new 1.98% estimate has Monte Carlo standard
error .139 percentage points. Fixed-cap-only results isolate the width veto:
stress neutral incompleteness 84.32% -> 22.61%; strong gain .90, 83.22% -> 0%.
The sequential new rule rejects .02% of stress-neutral streams falsely;
its directional error is controlled, not zero. No model guarantees resolution
near a limit or under nonstationary host noise.

Monte Carlo estimates are model-specific, not measured host error rates;
0/10,000 has a one-sided 95% upper bound about .030%. The new rule need not
reduce every empirical false-pass estimate: removing a width-only veto can
increase passes under both truth states. It does not increase the nominal
one-sided 2.5% error budget or weaken a regression limit. The old fixed-width
rule's incidental conservatism is not a declared alpha guarantee.

## Prospective scope and T058

V2 continuation segments explicitly name their rule. Replay rejects mixing
old-rule and v2 chains, altered raw evidence and unapproved block counts.
Only a newly frozen plan binding v6 gets the rule. Existing v1-v5 artifacts,
plans and their decisions are not rewritten. T058 attempts 1 and 2 are not
re-decided, pooled or assigned another attempt by this change.

**A fresh T058 proposal is allowed, but a third retry of its exhausted frozen
leaf is not.** T022 requires a fresh current-base comparison/opportunity queue,
new exact-image profile and newly validated/frozen plan before implementation
or acceptance; a rejected implementation does not ban a general mechanism.
The new proposal must justify its current priority and genuinely new evidence,
freeze v6 and its complete attempt/measurement budget prospectively, retain
both historical outcomes, and collect fresh acceptance evidence (all layout,
formal, held-out, resource if applicable and exact-candidate zero-gap gates).
A rebased candidate includes other changes and needs explicit attribution;
old timing cannot verify it. Policy adoption alone grants no extra attempt,
and renaming the old plan is insufficient. No proposal, plan or timing is
created for T058 here.

## Changed files

- Rule/statistics and raw collection/replay: `tools/benchmark/formal_precision.py`,
  `layout_formal.py`, `layout_formal_continuation.py`, `layout_policy.py`.
- CPU-only study and coverage: `tools/benchmark/precision_validation.py`,
  `tools/benchmark/tests/test_formal_precision.py`.
- Frozen prospective artifact: `benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v6.json`
  and its `.sha256` sidecar; v1-v5 and all frozen unit plans unchanged.
- Hosted import integrity: `benchmarks/performance-policy.json` and
  `tools/benchmark/performance_policy_frozen.py` include/hash the new helper.
- Owning docs: `docs/performance-workflow.md`, `docs/benchmarking.md`,
  `docs/layout-policy-artifacts.md`, `tasks/T022-performance-priority-controller.md`,
  `tasks/README.md`, and this record. No dependency or engine API changes.

## Final verification

`PYTHONDONTWRITEBYTECODE=1 CARGO_INCREMENTAL=0 ./scripts/check.sh` passed:
format/clippy including feature builds, Rust workspace/feature tests, 659
benchmark-tool tests, all 93 frozen unit plans, 35 script tests, context,
hook isolation, shell syntax, file sizes and all 5,170 curated Test262 cases.
No focused engine semantic slice matches this tooling-only change. The initially
empty Test262 checkout was initialized at pinned `42496613`; no pointer changed.
The policy's case-per-line formatting was then checked for identical parsed
values, updated SHA-256 and successful prospective-policy loading. Eight new
focused tests cover exact coverage/ranks, decisions, prospective binding and
lossless 240-block replay with unapproved counts refused. Hosted policy audit
also passed. Commit's pre-commit hook runs `check-touched.sh --staged --explain`.
Build outputs created here are removed after the commit; no timing queued,
no merge and no push. The initialized read-only reference checkout is retained.
