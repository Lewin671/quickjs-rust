# T017: resource-lane launch admission (protocol v4)

- Status: closed (fix on `agent/resource-test-flake/claude`, awaiting integration).
- Implementation: `tools/benchmark/resource_runner.py` (`_exec_admission`) and
  `resource_validation.py` (`run_start.exec_admission`); resource measurement protocol
  `quickjs-resource-measurement-protocol-v4`.
- Verified at: base `14e151fa` (the pre-fix numbers) and the fix branch; 2026-10-20, on the local
  arm64 Mac (10 CPUs). Another session was loading the host the whole time (1-minute load 5–200).
- Evidence: this record.
- Unresolved: none.
- Next action: none.

## Symptom

The pre-push gate failed `test_real_fresh_process_rejects_and_kills_redirected_descendants` at
`all(row["duration_ns"] < 5_000_000_000 for row in samples)`. The 1-minute load average was 7–9 at the
time, with another session running a full-core test build.

## Cause

- **macOS assesses a newly written executable on its first `execve`.** Measured directly:
  - a new script: 310–330 ms on first exec, then 6–16 ms;
  - a new copy of qjs (5,122 KiB): 339–480 ms, then 4 ms;
  - running the file through `sh file` does not trigger the assessment, and rewriting an already
    launched file does not trigger it again.
- **Resource runs execute fresh snapshots.** Each run copies every engine into a new snapshot file, and
  the fresh-process lane had no warmup, so the cost landed in each engine's first sample.
- **Cost depends on concurrent first launches, not CPU load.** It grows when other processes are
  launching new executables at the same time, as cargo test binaries do. It grows only slightly with
  CPU load.
- **What it is not.** The tool did not wait for the descendant: the timer stops at the direct child's
  reap, and `sleep` holds no pipe. No timeout of the tool was involved.

## Reproduction

The test is the real one, run repeatedly. "Exec storm" means N shell loops that each keep writing and
executing a new script; "CPU" means 20 `yes` processes.

| Code | Condition | Runs | Failures | First sample (ms) | Max of other samples (ms) |
| --- | --- | ---: | ---: | --- | ---: |
| `14e151fa` (pre-fix) | none | 15 | 0 | 289–379 | 54 |
| `14e151fa` | CPU | 10 | 0 | 295–639 | 26 |
| `14e151fa` | 8-loop exec storm | 10 | 0 | 2,347–3,091 | 29 |
| `14e151fa` | 16-loop exec storm | 10 | **10** | 6,311–9,637 | 35 |
| fix (v4) | none | 25 | 0 | max sample 26; admission 292–611 | – |
| fix (v4) | CPU | 25 | 0 | max sample 35; admission 292–684 | – |
| fix (v4) | 16-loop exec storm | 30 | 0 | max sample 28; admission 4,400–6,731 | – |

An earlier session at load about 200 also saw a 3,731 ms first sample with no added storm. With the
storm, the first exec can exceed the lane's 10 s case timeout, which before the fix made the first
sample a timeout and failed the run.

## Fix and contract (protocol v4)

- **Admission.** Before the plan, every lane that executes engines (fresh, RSS) launches each snapshot
  once with `-e 0`, plus `--raw` for the raw adapter. Each launch is untimed and has a 120 s timeout,
  the throughput tools' launch-preflight bound.
- **Failure stops the run.** A nonzero exit, a timeout or truncated output stops the run before any
  record is written. A descendant does not fail the admission; the sample rules judge descendants.
- **Record.** `run_start.exec_admission` holds argv, the executed binary's SHA-256, duration, exit code
  and `descendants_detected` for each role. The validator requires it on admitting lanes and `null` on
  the binary-size lane.
- **Version bump.** The protocol id moves to v4, following the `598478a6` precedent: the semantics
  changed, and `START_FIELDS` is matched exactly. v3 fresh-latency data still contains the first-exec
  cost and is not directly comparable with v4. No frozen plan or running job uses this protocol.
