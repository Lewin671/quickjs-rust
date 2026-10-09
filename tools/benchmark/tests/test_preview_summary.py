from __future__ import annotations

import unittest

from tools.benchmark.preview_summary import LARGEST_CHANGES, render_preview


def lane_case(case_id: str, ratio: float, *, median: float = 100.0) -> dict[str, object]:
    return {
        "id": case_id, "ratio": ratio,
        "ci_lower": ratio * 0.98, "ci_upper": ratio * 1.02,
        "candidate_median_ns_per_op": median,
        "comparator_median_ns_per_op": median / ratio,
    }


def broad(base: dict[str, float], reference: float = 1.5) -> dict[str, object]:
    def comparison(ratios: dict[str, float], overall: float) -> dict[str, object]:
        return {
            "ratio": overall, "ci_lower": overall * 0.99, "ci_upper": overall * 1.01,
            "cases": [lane_case(case_id, ratio) for case_id, ratio in ratios.items()],
        }

    return {
        "health": "inconclusive", "linearity": "pass",
        "valid_blocks": 3, "requested_blocks": 3,
        "harness": {"mode": "main_push_head_owned_harness", "revision": "a" * 40},
        "integrity_scope": "trusted_main_push",
        "profile_id": "github-hosted-linux-x86_64-informational-v1",
        "engines": {
            role: {"source_revision": digit * 40, "binary_sha256": digit * 64}
            for role, digit in (("candidate", "1"), ("base", "2"), ("quickjs-ng", "3"))
        },
        "comparisons": {
            "candidate vs base": comparison(base, 1.01),
            "candidate vs QuickJS-NG": comparison(dict.fromkeys(base, reference), reference),
        },
    }


def sentinel(base: dict[str, float], reference: float = 1.25) -> dict[str, object]:
    def comparison(ratios: dict[str, float], geomean: float) -> dict[str, object]:
        return {
            "geomean": geomean,
            "cases": [
                {"id": case_id, "ratio": ratio, "ci_lower": ratio * 0.9, "ci_upper": ratio * 1.1}
                for case_id, ratio in ratios.items()
            ],
        }

    return {
        "health": "inconclusive", "linearity": "pass", "valid_blocks": 3,
        "comparisons": {
            "candidate vs base": comparison(base, 0.98),
            "candidate vs QuickJS-NG": comparison(dict.fromkeys(base, reference), reference),
        },
    }


def external_case(
    case_id: str, base_ratio: float | None, reference_ratio: float | None,
    **capability: str,
) -> dict[str, object]:
    def effect(ratio: float | None) -> dict[str, object] | None:
        if ratio is None:
            return None
        return {
            "confidence_interval": {"lower": ratio * 0.97, "upper": ratio * 1.03},
            "status": "inconclusive",
        }

    measured = base_ratio is not None
    return {
        "id": case_id,
        "candidate_over_base": base_ratio,
        "candidate_over_quickjs_ng": reference_ratio,
        "capability": {
            "candidate": "ok", "base": "ok", "quickjs-ng": "ok", **capability,
        },
        "median_duration_ns": {
            "candidate": 40_000_000 if measured else None,
            "base": 41_000_000 if measured else None,
            "quickjs-ng": 13_050_093_766,
        },
        "paired_comparisons": {
            "base": effect(base_ratio), "quickjs-ng": effect(reference_ratio),
        },
    }


def external(cases: list[dict[str, object]]) -> dict[str, object]:
    comparable = sum(case["candidate_over_base"] is not None for case in cases)
    return {
        "blocks": 3,
        "analysis": {"min_blocks": 30},
        "suites": [{
            "id": "kraken-1.1", "name": "Kraken 1.1",
            "case_count": len(cases),
            "base_comparable_case_count": comparable,
            "comparable_case_count": comparable,
            "diagnostic_candidate_over_base_geomean_ratio": 1.004,
            "diagnostic_comparable_case_geomean_ratio": 0.825,
            "cases": cases,
        }],
    }


