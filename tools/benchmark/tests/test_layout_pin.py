"""Tests for the layout targets, selection, pinning and audit.

The fixtures give a small engine with the same shape as the real one: the
legacy build's closure spellings for the two executor instantiations, its
callees (newer targets are reported absent), and an order file in an
older build's spelling. Every test here is a property the tool has to hold for
the layout to mean anything -- raw staleness, ambiguity, and which names the
head is allowed to write.

Two contracts these tests hold the implementation to:

* staleness is judged on the *raw* spelling, because that is what the linker
  resolves; a canonical match is a replacement to write, never proof the entry
  is live.
* two live definitions of one body at different addresses are ambiguous, and
  coalesced spellings at one address are unpinnable: neither is ever chosen
  silently.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import layout_pin
from tools.benchmark.tests import layout_fixtures as fx

EXECUTOR = layout_pin.WIDE_EXECUTOR
INTERPRETER = layout_pin.INTERPRETER_EXECUTOR
CALL_LEAF = layout_pin.CALLEES[3]
TAIL_DROP = layout_pin.TAIL_CALLEES[1]
# Two legacy callees absent from the fixture: one is a historical
# target a rewrite may drop, the other is simply gone.
BOXED_EQUALITY = f"{layout_pin.CALLEES[11]}"
GET_NAMED = layout_pin.CALLEES[12]


def slot_position(head, symbols, name):
    """The position `name` lands at, as the linker order would place it."""
    position = symbols.start
    for entry in head:
        if entry == name:
            return position
        position += symbols.sizes.get(entry, 0)
    raise AssertionError(f"{name} is not in the head")


def finding_for(symbols, role, target=None, listed=()):
    for finding in layout_pin.findings_for(symbols, listed):
        if finding.role == role and (target is None or finding.target == target):
            return finding
    raise AssertionError(f"{role} {target}")


class FillerTests(unittest.TestCase):
    # Three independent 0x40 candidates, so excluding any one of them still
    # leaves a subset that sums to 0x80, and 0x20 is below every size.
    sizes = {"__RNvNtCsg55jX0GwzBC_3std2io2f0": 0x40,
             "__RNvNtCsg55jX0GwzBC_3std2io2f1": 0x40,
             "__RNvNtCsg55jX0GwzBC_3std2io2f2": 0x40}
    counts = {name: 1 for name in sizes}
    generic = "__RINvNtCsg55jX0GwzBC_3std2rt15handle_rt_paniciE"

    def test_sums_exactly_and_uses_aligned_concrete_std_functions_only(self):
        picked = layout_pin.filler(self.sizes, self.counts, 0x80, set())
        self.assertEqual(sum(self.sizes[name] for name in picked), 0x80)
        self.assertEqual(len(picked), 2)
        # 0x20 is below every candidate's size, so nothing can sum to it; a
        # generic standard-library function is not a fixed-size candidate; and
        # neither is a name the binary spells more than once.
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler(self.sizes, self.counts, 0x20, set())
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler(dict(self.sizes, **{self.generic: 0x40}), self.counts,
                              0x40, set(list(self.sizes)[:3] + [self.generic]))
        doubled = dict(self.counts, **{list(self.sizes)[0]: 2})
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler(self.sizes, doubled, 0x40, set(list(self.sizes)[1:]))

    def test_a_pool_that_cannot_sum_fails_rather_than_guessing(self):
        unaligned = {"__RNvNtCsg55jX0GwzBC_3std2io2f0": 0x30}
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler(unaligned, {list(unaligned)[0]: 1}, 0x80, set())

    def test_an_excluded_name_is_absent_from_a_selection_that_still_succeeds(self):
        # Excluding one candidate changes which names are chosen, and never
        # removes a selection that some other subset can still make.
        picked = layout_pin.filler(self.sizes, self.counts, 0x80, set())
        excluded = picked[0]
        alternative = layout_pin.filler(self.sizes, self.counts, 0x80, {excluded})
        self.assertNotIn(excluded, alternative)
        self.assertEqual(sum(self.sizes[name] for name in alternative), 0x80)
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler(self.sizes, self.counts, 0x80, set(self.sizes))

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

    def test_a_workspace_instance_is_not_a_filler_candidate(self):
        workspace = "__RNvNtCsg55jX0GwzBC_3std2rt4workCsWorkspaceHash_11qjs_runtime"
        with self.assertRaises(layout_pin.LayoutError):
            layout_pin.filler({workspace: 0x70}, {workspace: 1}, 0x70, set())

    def test_slot_budget_keeps_a_recorded_slot_and_grows_a_smaller_one(self):
        self.assertEqual(layout_pin.slot_budget(0x200, 0x300), 0x300)
        self.assertEqual(layout_pin.slot_budget(0x310, 0x300), 0x500)
        self.assertEqual(layout_pin.slot_budget(0x200, None), 0x300)


class TextSymbolTests(unittest.TestCase):
    def test_reads_the_section_and_ignores_non_text_rows(self):
        symbols = fx.read_symbols()
        self.assertEqual(symbols.start, fx.START)
        self.assertEqual(symbols.size_of(fx.WIDE_CLOSURE), 0x2100)
        self.assertEqual(symbols.size_of(fx.CALL_LEAF), 0x300)
        self.assertIsNone(symbols.size_of("_some_bss"))
        self.assertIsNone(symbols.size_of("__NSGetArgc"))

    def test_the_mach_header_is_outside_the_section_and_not_a_candidate(self):
        symbols = fx.read_symbols()
        # The header symbol sits before `__text`, so it is not one of the
        # section's functions and may not be laid out by its distance to one.
        self.assertNotIn(fx.HEADER_NAME, symbols.names)
        self.assertIsNone(symbols.size_of(fx.HEADER_NAME))
        self.assertNotIn(fx.HEADER_NAME, symbols.counts)

    def test_the_last_function_has_the_section_size_as_its_extent(self):
        symbols = fx.read_symbols()
        last_name, last_size = fx.ENGINE[-1]
        self.assertEqual(last_name, fx.MAIN_NAME)
        self.assertEqual(symbols.size_of(last_name), last_size)
        self.assertEqual(symbols.address_of(last_name),
                         fx.START + fx.section_size(fx.ENGINE) - last_size)

    def test_spellings_that_share_an_address_are_all_unpinnable(self):
        # `nm` alone cannot say which of two names at one address is the real
        # atom, so none of them may be sized, pinned or used as filler.
        entries = ((fx.CALL_LEAF_OLD, 0), (fx.CALL_LEAF, 0x300), (fx.MAIN, 0x200))
        symbols = fx.read_symbols(entries)
        for name in (fx.CALL_LEAF_OLD, fx.CALL_LEAF):
            with self.subTest(name=name):
                self.assertIsNone(symbols.size_of(name))
                self.assertNotIn(name, symbols.sizes)
        finding = finding_for(symbols, "callee", CALL_LEAF)
        self.assertFalse(finding.in_binary)
        self.assertEqual({entry.text for entry in finding.unpinnable},
                         {fx.CALL_LEAF_OLD, fx.CALL_LEAF})


class DefinitionTests(unittest.TestCase):
    def test_two_spellings_at_two_addresses_are_two_bodies(self):
        entries = fx.ENGINE + ((fx.WIDE_PLAIN_OLD, 0x800),)
        symbols = fx.read_symbols(entries)
        matched = layout_pin.SymbolIndex(symbols.names).match(EXECUTOR)
        self.assertEqual({entry.text for entry in matched},
                         {fx.WIDE_CLOSURE, fx.WIDE_PLAIN_OLD})
        selected, unselected = layout_pin.definitions(matched, symbols)
        self.assertEqual(len(selected), 2)
        self.assertEqual(unselected, ())

    def test_coalesced_spellings_are_one_unpinnable_address(self):
        entries = ((fx.CLONE, 0), (fx.CLONE + ".3946", 0x80), (fx.MAIN, 0x200))
        symbols = fx.read_symbols(entries)
        matched = layout_pin.SymbolIndex(symbols.names).match(layout_pin.TAIL_CALLEES[0])
        self.assertEqual(len(matched), 2)
        selected, unselected = layout_pin.definitions(matched, symbols)
        self.assertEqual(selected, ())
        self.assertEqual({entry.text for entry in unselected},
                         {fx.CLONE, fx.CLONE + ".3946"})


class FindingTests(unittest.TestCase):
    def test_direct_dispatch_is_preferred_over_entry_and_deopt_wrappers(self):
        direct = ("__RINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute7execute"
                  "NtNtNtNtB6_10compact_fn4wide10loop_frame13WideLoopFrameEB8_")
        deopt = ("__RNCINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute7execute"
                 "NtNtNtNtB8_10compact_fn4wide10loop_frame13WideLoopFrameE0Ba_")
        direct_vm = ("__RINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute7execute"
                     "NtNtB6_2vm2VmEB8_")
        symbols = fx.read_symbols(fx.ENGINE + ((deopt, 0x70), (direct, 0x3500), (direct_vm, 0x3600)))
        wide = finding_for(symbols, "wide executor")
        self.assertEqual(wide.chosen.text, direct)
        self.assertFalse(wide.ambiguous)
        self.assertEqual(finding_for(symbols, "interpreter executor").chosen.text, direct_vm)
        # With only the deopt closure, the old run body still owns dispatch.
        legacy = fx.read_symbols(fx.ENGINE + ((deopt, 0x70),))
        self.assertEqual(finding_for(legacy, "wide executor").chosen.text, fx.WIDE_CLOSURE)

    def test_executors_are_found_through_the_closure_wrapper(self):
        symbols = fx.read_symbols()
        wide = finding_for(symbols, "wide executor")
        self.assertEqual(wide.chosen.text, fx.WIDE_CLOSURE)
        self.assertFalse(wide.ambiguous)
        self.assertEqual(wide.address, symbols.address_of(fx.WIDE_CLOSURE))
        self.assertEqual(wide.size, 0x2100)
        self.assertNotEqual(wide.offset, fx.read_symbols().address_of(fx.CALL_LEAF) % 0x1000)
        interpreter = finding_for(symbols, "interpreter executor")
        self.assertEqual(interpreter.chosen.text, fx.INTERPRETER_CLOSURE)
        self.assertEqual(interpreter.alternatives, (layout_pin.INTERPRETER_EXECUTOR_FALLBACK,))
        # The fixture puts the two instantiations on different page offsets.
        self.assertNotEqual(wide.offset, interpreter.offset)

    def test_a_primary_executor_alone_is_not_a_failure(self):
        entries = tuple((name, size) for name, size in fx.ENGINE
                        if name != fx.INTERPRETER_FALLBACK)
        symbols = fx.read_symbols(entries)
        layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)  # must not raise
        self.assertEqual(finding_for(symbols, "interpreter executor").alternatives, ())

    def test_the_two_executor_roles_never_conflate_vm_with_the_wide_loop(self):
        symbols = fx.read_symbols()
        wide = finding_for(symbols, "wide executor")
        interpreter = finding_for(symbols, "interpreter executor")
        self.assertNotEqual(wide.target, interpreter.target)
        vm_loop = "qjs_runtime::bytecode::typed_loop::execute::run::" \
                  "<qjs_runtime::bytecode::vm::Vm>"
        self.assertEqual(interpreter.target, vm_loop)
        # `Vm::run_current_activation` is a different function with a similar
        # name, and it is in this binary.
        self.assertIn(fx.VM_RUN_CURRENT_ACTIVATION, symbols.names)
        self.assertNotEqual(wide.target, vm_loop)

    def test_two_live_bodies_of_one_target_are_ambiguous(self):
        symbols = fx.read_symbols(fx.ENGINE + ((fx.WIDE_PLAIN_OLD, 0x800),))
        wide = finding_for(symbols, "wide executor")
        self.assertTrue(wide.ambiguous)
        self.assertIsNone(wide.chosen)
        self.assertEqual(len(wide.addresses), 2)
        with self.assertRaises(layout_pin.LayoutError) as caught:
            layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)
        self.assertIn("wide executor", str(caught.exception))

    def test_an_explicit_selection_resolves_ambiguity(self):
        symbols = fx.read_symbols(fx.ENGINE + ((fx.WIDE_PLAIN_OLD, 0x800),))
        with patch.dict(layout_pin.SELECTED_SPELLINGS, {EXECUTOR: fx.WIDE_PLAIN_OLD}):
            wide = finding_for(symbols, "wide executor")
            self.assertFalse(wide.ambiguous)
            self.assertEqual(wide.chosen.text, fx.WIDE_PLAIN_OLD)
            layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)  # must not raise
        with patch.dict(layout_pin.SELECTED_SPELLINGS, {EXECUTOR: "not_a_spelling"}):
            with self.assertRaises(layout_pin.LayoutError) as caught:
                layout_pin.findings_for(symbols, ())
            self.assertIn("which the binary does not define", str(caught.exception))

    def test_the_two_bodies_of_drop_in_place_make_it_unpinnable(self):
        symbols = fx.read_symbols()
        tail = finding_for(symbols, "tail callee", TAIL_DROP)
        self.assertTrue(tail.ambiguous)
        self.assertEqual([entry.text for entry in tail.binary],
                         [fx.DROP_IN_PLACE, fx.DROP_IN_PLACE_DUPLICATE])
        self.assertIsNone(tail.chosen)
        head = layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)
        self.assertNotIn(fx.DROP_IN_PLACE, head)
        self.assertNotIn(fx.DROP_IN_PLACE_DUPLICATE, head)

    def test_a_callee_the_binary_lacks_is_reported_not_inferred(self):
        symbols = fx.read_symbols()
        missing = [finding for finding in layout_pin.findings_for(symbols, fx.ENGINE_ORDER)
                   if finding.role == "callee" and not finding.in_binary]
        self.assertEqual([finding.target for finding in missing],
                         [layout_pin.WIDE_EXECUTOR_FALLBACK, BOXED_EQUALITY, *layout_pin.CALLEES[16:]])
        self.assertTrue(all(not finding.in_binary for finding in missing))
        # The order file names an old spelling of the first one; nothing names
        # the second at all. Either way the head may not write them.
        self.assertTrue(missing[0].listed)
        unlisted = [finding for finding in layout_pin.findings_for(symbols, ())
                    if finding.target == BOXED_EQUALITY]
        self.assertFalse(unlisted[0].listed)
        self.assertTrue(finding_for(symbols, "callee", GET_NAMED).in_binary)


class RequireTests(unittest.TestCase):
    def test_a_binary_without_the_executor_fails_even_when_the_order_file_names_it(self):
        entries = tuple((name, size) for name, size in fx.ENGINE if name != fx.WIDE_CLOSURE)
        symbols = fx.read_symbols(entries)
        with self.assertRaises(layout_pin.LayoutError) as caught:
            layout_pin.pin_head(fx.ENGINE_ORDER, symbols, offset=0x80, vm_offset=0x40)
        self.assertIn("wide executor", str(caught.exception))

    def test_an_executor_that_shares_an_address_cannot_be_pinned(self):
        entries = ((fx.WIDE_PLAIN_OLD, 0), (fx.WIDE_CLOSURE, 0x2100),
                   (fx.INTERPRETER_CLOSURE, 0x1000), (fx.MAIN, 0x200))
        symbols = fx.read_symbols(entries)
        self.assertIsNone(symbols.size_of(fx.WIDE_CLOSURE))
        with self.assertRaises(layout_pin.LayoutError) as caught:
            layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)
        self.assertIn("wide executor", str(caught.exception))

    def test_the_interpreter_executor_is_only_required_when_asked_for(self):
        entries = tuple((name, size) for name, size in fx.ENGINE
                        if name not in (fx.INTERPRETER_CLOSURE, fx.INTERPRETER_FALLBACK))
        symbols = fx.read_symbols(entries)
        layout_pin.pin_head([], symbols, offset=0x80, vm_offset=None)  # must not raise
        with self.assertRaises(layout_pin.LayoutError) as caught:
            layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)
        self.assertIn("interpreter executor", str(caught.exception))


class PinTests(unittest.TestCase):
    def test_head_places_the_executors_at_their_offsets(self):
        symbols = fx.read_symbols()
        head = layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40)
        self.assertEqual(slot_position(head, symbols, fx.INTERPRETER_CLOSURE) % 0x1000, 0x40)
        self.assertEqual(slot_position(head, symbols, fx.WIDE_CLOSURE) % 0x1000, 0x80)
        self.assertLess(head.index(fx.INTERPRETER_CLOSURE), head.index(fx.WIDE_CLOSURE))
        for name in head[:head.index(fx.INTERPRETER_CLOSURE)]:
            self.assertEqual(layout_pin.definition_crate(name), "std")

    def test_the_head_writes_only_names_the_binary_has(self):
        symbols = fx.read_symbols()
        head = layout_pin.pin_head(fx.ENGINE_ORDER, symbols, offset=0x80, vm_offset=0x40)
        for name in head:
            self.assertIn(name, symbols.sizes, name)
        # The callee the binary lacks, the stale old spellings, and the
        # ambiguous duplicate are left out rather than written anyway.
        self.assertNotIn(layout_pin.WIDE_EXECUTOR_FALLBACK, head)
        self.assertNotIn(fx.BOXED_EQUALITY_OLD, head)
        self.assertNotIn(fx.WIDE_PLAIN_OLD, head)
        self.assertNotIn(fx.DROP_IN_PLACE_DUPLICATE, head)

    def test_a_callee_slot_keeps_the_next_callee_in_place_when_it_shrinks(self):
        symbols = fx.read_symbols()
        next_callee = fx.TRY_EVAL_NUMERIC_LEAF  # the callee slot after call_leaf
        budgets: dict[str, int] = {}
        head = layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40, budgets=budgets)
        where = slot_position(head, symbols, next_callee)
        self.assertEqual(budgets[fx.CALL_LEAF],
                         layout_pin.slot_budget(symbols.size_of(fx.CALL_LEAF),
                                                budgets.get(fx.CALL_LEAF)))
        smaller = dict(symbols.sizes, **{fx.CALL_LEAF: 0x180})
        shrunk = layout_pin.TextSymbols(symbols.start, symbols.names, symbols.addresses,
                                        smaller, symbols.counts)
        # The recorded slot lives in the order file, so a re-pin is given it
        # back; that is what keeps a function that fits inside its slot from
        # moving anything after it.
        shrunk_head = layout_pin.pin_head([], shrunk, offset=0x80, vm_offset=0x40,
                                          budgets=budgets)
        self.assertEqual(budgets[fx.CALL_LEAF], 0x400)
        self.assertEqual(slot_position(shrunk_head, shrunk, next_callee), where)

    def test_a_recorded_slot_larger_than_a_page_is_padded_in_full(self):
        # A slot big enough that its padding exceeds the page size: a rewrite
        # that padded modulo the page would lose a page of filler and move
        # every callee after it. Both runs pad their callees; only call_leaf's
        # recorded slot differs.
        symbols = fx.read_symbols()
        next_callee = fx.TRY_EVAL_NUMERIC_LEAF
        base = layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40,
                                   budgets={fx.CALL_LEAF: 0x400})
        head = layout_pin.pin_head([], symbols, offset=0x80, vm_offset=0x40,
                                   budgets={fx.CALL_LEAF: 0x3000})
        self.assertEqual(slot_position(head, symbols, next_callee)
                         - slot_position(base, symbols, next_callee), 0x3000 - 0x400)

    def test_pin_puts_the_head_before_the_rest_of_the_list(self):
        symbols = fx.read_symbols()
        body = [fx.MAIN_NAME, fx.CALL_LEAF, fx.CLONE]
        pinned = layout_pin.pin(body, symbols, offset=0x80, vm_offset=0x40)
        head_length = len(pinned) - len([line for line in body if line not in pinned[:0]] or [])
        self.assertLess(pinned.index(fx.WIDE_CLOSURE), pinned.index(fx.CALL_LEAF))
        self.assertLess(pinned.index(fx.WIDE_CLOSURE), pinned.index(fx.CLONE))
        # Every body line not used by the head still follows it, and appears once.
        for line in body:
            self.assertEqual(pinned.count(line), 1)
        self.assertLess(pinned.index(fx.CLONE), pinned.index(fx.MAIN_NAME))
        self.assertEqual(head_length, len(pinned) - len(body))


class OrderFileTests(unittest.TestCase):
    def test_audit_reads_the_whole_file_including_a_previous_pin_head(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(
                Path(work) / "order", [fx.MAIN_NAME, fx.CLONE],
                header=["# pinned: typed-loop executor at 0x80 mod 4 KiB "
                        "(python3 -m tools.benchmark.layout_pin), head 1",
                        fx.WIDE_PLAIN_OLD])
            order_file = layout_pin.read_order(order)
            self.assertEqual(order_file.head_length, 1)
            self.assertEqual(order_file.head, (fx.WIDE_PLAIN_OLD,))
            self.assertEqual(order_file.body, (fx.MAIN_NAME, fx.CLONE))
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        # The head is where a stale name hides: reading only the body a rewrite
        # would keep would not have shown it.
        stale_names = [entry.name for entry in report.stale]
        self.assertIn(fx.WIDE_PLAIN_OLD, stale_names)
        self.assertEqual([entry.replacement for entry in report.stale
                          if entry.name == fx.WIDE_PLAIN_OLD], [fx.WIDE_CLOSURE])
        self.assertFalse(report.ok())

    def test_staleness_is_judged_on_the_raw_spelling_not_the_canonical_one(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [fx.WIDE_PLAIN_OLD])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        stale = report.stale[0]
        self.assertEqual(stale.name, fx.WIDE_PLAIN_OLD)
        self.assertEqual(stale.replacement, fx.WIDE_CLOSURE)
        self.assertTrue(stale.engine)
        # The function exists in the binary under another spelling; the match
        # is a replacement to write, never a reason to call the entry live.
        self.assertEqual(report.slot("wide executor").chosen.text, fx.WIDE_CLOSURE)
        self.assertFalse(report.ok())
        self.assertTrue(any(fx.WIDE_PLAIN_OLD in problem for problem in report.strict_problems()))

    def test_a_natural_order_of_every_name_is_current(self):
        entries = [name for name, _ in fx.ENGINE]
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", entries)
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        self.assertEqual(report.stale, ())
        self.assertEqual(report.executor_problems(), ())
        self.assertEqual(report.strict_problems(), ())
        self.assertTrue(report.ok())
        # The informational findings are still there: optional callees are
        # gone and the tail duplicate cannot be pinned.
        problems = report.problems()
        self.assertTrue(any(BOXED_EQUALITY in problem for problem in problems))
        self.assertTrue(any("tail callee" in problem for problem in problems))
        self.assertTrue(all("wide executor" not in problem for problem in problems))

    def test_a_stale_budget_and_a_duplicate_entry_are_reported(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(
                Path(work) / "order", [fx.MAIN_NAME, fx.MAIN_NAME],
                header=[f"# budget 0x100 {fx.WIDE_PLAIN_OLD}",
                        f"# budget 0x100 {fx.CALL_LEAF}"])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        by_name = {entry.name: entry for entry in report.stale}
        self.assertIn(fx.WIDE_PLAIN_OLD, by_name)       # a slot naming a stale symbol
        self.assertIn(fx.CALL_LEAF, by_name)            # a slot too small for the function
        self.assertEqual(by_name[fx.CALL_LEAF].source, "budget")
        self.assertEqual(report.duplicates, (fx.MAIN_NAME,))
        self.assertFalse(report.ok())
        self.assertTrue(any("more than once" in problem
                            for problem in report.strict_problems()))

    def test_a_pin_is_byte_idempotent_and_reports_what_it_dropped(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", list(fx.ENGINE_ORDER))
            with fx.tool_patch():
                first = layout_pin.pin_order_file(order, Path("qjs"), 0x80, 0x40)
                once = order.read_text()
                layout_pin.pin_order_file(order, Path("qjs"), 0x80, 0x40)
            self.assertEqual(once, order.read_text())
            # The rewrite names the current spellings, never the old ones, and
            # reports the historical target it had to drop.
            order_file = layout_pin.read_order(order)
            self.assertIn(fx.WIDE_CLOSURE, order_file.head)
            self.assertIn(fx.INTERPRETER_CLOSURE, order_file.head)
            self.assertNotIn(fx.WIDE_PLAIN_OLD, order_file.head)
            self.assertNotIn(fx.WIDE_PLAIN_OLD, order_file.body)
            self.assertNotIn(fx.BOXED_EQUALITY_OLD, order_file.symbols)
            self.assertTrue(any(fx.BOXED_EQUALITY_OLD in problem
                                for problem in first.problems()))
            # A body name the binary does have is preserved as written.
            self.assertIn(fx.CALL_LEAF, order_file.symbols)

    def test_pin_order_file_refuses_a_binary_without_the_executor(self):
        entries = tuple((name, size) for name, size in fx.ENGINE if name != fx.WIDE_CLOSURE)
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", list(fx.ENGINE_ORDER))
            original = order.read_text()
            with fx.tool_patch(entries):
                with self.assertRaises(layout_pin.LayoutError):
                    layout_pin.pin_order_file(order, Path("qjs"), 0x80, 0x40)
            self.assertEqual(order.read_text(), original)

    def test_pin_order_file_refuses_a_stale_name_it_cannot_place(self):
        # An engine name that is neither in the binary nor a known target: a
        # rewrite cannot tell where it belongs, so it must not write a file
        # that looks pinned while naming a function that is gone.
        body = list(fx.ENGINE_ORDER) + [fx.REMOVED_BY_LTO_OLD]
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", body)
            original = order.read_text()
            with fx.tool_patch():
                with self.assertRaises(layout_pin.LayoutError):
                    layout_pin.pin_order_file(order, Path("qjs"), 0x80, 0x40)
            self.assertEqual(order.read_text(), original)

    def test_executor_offsets_read_where_the_binary_put_them(self):
        symbols = fx.read_symbols()
        with fx.tool_patch():
            wide, interpreter = layout_pin.executor_offsets(Path("qjs"))
        self.assertEqual(wide, symbols.address_of(fx.WIDE_CLOSURE) % 0x1000)
        self.assertEqual(interpreter, symbols.address_of(fx.INTERPRETER_CLOSURE) % 0x1000)
        self.assertNotEqual(wide, interpreter)

    def test_preserve_offsets_reads_the_binary_and_rejects_explicit_offsets(self):
        symbols = fx.read_symbols()
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [])
            with fx.tool_patch():
                self.assertEqual(layout_pin.main(["--binary", "qjs", "--order", str(order),
                                                  "--preserve-offsets"]), 0)
            head = layout_pin.read_order(order).head
            self.assertEqual(slot_position(head, symbols, fx.WIDE_CLOSURE) % 0x1000,
                             symbols.address_of(fx.WIDE_CLOSURE) % 0x1000)
            with fx.tool_patch():
                self.assertEqual(layout_pin.main(["--binary", "qjs", "--order", str(order),
                                                  "--preserve-offsets", "--offset", "0x0"]), 1)


class ReportTests(unittest.TestCase):
    def test_summary_and_problems_are_deterministic(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", list(fx.ENGINE_ORDER))
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        self.assertEqual(report.summary(), report.summary())
        self.assertEqual(report.problems(), report.problems())
        self.assertIn("callees 14/19 in the binary", report.summary())
        self.assertEqual(report.ordered()[0].role, "wide executor")
        self.assertEqual(len(report.ordered()),
                         len(layout_pin.CALLEES) + len(layout_pin.TAIL_CALLEES) + 2)


if __name__ == "__main__":
    unittest.main()


class PreserveOptionalInterpreterTests(unittest.TestCase):
    def test_preserve_mode_does_not_enable_an_absent_interpreter(self):
        entries = tuple((name, size) for name, size in fx.ENGINE
                        if name not in (fx.INTERPRETER_CLOSURE, fx.INTERPRETER_FALLBACK))
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [name for name, _ in entries])
            with fx.tool_patch(entries):
                self.assertEqual(layout_pin.main(["--binary", "qjs", "--order", str(order),
                                                   "--preserve-offsets"]), 0)
            self.assertNotIn("interpreter's at", order.read_text())
