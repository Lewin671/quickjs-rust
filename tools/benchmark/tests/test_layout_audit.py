"""Tests for the audit CLI: what it reports and when it fails closed.

The exit status is the part that matters. A stale order file links happily and
looks pinned, so `--strict` is the only mode that asserts the file is current;
it has to fail when an entry names a function the binary does not have, and
pass on a file whose every entry is a spelling the binary really has.
"""

from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark import layout_audit, layout_pin
from tools.benchmark.tests import layout_fixtures as fx


def run_main(arguments):
    """(exit status, stdout) of the audit CLI, with its stderr swallowed."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        status = layout_audit.main(arguments)
    return status, out.getvalue()


def address_of(name):
    """Where the fixture binary defines `name`, from its own row layout."""
    for address, _, row_name in fx.rows(fx.ENGINE):
        if row_name == name:
            return address
    raise AssertionError(name)


class RenderTests(unittest.TestCase):
    def test_reports_both_executors_with_their_addresses_and_offsets(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        text = layout_audit.render(report)
        self.assertIn("binary qjs", text)
        self.assertIn("wide executor", text)
        self.assertIn("interpreter executor", text)
        for role in ("wide executor", "interpreter executor"):
            self.assertTrue(any(role in line and " mod 4 KiB" in line
                                for line in text.splitlines()), role)
        self.assertIn(f"{layout_pin.WIDE_EXECUTOR} ->", text)

    def test_lists_stale_entries_with_the_replacement_to_write(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [fx.WIDE_PLAIN_OLD])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        text = layout_audit.render(report)
        self.assertIn(f"stale symbol: {fx.WIDE_PLAIN_OLD}", text)
        self.assertIn(f"the binary spells it {fx.WIDE_CLOSURE}", text)
        self.assertIn("stale", text.splitlines()[1])

    def test_limit_bounds_the_listing_but_the_json_keeps_everything(self):
        names = [fx.WIDE_PLAIN_OLD, fx.INTERPRETER_PLAIN_OLD, fx.CALL_LEAF_OLD]
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", names)
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        self.assertEqual(len(report.stale), len(names))
        self.assertEqual(layout_audit.render(report, limit=1).count("stale symbol"), 1)
        self.assertEqual(len(layout_audit.as_json(report)["stale"]), len(names))

    def test_json_reports_each_role_and_its_addresses(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        data = layout_audit.as_json(report)
        self.assertEqual(data["binary"], "qjs")
        self.assertEqual(data["text_start"], fx.START)
        roles = [finding["role"] for finding in data["findings"]]
        self.assertEqual(roles[0], "wide executor")
        wide = data["findings"][0]
        self.assertTrue(wide["in_binary"])
        self.assertEqual([entry for entry in wide["binary"]], [fx.WIDE_CLOSURE])
        self.assertEqual(wide["addresses"], [address_of(fx.WIDE_CLOSURE)])
        self.assertEqual(data["stale"], [])
        self.assertEqual(data["executor_problems"], [])
        # An empty order file still leaves the absent optional callees
        # and the duplicated tail reported, without failing the executors.
        self.assertTrue(data["problems"])

    def test_json_is_serialisable_and_stable(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [fx.WIDE_PLAIN_OLD])
            with fx.tool_patch():
                report = layout_pin.audit(Path("qjs"), order)
        first = json.dumps(layout_audit.as_json(report), sort_keys=True)
        second = json.dumps(layout_audit.as_json(report), sort_keys=True)
        self.assertEqual(first, second)
        self.assertIn(fx.WIDE_PLAIN_OLD, first)


class ExitStatusTests(unittest.TestCase):
    def test_a_current_order_file_passes_with_and_without_strict(self):
        entries = [name for name, _ in fx.ENGINE]
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", entries)
            with fx.tool_patch():
                self.assertEqual(run_main(["--binary", "qjs", "--order", str(order)])[0], 0)
                status, text = run_main(["--binary", "qjs", "--order", str(order), "--strict"])
            self.assertEqual(status, 0)
            # Optional findings are still reported, they just do not fail it.
            self.assertIn("tail callee", text)

    def test_a_stale_order_file_fails_only_under_strict(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [fx.WIDE_PLAIN_OLD])
            with fx.tool_patch():
                self.assertEqual(run_main(["--binary", "qjs", "--order", str(order)])[0], 0)
                status, _ = run_main(["--binary", "qjs", "--order", str(order), "--strict"])
            self.assertEqual(status, 1)

    def test_a_binary_without_the_executor_always_fails(self):
        entries = tuple((name, size) for name, size in fx.ENGINE if name != fx.WIDE_CLOSURE)
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", list(fx.ENGINE_ORDER))
            with fx.tool_patch(entries):
                status, _ = run_main(["--binary", "qjs", "--order", str(order)])
            self.assertEqual(status, 1)

    def test_an_unreadable_order_file_is_an_error_not_a_pass(self):
        with fx.tool_patch():
            status, _ = run_main(["--binary", "qjs", "--order",
                                  "/nonexistent/order-file"])
        self.assertEqual(status, 1)

    def test_json_mode_prints_data_and_still_fails_closed(self):
        with tempfile.TemporaryDirectory() as work:
            order = fx.write_order(Path(work) / "order", [fx.WIDE_PLAIN_OLD])
            with fx.tool_patch():
                status, text = run_main(["--binary", "qjs", "--order", str(order),
                                         "--strict", "--json"])
            self.assertEqual(status, 1)
            self.assertEqual(json.loads(text)["stale"][0]["name"], fx.WIDE_PLAIN_OLD)


if __name__ == "__main__":
    unittest.main()
