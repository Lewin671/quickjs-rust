from __future__ import annotations

import unittest
from collections import Counter
from pathlib import Path
from unittest.mock import Mock, patch
import subprocess

from tools.benchmark import order_file

V0 = "_RNvNtNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10compact_fn4wide10activation3run"
PROFILE = f"""Call graph:
    2000 Thread_1   DispatchQueue_1: com.apple.main-thread  (serial)
    + 1990 start  (in dyld) + 1  [0x1]
    +   1990 main  (in qjs) + 2  [0x2]
    +     1500 {V0}  (in qjs) + 10  [0x3]
    +     ! 300 std::thread::local::LocalKey$LT$T$GT$::with::h9875c3c56831960a  (in qjs) + 4  [0x4]
    +     400 {V0}  (in qjs) + 12  [0x5]
    +     90 _free  (in libsystem_malloc.dylib) + 1  [0x6]
"""


class OrderFileTests(unittest.TestCase):
    def test_exited_target_is_reaped_without_starting_sample(self):
        target = Mock(pid=123)
        target.poll.return_value = 0
        with patch.object(order_file.subprocess, "Popen", return_value=target) as spawn, \
             patch.object(order_file.time, "sleep"):
            self.assertEqual(order_file.profile_process(["qjs"], 1), Counter())
        self.assertEqual(spawn.call_count, 1)
        target.kill.assert_not_called()

    def test_target_is_reaped_before_waiting_for_sample(self):
        events = []
        target = Mock(pid=123)
        target.poll.side_effect = [None, 0]
        target.wait.side_effect = lambda: events.append("reap target")
        sampler = Mock()
        sampler.poll.return_value = 0
        sampler.wait.side_effect = lambda **kwargs: events.append("wait sampler")
        def spawn(argv, **kwargs):
            if argv[0] == "qjs":
                return target
            Path(argv[argv.index("-file") + 1]).write_text(PROFILE)
            return sampler
        with patch.object(order_file.subprocess, "Popen", side_effect=spawn), \
             patch.object(order_file.time, "sleep"):
            self.assertEqual(order_file.profile_process(["qjs"], 1)["_" + V0], 1900)
        self.assertEqual(events, ["reap target", "wait sampler"])

    def test_a_stalled_sampler_is_killed_after_the_target_is_reaped(self):
        target = Mock(pid=123)
        target.poll.side_effect = [None, 0]
        sampler = Mock()
        sampler.poll.return_value = -9
        sampler.wait.side_effect = [subprocess.TimeoutExpired("sample", 10), -9]
        with patch.object(order_file.subprocess, "Popen", side_effect=[target, sampler]), \
             patch.object(order_file.time, "sleep"):
            self.assertEqual(order_file.profile_process(["qjs"], 1), Counter())
        target.wait.assert_called_once_with()
        sampler.kill.assert_called_once_with()

    def test_profile_keys_v0_symbols_by_name_and_legacy_ones_by_hash(self):
        weights = order_file.parse_profile(PROFILE)
        self.assertEqual(weights["_" + V0], 1900)
        self.assertEqual(weights["h9875c3c56831960a"], 300)
        # Frames outside the binary and unmangled C names are not listed.
        self.assertEqual(len(weights), 2)

    def test_profile_matches_the_named_image_only(self):
        renamed = PROFILE.replace("(in qjs)", "(in qjs-copy)")
        self.assertEqual(order_file.parse_profile(renamed), Counter())
        self.assertEqual(order_file.parse_profile(renamed, "qjs-copy")["_" + V0], 1900)

    def test_rank_weights_each_case_equally(self):
        small = Counter({"a": 1})
        large = Counter({"b": 1000, "c": 10})
        self.assertEqual(order_file.rank([small, large], 1.0), ["a", "b", "c"])
        self.assertEqual(order_file.rank([small, large], 0.5), ["a"])


if __name__ == "__main__":
    unittest.main()