class OverviewTests(unittest.TestCase):
    def test_the_answer_precedes_every_per_case_table(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.02}),
            external([external_case("ai-astar", 1.015, 1.353)]),
            sentinel({"recursive_call_tree": 0.97}),
        )
        overview = markdown.split("<details>", 1)[0]
        self.assertIn("**Candidate `11111111` vs base `22222222`**", overview)
        self.assertIn("| Interpreter sentinels | 1/1 | -2.0% | 1.250× |", overview)
        self.assertIn("| Kraken 1\\.1 | 1/1 | +0.4% | 0.825× |", overview)
        self.assertIn(
            "| Broad microbenchmarks (specializer coverage) | 1/1 | +1.0% | 1.500× |",
            overview,
        )
        # Sentinels and whole programs lead; the foldable broad lane is last.
        self.assertLess(overview.index("Interpreter sentinels"), overview.index("Kraken"))
        self.assertLess(overview.index("Kraken"), overview.index("Broad microbenchmarks"))
        self.assertIn("negative change or a ratio below 1× means the candidate took less time", overview)
        self.assertNotIn("binary", overview.lower())
        self.assertNotIn("(inconclusive)", markdown)
        self.assertEqual(markdown.count("<details>"), markdown.count("</details>"))
        self.assertEqual(markdown.count("<details>"), 4)
        self.assertIn("`" + "1" * 40 + "` / `" + "1" * 64 + "`", markdown)

    def test_an_absent_lane_is_named_with_its_reason(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.0}), None, None,
            {"sentinel": "the sentinel lane did not complete within its deadline."},
        )
        self.assertIn("| Interpreter sentinels | — | — | — |", markdown)
        self.assertIn("| External benchmarks | — | — | — |", markdown)
        self.assertIn(
            "- **Interpreter sentinels:** the sentinel lane did not complete within its deadline.",
            markdown,
        )
        self.assertIn("- **External benchmarks:** no evidence was produced.", markdown)
        self.assertNotIn("<summary>Interpreter sentinels", markdown)
        self.assertNotIn("<summary>External benchmarks", markdown)

    def test_no_lane_at_all_still_renders_a_truthful_document(self) -> None:
        markdown = render_preview(None, None, None, {"broad": "the broad lane failed."})
        self.assertIn("- **Broad microbenchmarks (specializer coverage):** the broad lane failed.", markdown)
        self.assertNotRegex(markdown, r"\d\.\d+×|[+-]\d+\.\d%")
        self.assertNotIn("Largest observed changes", markdown)

    def test_unequal_external_coverage_is_spelled_out(self) -> None:
        report = external([external_case("ai-astar", 1.0, 1.3)])
        report["suites"][0]["case_count"] = 14
        report["suites"][0]["base_comparable_case_count"] = 13
        report["suites"][0]["comparable_case_count"] = 12
        markdown = render_preview(None, report, None)
        self.assertIn("| base 13/14 · QuickJS-NG 12/14 |", markdown)


class LargestChangeTests(unittest.TestCase):
    def test_cases_are_ranked_by_magnitude_across_lanes_in_either_direction(self) -> None:
        markdown = render_preview(
            broad({"empty_loop": 1.01, "math_abs": 1.10, "array_read": 0.999}),
            external([
                external_case("ai-astar", 0.80, 1.3),
                external_case("audio-fft", 1.002, 0.6),
            ]),
            sentinel({"recursive_call_tree": 1.06, "string_key_map_churn": 0.95}),
        )
        section = markdown.split("**Largest observed changes vs base**", 1)[1].split(
            "<details>", 1
        )[0]
        rows = [line for line in section.splitlines() if line.startswith("| `")]
        self.assertEqual(len(rows), LARGEST_CHANGES)
        self.assertEqual(
            [row.split("|")[1].strip() for row in rows],
            [
                "`kraken-1.1/ai-astar`", "`math_abs`", "`recursive_call_tree`",
                "`string_key_map_churn`", "`empty_loop`",
            ],
        )
        self.assertIn("| `kraken-1.1/ai-astar` | Kraken 1\\.1 | -20.0% | [-22.4%, -17.6%] |", section)
        self.assertIn("Selected by observed magnitude from 7 cases", section)
        self.assertIn("not adjusted for that selection", section)
        for word in ("regression", "win", "significant", "faster", "slower"):
            self.assertNotIn(word, section.lower())

    def test_equal_magnitudes_order_deterministically(self) -> None:
        cases = {"b_case": 1.05, "a_case": 1.05}
        first = render_preview(broad(cases))
        second = render_preview(broad(dict(reversed(list(cases.items())))))
        pick = lambda text: text.split("**Largest observed changes vs base**", 1)[1].split("Selected", 1)[0]
        self.assertEqual(pick(first), pick(second))
        self.assertLess(pick(first).index("a_case"), pick(first).index("b_case"))


class IncompleteComparisonTests(unittest.TestCase):
    def test_a_case_without_a_ratio_is_reported_above_the_fold(self) -> None:
        markdown = render_preview(None, external([
            external_case("ai-astar", 1.0, 1.3),
            external_case(
                "imaging-gaussian-blur", None, None, candidate="timeout", base="timeout"
            ),
        ]), None)
        above = markdown.split("<details>", 1)[0]
        self.assertIn(
            "- `kraken-1.1/imaging-gaussian-blur` — candidate timeout, base timeout, "
            "QuickJS-NG ok. It is excluded from the affected suite ratios.",
            above,
        )
        self.assertIn("| Kraken 1\\.1 | 1/2 |", above)
        self.assertNotIn("imaging-gaussian-blur", above.split("Cases without", 1)[0])
        self.assertIn("| `imaging-gaussian-blur` | — | — | 13,050.1 | — | — | — | — |", markdown)

    def test_hostile_identifiers_and_names_cannot_inject_markup(self) -> None:
        report = external([external_case("x`<script>|![a](b)", 1.2, 1.3)])
        report["suites"][0]["name"] = "<img src=x>|**bold**"
        markdown = render_preview(None, report, None)
        self.assertNotIn("<script>", markdown)
        self.assertNotIn("<img", markdown)
        self.assertNotIn("![a](b)", markdown)
        self.assertNotIn("x`<", markdown)


if __name__ == "__main__":
    unittest.main()
