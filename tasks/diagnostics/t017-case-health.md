# T017: case-health containment adoption

Initial adoption: `63b6fb82`. Subsequent [review fixes](t017-case-health-review.md)
own current job reporting and replay bindings.

Owner: [T017](../T017-performance-benchmark-system.md). Base `eeaf696e`; branch
`agent/t017-case-health/codex`. Adopt T073 proposal part 1 prospectively; part 2
is not adopted. Runtime, qualified policy JSON, frozen unit plans, thresholds,
attempt budgets, load eligibility and T072's two charged attempts are unchanged.

## Design and boundary

`pair_recording.py` returns a typed terminal failed-case result after sealing
load-only selection. Complete failed increments keep all raw samples and signed
wall/user-CPU/instruction differences; timeout, nonzero exit and output-contract
failures keep partial attempts/calibration and process failure results. No failed
case exports costs or ratios. `case_health.py` owns state/diagnostics;
`case_health_replay.py` reconstructs terminal process failures. Every default
statistical reader refuses failed cases. Arbitrary infrastructure exceptions
remain exceptions; no generic worker-pool failure is waived.

`case_schedule.py` continues each case's requested layouts and every unrelated
case, in both the unchanged serial order and deterministic forked shards.
Workers return normally only for a recorded terminal health result, so the
shared `worker_pool.py` needs no change: infrastructure errors still stop its
pending queue. Inventory completion and evidence readiness are separate.
`screen_replay.py`, `layout_evidence.py`, diagnostic `layout_replay.py`,
`layout_rule.py` and `layout_formal.py` replay/refuse health before statistics.
Calibration seals its failed inventory without allocating precision retries.
The owning [artifact contract](../../docs/layout-policy-artifacts.md#load-only-pair-recovery)
defines fields and acceptance rules.

## Verification evidence

At base `eeaf696e` plus this change, focused tests inject a checksum-correct slow
N with unchanged instructions: every requested process is retained, all 3 cases
across 6 layouts finish under serial and workers-4, its signed increment stays
negative, and no aggregate is called. Tests cover short/zero/negative wall and
instruction increments, timeout/nonzero exit/checksum/calibration failure,
continued unrelated collection, load-only replacement, missing inventory,
tampered hashes/state and decision-verifier refusal before formal statistics.

The captured T072 attempt-2 packet is committed unchanged as a test fixture,
SHA-256 `a80ea014415d92c87b29ef8184aca85ab6f08880aabe3275324fba45f6a00497`.
It comes from the existing non-timing export job (fetched, no new iMac job).
Old `eeaf696e` replay and new replay both refuse the historical `complete: false`
packet. An observational copy with collection completion enabled, without any
sample/load changes, fails old analysis with `invalid screen increment` and new
analysis with `CaseHealthFailure`: attempt 0 candidate wall -49,952,756 ns,
user CPU -50,000,000 ns, instructions +4,544,284,580, load-valid. No historical
file, decision or attempt is rewritten.

A/A byte comparison against isolated `eeaf696e` modules passes for deterministic
healthy raw JSON, JSONL, result, rendered screen and selection verdict, together
with replay/statistics of an existing complete T058 held-out packet (9 cases,
30 pairs each, all 6 layouts; v4 policy). The combined canonical JSON is
124,424 bytes, SHA-256
`ce403ce3b2658c56484416f53330702f100cc2f0d7109f0e013b504cd84fc9fd`
under both implementations. This is an analysis/serialization comparison of
retained evidence, not fresh host noise or timing qualification.

## Implementation binding and gates

The same inventory/hash procedure as T067 adds the two new health modules to
`performance_policy_frozen.py` and `benchmarks/performance-policy.json`, then
recomputes `_implementation_sha256(ROOT, PREVIEW_IMPLEMENTATION_FILES)` over
all bound paths and their content hashes. The implementation aggregate is
`76b8c92677780a99528ba83c07beb301f8552e839917837d05e045785e0cbf09`.
Measurement protocol hashes, qualified policy JSON, gate flags and plans stay
unchanged. Performance-policy audit passes; no new dependency or public runtime
API is introduced.

Final full gate passes on `eeaf696e` plus this implementation:
`TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`.
It covers default/agents/perf-counters clippy and tests, 744 benchmark-tool tests
(including 12 new case-health tests), 35 script tests, unit plans, context,
size, hook isolation, shell syntax and all 5,170 curated Test262 cases. Log
SHA-256 `d47adf3832bccfeeda303900bfacb4180a92d911762be6aae6e3a18d7a528601`.
Separate performance-policy and external-corpus audits pass. The final A/A
comparison is byte-identical after the last implementation change.

An earlier full run passed with the first 10 health tests; the final run above
adds diagnostic panel replay and counter-wrapper failure coverage. The initial
standalone tool-suite run overlapped implementation-inventory edits and failed
its policy-inventory checks; fresh final runs use matching bindings and pass.
No engine semantics changed, so the staged fast gate selects no focused
Test262 slice; the full gate still runs the complete curated subset.

Cleanup runs `cargo clean` (5,530 files / 3.3 GiB) followed by `git checkout -- target`, restoring the
tracked rejected-screen fixture. The commit's pre-commit hook runs
`./scripts/check-touched.sh --staged --explain` on the staged change. No push,
merge, new iMac timing, or fresh noise qualification occurs. Next T017 work is
M6's fixed-hardware A/A qualification; T073 part 2 stays unadopted.
