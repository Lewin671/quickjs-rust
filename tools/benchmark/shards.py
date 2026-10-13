"""Measure one run's cases in several processes and write one raw file.

`runner.py` is a frozen protocol file, so this drives the unchanged run
instead of editing it. A forked child executes the run with every case but
its own skipped, into its own file. The parent then executes the run once
more with the measured samples handed back in place of new processes, which
writes them in the order one process would have: setup by engine and case,
then the seeded plan. How a sample is taken is the runner's, as before.
"""
from __future__ import annotations

from collections import defaultdict
from typing import Any

from .adapters import Engine
from .runner import BenchmarkRun, JsonlWriter
from .schema import Case


class ShardedRun(BenchmarkRun):
    _selected: frozenset[str] = frozenset()
    _setup: dict[tuple[str, str], list[dict[str, Any]]] | None = None
    _measured: dict[tuple[int, int], dict[str, Any]] = {}

    def measure_shard(self, case_ids: list[str], writer: JsonlWriter) -> None:
        """In a forked child: measure `case_ids` into `writer`. The parent
        owns the snapshots, so the child must exit without closing the run."""
        own, self.writer, self._selected = self.writer, writer, frozenset(case_ids)
        try:
            self._execute()
        finally:
            self.writer = own

    def merge_shards(self, records: list[dict[str, Any]]) -> bool:
        """In the parent: write the run from what the shards recorded."""
        self._setup, self._measured = defaultdict(list), {}
        # The records alone say what happened; a shard measured in this
        # process (no fork) must not leave its own state behind.
        for state in (self.iterations, self.startup_ns, self.measurement_counts, self.linearity_counts):
            state.clear()
        self.failed = False
        for record in records:
            if record["record_type"] != "sample":
                continue
            if record["phase"] == "measurement":
                self._measured[(record["block"], record["order"])] = record
            else:
                self._setup[(record["role"], record["case_id"])].append(record)
        return self.execute()

    def _write_back(self, record: dict[str, Any]) -> None:
        key = (record["role"], record["case_id"])
        if record["status"] != "ok":
            self.failed = True
        elif record["measurement_eligible"]:
            self.measurement_counts[key] += 1
        elif record["phase"] == "linearity":
            self.linearity_counts[key] += 1
        self.writer.write(record)

    def _calibrate(self, engine: Engine, case: Case) -> bool:
        if self._setup is None:
            return case.id in self._selected and super()._calibrate(engine, case)
        for record in self._setup[(engine.role, case.id)]:
            self._write_back(record)
        # Leaves the pair uncalibrated, so the run asks `_not_run` for every
        # planned sample and gets the measured one.
        return False

    def _not_run(self, engine: Engine, case: Case, block: int, order: int) -> None:
        if self._setup is not None:
            self._write_back(self._measured[(block, order)])
        elif case.id in self._selected:
            super()._not_run(engine, case, block, order)
