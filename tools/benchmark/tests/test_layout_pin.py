from __future__ import annotations

import unittest

from tools.benchmark import layout_pin

EXECUTOR = ("__RINvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute18"
            "try_run_typed_loopNtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_")
CALL_LEAF = "__RNvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute9call_leaf"
VM = "__RNvMs5_NtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode2vmNtB5_2Vm22run_current_activation"
STD = [f"__RNvNtCsg55jX0GwzBC_3std2io{n}" for n in ("a", "b", "c")]
GENERIC = "__RINvNtCsg55jX0GwzBC_3std2rt15handle_rt_paniciEB4_"


class LayoutPinTests(unittest.TestCase):
    sizes = {STD[0]: 0x40, STD[1]: 0x30, STD[2]: 0x100, GENERIC: 0x10, EXECUTOR: 0x3000,
             CALL_LEAF: 0x200, VM: 0x5000}
    counts = {name: 1 for name in sizes}

    def test_filler_sums_exactly_and_uses_concrete_std_functions_only(self):
        picked = layout_pin.filler(self.sizes, self.counts, 0x70, set())
        self.assertEqual(sorted(picked), sorted(STD[:2]))
        self.assertNotIn(GENERIC, layout_pin.filler(self.sizes, self.counts, 0x40, set()))
        with self.assertRaises(ValueError):
            layout_pin.filler(self.sizes, self.counts, 0x20, set())

    def test_pin_places_filler_then_executor_then_callees(self):
        lines = [VM, CALL_LEAF, EXECUTOR]
        # __text starting 0x890 below the target offset needs 0x70 of filler.
        pinned = layout_pin.pin(lines, 0x100000f40, self.sizes, self.counts, 0xfb0)
        self.assertEqual(sorted(pinned[:2]), sorted(STD[:2]))
        self.assertEqual(pinned[2:], [EXECUTOR, CALL_LEAF, VM])

    def test_pin_requires_the_executor(self):
        with self.assertRaises(ValueError):
            layout_pin.pin([VM], 0x100000f40, self.sizes, self.counts, 0xfb0)


if __name__ == "__main__":
    unittest.main()
