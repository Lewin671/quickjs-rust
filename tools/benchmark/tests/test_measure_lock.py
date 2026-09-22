from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tools.benchmark import measure_lock


class LockTests(unittest.TestCase):
    def test_lock_is_released_after_use(self):
        with tempfile.TemporaryDirectory() as work:
            lock = Path(work) / "lock"
            with measure_lock.measurement_lock("t", lock):
                owner = json.loads((lock / "owner.json").read_text())
                self.assertEqual(owner["pid"], os.getpid())
            self.assertFalse(lock.exists())

    def test_stale_lock_from_a_dead_owner_is_broken(self):
        with tempfile.TemporaryDirectory() as work:
            lock = Path(work) / "lock"
            dead = subprocess.Popen([sys.executable, "-c", "pass"])
            dead.wait()
            lock.mkdir()
            (lock / "owner.json").write_text(json.dumps({"pid": dead.pid, "label": "old"}))
            with measure_lock.measurement_lock("t", lock, sleep=self.fail):
                pass
            self.assertFalse(lock.exists())

    def test_live_owner_is_waited_for(self):
        with tempfile.TemporaryDirectory() as work:
            lock = Path(work) / "lock"
            owner = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
            try:
                lock.mkdir()
                (lock / "owner.json").write_text(json.dumps({"pid": owner.pid, "label": "other"}))
                waits = []

                def sleep(seconds):
                    waits.append(seconds)
                    if len(waits) == 2:
                        owner.kill()
                        owner.wait()

                with measure_lock.measurement_lock("t", lock, sleep=sleep):
                    pass
                self.assertGreaterEqual(len(waits), 2)
            finally:
                owner.kill()
                owner.wait()


class BusyProcessTests(unittest.TestCase):
    def test_parse_ps_names_competing_commands_only(self):
        output = ("  101 /Users/x/.cargo/bin/cargo\n  102 /usr/bin/python3\n"
                  "  103 /repo/target/release/qjs\n  104 rustc\n  999 /repo/target/release/qjs\n")
        self.assertEqual(measure_lock.parse_ps(output, own_pid=999),
                         ["cargo(101)", "qjs(103)", "rustc(104)"])

    def test_wait_for_builds_gives_up_after_its_budget(self):
        waits = []
        left = measure_lock.wait_for_builds(4, sleep=waits.append, probe=lambda: ["cargo(1)"])
        self.assertEqual((left, waits), (["cargo(1)"], [2.0, 2.0]))

    def test_wait_for_builds_returns_once_quiet(self):
        probes = iter([["cargo(1)"], []])
        self.assertEqual(measure_lock.wait_for_builds(60, sleep=lambda _s: None,
                                                      probe=lambda: next(probes)), [])


if __name__ == "__main__":
    unittest.main()
