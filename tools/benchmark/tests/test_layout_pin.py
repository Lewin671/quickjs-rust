from __future__ import annotations

import unittest

from tools.benchmark import layout_pin

EXECUTOR = ("__RINvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute18"
            "try_run_typed_loopNtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_")
CALL_LEAF = "__RNvNtNtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode10typed_loop7execute9call_leaf"
VM = "__RNvMs5_NtNtCs9nYd1Hk1rek_11qjs_runtime8bytecode2vmNtB5_2Vm22run_current_activation"
STD = [f"__RNvNtCsg55jX0GwzBC_3std2io1{n}" for n in ("a", "b", "c")]
GENERIC = "__RINvNtCsg55jX0GwzBC_3std2rt15handle_rt_paniciE"


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

    def test_filler_accepts_other_std_hashes_but_keeps_all_exclusions(self):
        alternate = "__RNvNtCsAnotherHash_3std2io1a"
        unhashed = "__RNvNtC3std2io1b"
        # A real identifier can contain text resembling a crate marker.
        fake_marker = "CsAnotherHash_3std"
        non_std = f"__RNvCsOtherHash_5other{len(fake_marker)}{fake_marker}"
        stdio = "__RNvNtCsOtherHash_5stdio2io1a"
        duplicate = "__RNvNtCsAnotherHash_3std2io1c"
        misaligned = "__RNvNtCsAnotherHash_3std2io1d"
        excluded = "__RNvNtCsAnotherHash_3std2io1e"
        workspace_instance = ("__RNvNtCsAnotherHash_3std2rt4work"
                              "CsWorkspaceHash_11qjs_runtime")
        sizes = {alternate: 0x30, unhashed: 0x40, non_std: 0x70, stdio: 0x70,
                 duplicate: 0x70, misaligned: 0x71, excluded: 0x70,
                 workspace_instance: 0x70, GENERIC: 0x70}
        counts = {name: 1 for name in sizes}
        counts[duplicate] = 2
        self.assertEqual(set(layout_pin.filler(sizes, counts, 0x70, {excluded})),
                         {alternate, unhashed})
        for rejected in (non_std, stdio, duplicate, misaligned, excluded,
                         workspace_instance, GENERIC):
            with self.subTest(symbol=rejected), self.assertRaises(ValueError):
                layout_pin.filler({rejected: sizes[rejected]}, counts, 0x70, {excluded})

    def test_definition_crate_follows_impl_paths_without_searching_identifiers(self):
        module = "NtCsAlternate_3std2io"
        error = "NtNtCsAlternate_3std2io5Error"
        display = "NtNtCsCoreHash_4core3fmt7Display"
        cursor = "INtNtCsAlternate_3std2io6CursorjE"
        for parent in (module, f"Ms7_{module}{error}",
                       f"Xs7_{module}{error}{display}", f"Y{cursor}{display}"):
            self.assertEqual(layout_pin.definition_crate(f"__RNv{parent}1a"), "std")
        for malformed in ("__RNv", "__RNvC", "__RNvCsMissingTerminator3std",
                          "__RNvC99std", "__RNvN1C3std", "__RNvB0_"):
            self.assertIsNone(layout_pin.definition_crate(malformed))

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
        sizes = dict(self.sizes, **{f"__RNvNtCsg55jX0GwzBC_3std2io{len(f'f{n}')}f{n}": 0x10 * (n + 1)
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
        # The callee has an independent baseline alignment, even if the
        # executor slot grows or the selected executor window changes.
        self.assertEqual(position, 0x3580)
        self.assertEqual(position % layout_pin.PAGE, 0x580)
        # A smaller call_leaf keeps its slot, so whatever follows stays put.
        smaller = dict(sizes, **{CALL_LEAF: 0x180})
        layout_pin.pin_head([VM, CALL_LEAF, EXECUTOR], 0x100000000, smaller, counts,
                            0x0, None, budgets)
        self.assertEqual(budgets[CALL_LEAF], 0x300)
        smaller_head = layout_pin.pin_head([VM, CALL_LEAF, EXECUTOR], 0x100000000,
                                          smaller, counts, 0x0, None, budgets)
        new_position = sum(smaller[name] for name in
                           smaller_head[:smaller_head.index(CALL_LEAF)])
        self.assertEqual(new_position, position)
        # One that outgrows it gets a new slot with headroom.
        self.assertEqual(layout_pin.slot_budget(0x310, 0x300), 0x500)

    def test_tail_callees_accept_alternate_hashes_and_keep_exact_value_types(self):
        clone = ("__RNvXs1_NtCs9nYd1Hk1rek_11qjs_runtime5valueNtB5_5Value"
                 "NtNtCsl8K0bEFm1U0_4core5clone5Clone5clone")
        drop = ("__RINvNtCsl8K0bEFm1U0_4core3ptr13drop_in_place"
                "NtNtCs9nYd1Hk1rek_11qjs_runtime5value5ValueEBK_")
        # Preserve encoded lengths so the compiler-style backrefs remain valid.
        clone = clone.replace("l8K0bEFm1U0", "a" * len("l8K0bEFm1U0"))
        drop = drop.replace("9nYd1Hk1rek", "b" * len("9nYd1Hk1rek"))
        unrelated = [clone.replace("5Value", "5Other"),
                     drop.replace("11qjs_runtime", "11not_runtime"),
                     drop.replace("5Value", "5Other")]
        sizes = dict(self.sizes, **{name: 0x10 for name in [clone, drop, *unrelated]})
        counts = {name: 1 for name in sizes}
        head = layout_pin.pin_head([EXECUTOR], 0x100000000, sizes, counts, 0, None)
        self.assertEqual(set(head[-2:]), {clone, drop})
        self.assertTrue(set(unrelated).isdisjoint(head))

if __name__ == "__main__":
    unittest.main()
