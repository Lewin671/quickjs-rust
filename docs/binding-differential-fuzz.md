# Seeded binding differential checks

`tools/binding_fuzz` adapts reviewer A's `gen.py`, `gen2.py`, `gen_this.py` and
`drive.py` supplied with [T077 round 3](../tasks/diagnostics/T077-s3b-lead-feedback-3.md).
It checks correctness, not timing. No held-out or benchmark inputs are used.

Build a release CLI and an optimized verifier with assertions **and overflow
checks**, plus a release binary from an isolated, revision-verified base source
archive inside the current worktree's ignored target directory. Example:

```sh
CARGO_TARGET_DIR=target/fuzz-default cargo build --release -p qjs-cli
CARGO_TARGET_DIR=target/fuzz-verify cargo build --release -p qjs-cli \
  --config profile.release.debug-assertions=true \
  --config profile.release.overflow-checks=true
python3 -m tools.binding_fuzz.run \
  --binary target/fuzz-default/release/qjs \
  --verify-binary target/fuzz-verify/release/qjs \
  --base-binary target/fuzz-base/release/qjs --base-revision <sha> \
  --ng third_party/quickjs-ng/build/qjs \
  --count 2000 --jobs 4 --output-dir target/fuzz-receipt
```

All paths must exist. The base revision is a provenance claim: verify its source
archive and build recipe, and retain those receipts alongside the run. The
runner hashes every binary, current Rust/Cargo/config input and producer before
the run and rejects input drift afterward. Output directories cannot be reused.
`request.json` freezes profiles/seed range/modes/deadline/hashes; `cases.jsonl`
retains each generated-source hash and every mode's raw and normalized result.
Nonmatching programs are retained under findings. No result replaces a failed
mode's output. Harness exceptions abort and never count as completed cases.

Each seed runs NG, candidate default, IC-off, IC-verify, combined binding/IC
stress, and base. QJS_* variables are cleared before setting the selected
mode; TZ is UTC. The verifier must actually enable debug controls. Combined
stress enables verify-bindings, forced exposure, poison and IC verification.
Individual controls/full suites remain separate stage obligations.

Receiver loops use distinct nested counters; generated index expressions follow
the nearest loop so nested var/let counters cannot invalidate whole programs.

Profiles are `binding` (scope/eval/with/NFE/default/rest/arguments/generators/
classes/jobs), `binding-simple` (the review's alternative simple-parameter
sampling distribution), and `receiver` (this/arguments/new.target and compound
operations across native/JS/constructor/loop entry). All run by default; count
is per profile. Each generator uses its own seeded RNG, so concurrency does
not affect generated source. Replay one program with the recorded seed/profile
using `program()` in run.py, or run that profile with start=<seed>, count=1.

Only the CLI's trailing Undefined completion and implementation-specific error
messages/stack frames are normalized. Printed observations, uncaught primitive
values, exit success/failure, panic, crash and timeout evidence remain visible.
Per-observation classifications distinguish:

- REGR: base matches NG, candidate differs.
- INCONS: candidate modes disagree; always investigate.
- CHANGED: base/candidate/NG differ; may be a partial repair, requires review.
- PRE: candidate equals base, both differ from NG; preserve and classify by spec.
- FIXED: candidate modes match NG, base differs.

Panic/crash/timeout/reference-internal errors and REGR/INCONS/CHANGED make the
runner fail. Exit zero means no new regression/mode failure was found; it does
**not** mean every engine agreed. PRE results remain in the receipt and require
triage before handoff. Do not change correct candidate behavior to match an NG
bug, suppress inputs, weaken the deadline, or call unexplained differences clean.
The owning task records reviewed findings and revision-bound totals.

Tests: `python3 -m unittest tools.binding_fuzz.test_run -v`.
