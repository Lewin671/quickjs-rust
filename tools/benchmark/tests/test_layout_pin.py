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
        sizes = {name: size for name, size in self.sizes.items() if name != EXECUTOR}
        with self.assertRaises(ValueError):
            layout_pin.pin([VM], 0x100000f40, sizes, self.counts, 0xfb0)

    def test_pin_prefers_an_out_of_line_run_the_order_file_never_listed(self):
        run = ("__RINvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute3run"
               "NtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_")
        sizes = dict(self.sizes, **{run: 0x3000})
        counts = dict(self.counts, **{run: 1})
        pinned = layout_pin.pin([VM, EXECUTOR], 0x100000f40, sizes, counts, 0xfb0)
        self.assertEqual(pinned[2:6], [run, EXECUTOR, CALL_LEAF, VM])

    def test_pin_places_the_interpreter_executor_first(self):
        run_vm = ("__RINvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute3run"
                  "NtNtB6_2vm2VmEB8_")
        sizes = dict(self.sizes, **{run_vm: 0x2f00})
        counts = dict(self.counts, **{run_vm: 1})
        # __text at 0xf40: 0x100 of filler puts run<Vm> at 0x040; it ends at
        # 0x2f40, and 0x70 more puts the wide executor at 0xfb0.
        head = layout_pin.pin_head([VM, EXECUTOR], 0x100000f40, sizes, counts, 0xfb0, 0x040)
        self.assertEqual(head[0], STD[2])
        self.assertEqual(head[1], run_vm)
        self.assertEqual(sorted(head[2:4]), sorted(STD[:2]))
        self.assertEqual(head[4:], [EXECUTOR, CALL_LEAF])
        pinned = layout_pin.pin([VM, EXECUTOR], 0x100000f40, sizes, counts, 0xfb0, 0x040)
        self.assertEqual(pinned, head + [VM])

    def test_budgeted_callees_keep_their_addresses_when_one_shrinks(self):
        sizes = dict(self.sizes, **{f"__RNvNtCsg55jX0GwzBC_3std2io{n}": 0x10 * (n + 1)
                                    for n in range(24)})
        counts = {name: 1 for name in sizes}
        budgets: dict[str, int] = {}
        head = layout_pin.pin_head([VM, CALL_LEAF, EXECUTOR], 0x100000000, sizes, counts,
                                   0x0, None, budgets)
        self.assertEqual(budgets[CALL_LEAF], 0x300)
        self.assertEqual(budgets[EXECUTOR], 0x3100)
        position = 0
        for name in head:
            if name == CALL_LEAF:
                break
            position += sizes[name]
        self.assertEqual(position, 0x3100)
        # A smaller call_leaf keeps its slot, so whatever follows stays put.
        smaller = dict(sizes, **{CALL_LEAF: 0x180})
        layout_pin.pin_head([VM, CALL_LEAF, EXECUTOR], 0x100000000, smaller, counts,
                            0x0, None, budgets)
        self.assertEqual(budgets[CALL_LEAF], 0x300)
        # One that outgrows it gets a new slot with headroom.
        self.assertEqual(layout_pin.slot_budget(0x310, 0x300), 0x500)

if __name__ == "__main__":
    unittest.main()
