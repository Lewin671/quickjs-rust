# T017: benchmark-tool tests that failed on a loaded host

- Status: closed (fixes on `agent/resource-test-flake/claude`, awaiting integration).
- Implementation: `tools/benchmark/tests/bounded.py` (`run_bounded`), 37 converted call sites, the
  sentinel drift test, default fake admission in `test_resource_system`, two runner-test case
  timeouts, and `GIT_OPTIONAL_LOCKS=0` in `scripts/check.sh` and `scripts/check-touched.sh`.
- Verified at: base `a52cec77`; 2026-10-20, on the local arm64 Mac (10 CPUs) while another session
  loaded the host (1-minute load average 40–365).
- Evidence: this record.
- Unresolved: the tool-side `git status` probes listed below can still leave `index.lock` during real
  runs.
- Next action: none here.

## Failure being explained

During a fast gate at a load average of about 58,
`test_numeric_measurements_are_bounded_before_float_conversion` raised `TimeoutExpired`: its child
`scripts/resource-benchmark-report.sh` exceeded the **test's own** `timeout=10`. The tool sets no
timeout of its own.

## Where the time goes

| Measurement | Result |
| --- | --- |
| The CLI on the test's input (Python start, imports, one validation that fails at row 1) | 0.10 s wall, 0.07 s user |
| The same with no bytecode cache | 0.33 s |
| The test as a whole | 1.7 s |
| Of which: `git status --porcelain` in `ResourceRun._runner_repo` (it recurses into the checked-out `third_party` submodules) | 0.77 s per run (0.03 s with `--ignore-submodules=all`) |
| Of which: the real first launch of a new snapshot (`exec_admission`) | ~0.3 s or more per run |
| Same test after the fix | 0.9 s (fake admission) |

The CLI does no avoidable work.

## Which step timed out, and why

- **The step.** The CLI subprocess, under a wall-clock bound.
- **Load reproductions all stayed far below 10 s.** 40–30 runs of the CLI each:
  - 20 `yes`: max 0.35 s;
  - 20 `yes` plus four parallel test suites: max 0.46 s;
  - 16 loops launching new executables: max 0.41 s;
  - four `dd`+`sync` writers: max 0.48 s.
- **What the host was doing instead.** At the failure the host had 4.7 GB of 6 GB swap in use. In the
  same minute a tool `git status` (5 s timeout) was killed and left
  `.git/worktrees/resource-test-flake/index.lock` behind, which blocked the next commit. The lock
  reappeared at 04:50 during the four-suite load run.
- **Conclusion.** The failure was a host-wide stall (memory pressure is the likely cause; not
  reproduced), not work the CLI does. A wall-clock bound measures the host, not the child.

## Other load-sensitive behavior found

- **Shared-checkout mutation.** `test_a_diverging_sentinel_reference_fails_the_cross_check` rewrote the
  tracked `benchmarks/generic-sentinels-manifest.json` in place. Pre-fix, 20 `yes` plus four parallel
  suites on one checkout failed 2/4 suites: `test_output_is_atomic_and_never_overwrites` and
  `test_gate_requirements_and_argparse_fail_structurally` read the drifted file. The 4/4 run also left
  `index.lock`.
- **1 s case timeouts.** Two runner tests gave freshly written engine scripts a 1 s case timeout. A first
  launch takes 2.3–9.6 s while other new executables launch
  ([launch admission record](T017-resource-launch-admission.md)).

## Fix

- **CPU-time bounds.** `run_bounded` replaces every wall-clock `timeout=` on a CLI subprocess in
  `tools/benchmark/tests`: 37 sites in 11 files. It runs the child under `ulimit -t 60`, a CPU-time limit
  inherited across exec, with a 900 s wall backstop that only a child blocked forever reaches.
- **Drift without rewriting files.** The sentinel drift is injected by patching the manifest loader.
  The test no longer writes tracked files.
- **Fake admission by default.** `test_resource_system._run` fakes the launch admission unless a test
  passes one; the descendant test passes the real `run_fresh_process`.
- **Runner-test case timeouts.** The two 1 s timeouts, which those tests do not test, are now 60 s.
- **No stale index locks.** The gates run the tool tests with `GIT_OPTIONAL_LOCKS=0`, so a timed-out
  `git status` cannot leave `index.lock`. No protocol file changed.

## Result

| Condition | Before | After |
| --- | --- | --- |
| 20 `yes` plus four parallel full suites | 2/4 suites fail; `index.lock` left | 4/4 pass at load up to 365; no lock |
| Suite wall time under that load | 538 s | 326–332 s |
| One full suite at load 139 | – | 662 tests OK |

## Remaining (not fixed here)

- **Tool-side `git status` probes.** `runner.py`, `resource_runner.py`, `local_receipt.py`, `preview.py`
  and `layout_iteration.py` use 5–10 s timeouts. A killed probe still leaves `index.lock` in real runs.
  The fix is `git --no-optional-locks`, but `runner.py` and `resource_runner.py` are protocol files.
  `--ignore-submodules=dirty` would also cut 0.7 s per run.
- **Race tests.** `test_runner.py` `test_normal_parent_exit_contains_pipe_inheriting_child` (child
  sleeps 2 s, test checks at 2.2 s) and `test_timeout_kills_spawned_child_process_group` (0.3/0.4 s)
  are timing races by construction. The first fails if containment takes over 2 s on a stalled host;
  the second can pass vacuously.
- **Child-death deadline.** `test_resource_system.py`'s descendant test allows 2 s for killed children
  to disappear.
