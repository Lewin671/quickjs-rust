# T076 completion for lead review

Requested execution is complete. All three candidates were implemented,
correctness-tested, committed and independently compared to the preceding
surviving bundle on the qualified iMac. Every unit misses the prospectively
frozen instruction filter, so every runtime change is removed. This is filter
application, not a promotion decision or a ban on any mechanism family.

## Result

| Unit | Candidate / preserved branch | Own instruction geomean | Control breaches | Disposition |
| --- | --- | ---: | --- | --- |
| U1 publication | `e5dcaef0`, `agent/t076-u1/codex` | 1.008294424 | 14; worst 3d-raytrace 1.028745575 | dropped |
| U2 scalar indices | `12aa95b8`, `agent/t076-u2/codex` | 0.997338770 | none | dropped; insufficient target gain |
| U3 received indices | `08bb19a7`, `agent/t076-u3/codex` | 0.987714706 | audio-beat-detection 1.010866433 | dropped |

Filter: own targets <=0.995 geomean, every other selection case <=1.003.
[Measurements](measurements.md) retain the mechanisms, tests, codegen/IR
observations, every control ratio and the U3 audio control's five raw pairs.
[Preparation](preparation.md) owns the fresh queue, exact-image profiles,
instruction A/A qualification and failed preliminary analyses. Every raw
counter median is independently replayed against hashed rusage files.

There are zero survivors. The bundle has no distinct candidate to triage;
no screen/formal/held-out/RSS/resource run is justified. Screen budget: 0/2
spent, not reset. No final decide/promotion command, origin push or merge.
The restored runtime is exactly base `19125d48c626869d776df9a501c44f3a00566e32`;
T071's compiler_class.rs/compiler_arguments.rs/vm_class.rs are untouched.
No dependency, public API, compiler policy, Value/opcode width or binding
architecture change remains. Each candidate and its tests remain reviewable
on its side branch and in this branch's history. These are frozen candidate
snapshots; current outcomes live in this completion record.

## iMac job ledger

Every iMac build/collection uses the serialized workspace wrapper. Non-timing
jobs are explicitly unmonitored for wall disturbance; no wall gain is inferred
from them. The sole timing job starts local 05:00:06, outside 02:45-05:00,
and has an empty intruder log. Other owners' jobs are never cancelled.

| Full job ID | Result |
| --- | --- |
| `20261023T192551Z-t076-baseline-instructions-profiles` | non-timing, exit 0 / 244 s; 86 cases, 351 counter processes, 21 profiles |
| `20261023T193828Z-t076-instruction-aa` | non-timing, exit 0 / 804 s; 1,170 processes, all 86 controls within 1.003 |
| `20261023T202815Z-t076-disk-preflight` | non-timing, exit 0 / 0 s; 52,550,344,704 free bytes |
| `20261023T210005Z-t076-base-queue` | timing, exit 0 / 2,074 s; healthy 30-block workers-4, 86 cases, no issues/imprecision/DISTURBED |
| `20261023T213557Z-t076-bind-base-queue` | non-timing, exit 0 / 40 s; qualified-host report replay and queue export |
| `20261023T215026Z-t076-u1-reference-instructions` | non-timing, exit 0 / 897 s; 257 NG fixtures, 1,170 counters; U1 misses |
| `20261023T221604Z-t076-u2-reference-instructions` | non-timing, exit 0 / 901 s; 257 NG fixtures, 1,170 counters; U2 misses |
| `20261023T224347Z-t076-u3-reference-instructions` | non-timing, exit 0 / 892 s; 257 NG fixtures, 1,170 counters; U3 misses |
| `20261023T224727Z-t076-u3-conditional-conformance` | non-timing, exit 0 / 0 s; independently replays U3's miss, records SKIP; no exact scan |

Total instruction processes: 5,031, all checksum-checked. Data remains bound
to exact images/source/workloads in the retained count reports. Raw images,
assembly, samples and rusage assets live in the named queue jobs and fetched
workspace outputs; those external paths alone are not the recorded result.

## Verification and limits

- Plan freeze `c0652810` precedes all runtime edits. Schema-3 `validate-unit`
  passes with all 21 content-verified receipts. Plan SHA-256 remains
  `de7c229d450e75c7fdd71b91a36180dbabe76ade8650fb42859dcae54c64c73a`;
  queue SHA-256 `fa992f84c3974e9a92dafc89be7dc4c5246c0eb0bdab2f7e6108c08d5b1cfd25`.
- Each exact candidate's identical staged runtime tree passes the full gate,
  focused tests, 65 selected Test262 cases and 257 UTC NG fixtures. Full gates
  include all 5,170 curated cases; per-unit logs/counts are recorded in measurements.
- Final restored-tree full/fast checks and cleanup are recorded below. No complete exact Test262 scan or new conformance standing is claimed:
  the conditional scan's prerequisite fails, and no complete scan means no ledger entry.
- U3's audio control varies in the base itself. Cause remains unproven; no
  threshold waiver, favorable retry, wall diagnosis or code repair follows it.
- Local arm64 timing-report replay rejects the fetched broad report; qualified
  iMac replay succeeds. The cause remains outside this task; no artifact is edited.
- `tasks/T045-typed-loop-boxed-demand-provenance.md` is stale outside this
  boundary: it still says its old screen is pending, although its retained
  iMac job finished FAIL (details in preparation). No T045 attempt is reopened.

Next useful work: lead review of these preserved candidates and the audio
instruction-control variability. A future repair/retry needs a new authorized
contract; these failed unit gates are not retuned or relabelled.

## Final restored-tree gate

`TEST262_DIR=<shared pinned corpus> ./scripts/check.sh` passes on the restored
engine: 2,382 runtime tests, agents/perf-counters/clippy, 753 benchmark-tool
tests, all frozen plans, context, 35 script tests, hook isolation, shell syntax,
file-size guard and all 5,170 curated Test262 cases. Full-log SHA-256:
`c2a10ee9c8d79d6aefdb443a2272f4746854158af92f757ac2c96ad597c30f72`.

`git diff --quiet 19125d48 -- crates scripts tools benchmarks Cargo.toml
Cargo.lock rust-toolchain.toml` returns 0. Runtime, producer, benchmark,
configuration and dependency source match the frozen base exactly. Python
collector syntax, retained raw-count replay, `git diff --check`, and final
context validation also pass. Final fast-gate/cleanup verification follows below.

## Cleanup and final commit verification

Restore/drop commit `eb6afe9e` passes its normal pre-commit fast gate, including
65 selected Test262 cases and context. Commit/gate log SHA-256:
`4431bf890ffa7c48019099288d39d67d52f45b1f1bb80921fa641260cf896b28`.
The full gate above verified its identical restored runtime tree.

Required `cargo clean` completes (16,077 files / 9.6 GiB), followed by
`git checkout -- target` to restore the tracked fixture. Local disk is 54 GiB
free (>=8 GiB). Working tree is clean after cleanup, and the production-source
comparison to `19125d48` still returns 0. The plan remains byte-identical.
All nine own iMac jobs are finished; no other owner's job was cancelled.

Final changes relative to base are only this task's documentation, frozen
plan/index and diagnostic collectors/data. This cleanup note's commit runs
the docs-only fast/context gate; it adds no runtime/build change or standing.
No origin push, merge, remote CI, final decide or promotion occurs.
