from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark import screen, symbols, tl_trace


def _result(case_id: str, cycles: list[float]) -> screen.CaseResult:
    return screen.CaseResult(case_id, "whole", None,
                             {"instructions": cycles, "cycles": cycles, "wall_ns": cycles})


class PlanGateTests(unittest.TestCase):
    gate = screen.PlanGate(
        unit_id="u", base_sha="0" * 40,
        target_ids=("external/s/target",), control_ids=("broad/control",),
        target_max=0.97, control_max=1.03,
    )

    def _results(self, target: list[float], control: float, sentinel: float) -> list[screen.CaseResult]:
        results = [_result("external/s/target", target), _result("broad/control", [control])]
        results += [_result(spec, [sentinel]) for spec in self.gate.case_specs()[2:]]
        return results

    def test_case_specs_add_every_sentinel_once(self):
        specs = self.gate.case_specs()
        self.assertEqual(specs[:2], ["external/s/target", "broad/control"])
        self.assertEqual(len(specs), 2 + 6)
        self.assertTrue(all(spec.startswith("sentinel/") for spec in specs[2:]))

    def test_pass(self):
        passed, failures = screen.gate_verdict(self._results([0.92, 0.93, 0.91], 1.0, 1.01), self.gate)
        self.assertTrue(passed, failures)

    def test_target_median_above_threshold_fails(self):
        passed, failures = screen.gate_verdict(self._results([0.98, 0.98, 0.98], 1.0, 1.0), self.gate)
        self.assertFalse(passed)
        self.assertIn("median cycles", failures[0])

    def test_one_non_improving_target_pair_fails(self):
        passed, failures = screen.gate_verdict(self._results([0.90, 0.91, 1.0], 1.0, 1.0), self.gate)
        self.assertFalse(passed)
        self.assertIn("did not improve", failures[0])

    def test_control_or_sentinel_regression_fails(self):
        self.assertFalse(screen.gate_verdict(self._results([0.9], 1.04, 1.0), self.gate)[0])
        self.assertFalse(screen.gate_verdict(self._results([0.9], 1.0, 1.04), self.gate)[0])

    def test_load_plan_gate_reads_fast_gate(self):
        with tempfile.TemporaryDirectory() as work:
            plan = Path(work) / "u.json"
            plan.write_text(json.dumps({
                "unit_id": "u", "base_sha": "a" * 40,
                "fast_gate": {"target_ids": ["t"], "control_ids": ["c"],
                              "target_max_candidate_over_base": 0.97,
                              "control_max_candidate_over_base": 1.03, "max_attempts": 2},
            }))
            gate = screen.load_plan_gate(plan)
            self.assertEqual((gate.target_ids, gate.control_max), (("t",), 1.03))
            plan.write_text(json.dumps({"unit_id": "u", "base_sha": "a" * 40}))
            with self.assertRaises(screen.ScreenError):
                screen.load_plan_gate(plan)


class SymbolTests(unittest.TestCase):
    def test_sizes_come_from_address_gaps_and_sum_duplicate_names(self):
        output = (
            "0000000100000000 T __mh_execute_header\n"
            "0000000100000890 t alloc::boxed::box_new_uninit\n"
            "00000001000008c0 t core::ptr::drop_in_place<Value>\n"
            "00000001000008e0 t core::ptr::drop_in_place<Value>\n"
            "0000000100000a00 s literal_data\n"
        )
        sizes = symbols.parse_nm(output)
        self.assertEqual(sizes["alloc::boxed::box_new_uninit"], 0x30)
        self.assertEqual(sizes["core::ptr::drop_in_place<Value>"], 0x20 + 0x120)
        self.assertNotIn("literal_data", sizes)

    def test_rust_hash_suffix_does_not_split_a_function(self):
        base = symbols.parse_nm("0000000000000010 t qjs::vm::run::h0123456789abcdef\n0000000000000050 t end\n")
        cand = symbols.parse_nm("0000000000000010 t qjs::vm::run::hfedcba9876543210\n0000000000000060 t end\n")
        self.assertEqual((base, cand), ({"qjs::vm::run": 0x40}, {"qjs::vm::run": 0x50}))

    def test_render_orders_by_absolute_change(self):
        text = symbols.render([("probe", 1200, 544), ("run", 100, 120)], 5)
        self.assertLess(text.index("probe"), text.index("run"))
        self.assertIn("-656", text)
        self.assertIn("no function changed size", symbols.render([], 5))


class TraceSummaryTests(unittest.TestCase):
    def test_histograms_join_edges_with_last_reason(self):
        lines = [
            "TLGIVEUP region 7..41 at ip 34 op Some(StoreLocal(2)) discovered_boxed=[2]\n",
            "TLEDGE region 7..41\n",
            "TLEDGE region 7..41\n",
            "TLEDGE region 9..12\n",
            "TLDEOPT region 7..41 ip 12 op Unbox { dst: 1 } bc 99 extra\n",
            "TLDEOPT region 7..41 ip 30 op Unbox { dst: 1 } bc 7 extra\n",
        ]
        text = tl_trace.summarize(lines)
        self.assertIn("2  TLDEOPT region 7..41 op Unbox { dst: 1 }", text)
        self.assertIn("2  7..41  TLGIVEUP region 7..41 at ip 34", text)
        self.assertIn("1  9..12  (no give-up/decline line)", text)


if __name__ == "__main__":
    unittest.main()
