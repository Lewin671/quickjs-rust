# T017: index-lock-safe git probes and event-driven process tests

- Status: closed (on `agent/resource-test-flake/claude`, awaiting integration).
- Implementation: the follow-up to [loaded-host test failures](T017-test-wall-timeouts.md), covering the
  items it left open.
- Verified at: 2026-10-20, local arm64 Mac; the tool test suite under 20 `yes` (results below).
- Evidence: this record.
- Unresolved: `tools/benchmark/runner.py` keeps its plain `git status` (known issue below).
- Next action: none.

## Changes

- **`run_bounded`.** The wall backstop is 300 s, inside the 10–35 minute CI job limits. `stdin` defaults
  to `/dev/null` unless the call passes `input`.
- **`--no-optional-locks`.** Added to the `git status` probes of:
  - `resource_runner.py`. This is a resource measurement protocol file. The change rides on the
    unpushed v4 with no new version; the v4 aggregate and both resource hashes in
    `performance-policy.json` are updated.
  - `local_receipt.py`.
  - `preview.py`, a hosted implementation file; the hosted implementation aggregate is updated.
- **`layout_iteration.py` is unchanged.** Its `git status` has no timeout, so nothing kills it midway
  and a stale lock needs an external kill. It is also a hosted implementation file.
- **`scripts/perf-compare.sh` and `scripts/perf-loop.sh`** export `GIT_OPTIONAL_LOCKS=0`. Neither
  script is in any protocol list.
- **Event-driven process tests.** Fixed sleeps are replaced by synchronization and polling with an
  upper bound:
  - `test_timeout_kills_spawned_child_process_group` and
    `test_normal_parent_exit_contains_pipe_inheriting_child`: the child records its PID before sleeping
    30 s, and the parent proceeds only after that record exists. The test then polls until the child is
    gone (bound 20 s) and checks that the marker was never written. The first test also asserts that
    the child existed before the timeout, so it can no longer pass vacuously.
  - The resource descendant test polls for its killed `sleep 30` descendants with a 20 s bound instead
    of 2 s. It still ends as soon as they are gone; only a missed descendant is alive at 20 s.

## Known issue

`tools/benchmark/runner.py` still runs `git status --porcelain` with a 5 s timeout and without
`--no-optional-locks`. It is in the throughput v10 and sentinel v3 protocol lists, so changing it would
break the v6 layout policy's `host.manifests` binding and every v6 decision. Until a protocol bump is
due anyway, these callers export `GIT_OPTIONAL_LOCKS=0`:
- `perf-compare.sh` and `perf-loop.sh`;
- the test gates;
- the workspace `imac-bench.sh`.
