import io
import json
import tempfile
import unittest
from dataclasses import replace
from fractions import Fraction
from pathlib import Path
from unittest import mock

from tools.benchmark.adapters import load_engine
from tools.benchmark.process import ProcessResult
from tools.benchmark.runner import BenchmarkRun, JsonlWriter
from tools.benchmark.schema import load_manifest
from tools.benchmark.shards import ShardedRun
from tools.benchmark.worker_pool import Task, run_tasks

ROOT = Path(__file__).resolve().parents[3]
# Fields that name one run's snapshot directory rather than what was measured.
PER_RUN = {"run_id", "snapshot_root", "argv", "binary_snapshot_path",
           "binary_version_probe_snapshot_path", "workload_snapshot_path"}


def scrub(value):
    if isinstance(value, dict):
        return {key: scrub(item) for key, item in value.items() if key not in PER_RUN}
    return [scrub(item) for item in value] if isinstance(value, list) else value


class ShardedRunTests(unittest.TestCase):
    def setUp(self):
        patcher = mock.patch("tools.benchmark.runner.probe_version", return_value="fake-version")
        patcher.start()
        self.addCleanup(patcher.stop)
        work = tempfile.TemporaryDirectory()
        self.addCleanup(work.cleanup)
        binary = Path(work.name) / "engine"
        binary.write_text("#!/bin/sh\n")
        binary.chmod(0o755)
        self.manifest = load_manifest(ROOT / "benchmarks/manifest.json")
        self.engines = [load_engine(role, "qjs-file", "quickjs-ng", binary)
                        for role in ("candidate", "base", "quickjs-ng")]
        self.cases = [replace(case, initial_iterations=1, max_iterations=1, min_window_ms=1,
                              startup_max_fraction=Fraction(1, 10), warmup_runs=0)
                      for case in self.manifest.cases[:5]]

    def _run(self, kind, stream):
        run = kind(self.manifest, self.engines, self.cases, 2, 7, JsonlWriter(stream), ROOT)
        self.addCleanup(run.close)
        return run

    def _rows(self, failing, shards):
        cases = {case.id: case for case in self.cases}

        def sample(argv, _timeout):
            case, iterations = cases[argv[-2]], int(argv[-1])
            stdout = "QJS_BENCH_RESULT " + json.dumps({
                "case_id": case.id, "iterations": iterations,
                "operations": case.expected_operations(iterations),
                "checksum": case.expected_checksum(iterations)}) + "\n"
            return ProcessResult("2026-01-01T00:00:00+00:00", 0, 1_000_000, 1_000_000,
                                 8 if case.id == failing else 0, False, stdout, "", False, False)

        output = io.StringIO()
        with mock.patch("tools.benchmark.runner.run_process", side_effect=sample):
            if not shards:
                success = self._run(BenchmarkRun, output).execute()
            else:
                run, measured, ids = self._run(ShardedRun, output), [], list(cases)
                for index in range(shards):
                    shard = io.StringIO()
                    run.measure_shard(ids[index::shards], JsonlWriter(shard))
                    measured += [json.loads(line) for line in shard.getvalue().splitlines()]
        if shards:
            # Merging starts no process: the samples are the shards'.
            with mock.patch("tools.benchmark.runner.run_process", side_effect=AssertionError):
                success = run.merge_shards(measured)
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        return success, [scrub(row) for row in rows]

    def test_shards_write_the_run_one_process_would_have(self):
        for failing in (None, self.cases[1].id):
            with self.subTest(failing=failing):
                whole = self._rows(failing, 0)
                self.assertEqual(whole[0], failing is None)
                self.assertEqual(whole[1][-1]["status"], "failed" if failing else "complete")
                for shards in (1, 2, 3):
                    self.assertEqual(self._rows(failing, shards), whole)


class WorkerPoolTests(unittest.TestCase):
    def setUp(self):
        work = tempfile.TemporaryDirectory()
        self.addCleanup(work.cleanup)
        self.work = Path(work.name)

    def _task(self, name, then=None, fail=False):
        def run():
            if fail:
                raise ValueError("no samples")
            (self.work / name).write_text(name)
        return Task(name, run, then)

    def test_tasks_and_the_tasks_they_hand_back_all_run(self):
        run_tasks([self._task("a", lambda: [self._task("c"), self._task("d")]), self._task("b")], 2)
        self.assertEqual({path.name for path in self.work.iterdir()}, set("abcd"))

    def test_a_failed_task_fails_the_pool_and_starts_nothing_more(self):
        with mock.patch("sys.stderr", new_callable=io.StringIO):
            with self.assertRaisesRegex(ValueError, "^b failed"):
                run_tasks([self._task("a"), self._task("b", lambda: [self._task("c")], fail=True),
                           self._task("d")], 1)
        self.assertEqual({path.name for path in self.work.iterdir()}, {"a"})
