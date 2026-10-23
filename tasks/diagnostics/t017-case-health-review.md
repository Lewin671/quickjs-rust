# T017: case-health integration review fixes

Owner: [T017](../T017-performance-benchmark-system.md). This records review
fixes to initial adoption `63b6fb82`, on `agent/t017-case-health/codex`.
The [initial adoption record](t017-case-health.md) retains its own verification.
No runtime semantics, qualified policy, frozen plan, timing statistic, threshold,
attempt accounting or load-recovery eligibility changes. T073 part 2 remains
unadopted; no new iMac job, push or merge occurs.

## Review resolutions

1. Calibration, A2 and panel collectors propagate top-level
   `readiness: incomplete` / `inventory_complete: true`. Their CLIs, screen,
   diagnostic replay and iteration return distinct status **4** on case health;
   ordinary failed gates remain 3. Schedule-calibration estimator/pilot/A2
   receipts propagate status without health retries. Speed reports are marked
   incomplete, and a failed natural speed screen has no panel time/hash.
   Statistical calibration/estimator readers raise `CaseHealthFailure` before
   accessing absent ratios. Healthy artifacts retain their original bytes.
2. `pair_recording.py` writes the initial failed-process packet before monitor,
   replay and workload checks. The recorder atomically finalizes its own still
   unsealed packet with either health or the secondary error. A monitor error,
   replay error or development workload mutation following a crash retains every
   available raw row and propagates the error; these packets cannot be admitted
   as usable health evidence.
3. Failed development replay shares healthy replay's manifest hash, timeout and
   workload-hash validation. It derives expected stdout from the frozen lane
   contract; external/held-out output contracts also come from their owners.
   A raw `expected_stdout` substitution cannot redefine correctness.
4. The formal-verifier health path retains prior missing-budget/K=6 diagnostics,
   the held-out successor-policy binding, and hashes of supplied continuation
   chains after raw replay, using the same hash constructor as a healthy return.
   It still computes no formal aggregate.
5. Iteration skips automatic variants 1..5 after a natural health failure,
   including instruction-blind/panel-required units. K=6 remains mandatory for
   any passing decision; it cannot repair this failed case. Speed qualification
   also stops before requesting its completion. Already requested inventories
   finish, and explicit diagnostic panel requests retain their original scope.

The [artifact contract](../../docs/layout-policy-artifacts.md#load-only-pair-recovery)
owns current behavior and fields.

## Tests and compatibility

Nine new review tests cover top-level collector artifacts and CLI status,
estimator/pilot/A2/speed jobs, no health retries, three secondary-error paths,
failed development identity/output tampering, typed consumer errors, formal
metadata/diagnostic conservation, and skipped automatic mandatory panels.
Existing containment tests still conserve every raw sample and requested case.
Their external synthetic fixtures now use the canonical stdout contract.

Against isolated `63b6fb82` modules, the existing healthy comparison remains
124,424 byte-identical bytes, SHA-256
`ce403ce3b2658c56484416f53330702f100cc2f0d7109f0e013b504cd84fc9fd`:
raw JSON/JSONL, result, rendered screen, selection verdict and a retained
9-case, 30-pair, six-layout held-out packet's replay/statistics. The captured
T072 failure also has an identical health diagnostic under old/new analysis.

A second A/A compares exact healthy calibration `identity.json`, `screen.json`,
`power.json` and schedule speed-report bytes under a shared deterministic mock
collection: 2,241 bytes identical (including paths), SHA-256
`16e45db0a52dca358a5b12bd6649495fd441404d57d094731f3876e3faa5da1b`.
These are analysis/serialization checks, not fresh host timing qualification.

## Binding and gates

The implementation inventory additionally binds `layout_qualification.py` and
`layout_replay.py`. Recomputing
`_implementation_sha256(ROOT, PREVIEW_IMPLEMENTATION_FILES)` gives
`f8261e9780ca619f32c018f6bee3c34934914825f4fe3eb6e0089a8aee9a4494`.
The active performance-policy JSON and frozen inventory list agree; no qualified
policy JSON or measurement-protocol hash is edited.

Full gate passes at `63b6fb82` plus these fixes:
`TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`.
It includes default/agents/perf-counters clippy and tests, **753 benchmark-tool
tests**, **35 script tests**, unit plans, context/size/hooks/shell checks and
**5,170 curated Test262 cases**. Log SHA-256:
`64a5db5138246e2690aafc628841cca327a01cca404c47ecc419aec859f1d267`.
Performance-policy and external-corpus audits also pass. The first standalone
tool-suite run found test-only mock leakage into a continuation module imported
inside a patch; preloading that owner before patching fixes isolation. The
focused pair of test modules and the final full suite both pass.

No runtime/parser/lexer semantics changed, so no focused semantic slice is
selected by the fast gate; the full curated subset still runs above. The
commit's pre-commit hook runs `./scripts/check-touched.sh --staged --explain`.
Cleanup runs `cargo clean` (5,530 files / 3.3 GiB), then `git checkout -- target`
to restore the tracked rejected-screen fixture. No new dependency is added.
Next T017 work remains M6 fixed-hardware A/A qualification.
