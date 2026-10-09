from __future__ import annotations

import unittest

from tools.benchmark.preview_summary import (
    GROUP_NOISE,
    HOW_TO_READ,
    LISTED_TESTS,
    TEST_NOISE,
    render_preview,
)

PUSH = "main_push_head_owned_harness"


def lane_case(case_id: str, ratio: float, *, median: float = 100.0) -> dict[str, object]:
    return {
        "id": case_id, "ratio": ratio,
        "ci_lower": ratio * 0.98, "ci_upper": ratio * 1.02,
        "candidate_median_ns_per_op": median,
        "comparator_median_ns_per_op": median / ratio,
    }


def broad(
    base: dict[str, float], reference: float = 1.5, *, overall: float = 1.01,
    mode: str = PUSH, base_binary: str = "2",
) -> dict[str, object]:
    def comparison(ratios: dict[str, float], ratio: float) -> dict[str, object]:
        return {
            "ratio": ratio, "ci_lower": ratio * 0.99, "ci_upper": ratio * 1.01,
            "cases": [lane_case(case_id, value) for case_id, value in ratios.items()],
        }

    return {
        "health": "inconclusive", "linearity": "pass",
        "valid_blocks": 3, "requested_blocks": 3,
        "harness": {"mode": mode, "revision": "a" * 40},
        "integrity_scope": "trusted_main_push",
        "profile_id": "github-hosted-linux-x86_64-informational-v1",
        "engines": {
            role: {"source_revision": digit * 40, "binary_sha256": binary * 64}
            for role, digit, binary in (
                ("candidate", "1", "1"), ("base", "2", base_binary), ("quickjs-ng", "3", "3"),
            )
        },
        "comparisons": {
            "candidate vs base": comparison(base, overall),
            "candidate vs QuickJS-NG": comparison(dict.fromkeys(base, reference), reference),
        },
    }


def sentinel(
    base: dict[str, float], reference: float = 1.25, *, overall: float = 0.98,
) -> dict[str, object]:
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
            "candidate vs base": comparison(base, overall),
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


def external(
    cases: list[dict[str, object]], *, base: float = 1.004, reference: float = 0.825,
) -> dict[str, object]:
    comparable = sum(case["candidate_over_base"] is not None for case in cases)
    return {
        "blocks": 3,
        "analysis": {"min_blocks": 30},
        "suites": [{
            "id": "kraken-1.1", "name": "Kraken 1.1",
            "case_count": len(cases),
            "base_comparable_case_count": comparable,
            "comparable_case_count": comparable,
            "diagnostic_candidate_over_base_geomean_ratio": base,
            "diagnostic_comparable_case_geomean_ratio": reference,
            "cases": cases,
        }],
    }


def above_the_fold(markdown: str) -> str:
    return markdown.split("<details>", 1)[0]


class VerdictTests(unittest.TestCase):
    def test_the_first_thing_said_is_whether_speed_changed(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.02}),
            external([external_case("ai-astar", 1.015, 1.353)]),
            sentinel({"recursive_call_tree": 0.97}, overall=0.99),
        )
        first = markdown.split("\n\n")[1]
        self.assertEqual(
            first,
            "**This commit: no clear change in speed** compared with the commit before it. "
            "Against QuickJS-NG it is slower on 2 and faster on 1 of 3 workload groups.",
        )
        self.assertIn(
            "This commit is `11111111`; the commit before it is `22222222`.", markdown
        )

    def test_a_group_beyond_noise_is_named_in_the_verdict(self) -> None:
        slower = render_preview(
            broad({"a": 1.0}, overall=1.03), external([external_case("x", 1.0, 1.0)]),
            sentinel({"b": 1.0}, overall=1.05),
        )
        self.assertIn(
            "**This commit looks slower than the commit before it** on "
            "Interpreter basics and Micro-operations.",
            slower,
        )
        faster = render_preview(broad({"a": 1.0}, overall=0.95))
        self.assertIn(
            "**This commit looks faster than the commit before it** on Micro-operations.",
            faster,
        )
        mixed = render_preview(
            broad({"a": 1.0}, overall=1.10), None, sentinel({"b": 1.0}, overall=0.90),
        )
        self.assertIn(
            "**This commit is mixed against the commit before it:** slower on "
            "Micro-operations, faster on Interpreter basics.",
            mixed,
        )

    def test_the_noise_floor_decides_what_counts_as_a_change(self) -> None:
        inside = render_preview(broad({"a": 1.0}, overall=GROUP_NOISE - 0.001))
        self.assertIn("no clear change in speed", inside)
        self.assertIn("⚪ no clear change (1.9% slower)", inside)
        at = render_preview(broad({"a": 1.0}, overall=GROUP_NOISE))
        self.assertIn("looks slower", at)
        self.assertIn("🔴 2.0% slower", at)
        # The floor is the percentage the reader is shown, on both sides.
        mirrored = render_preview(broad({"a": 1.0}, overall=2 - GROUP_NOISE))
        self.assertIn("looks faster", mirrored)
        self.assertIn("🟢 2.0% faster", mirrored)
        near = render_preview(broad({"a": 1.0}, overall=2 - GROUP_NOISE + 0.001))
        self.assertIn("⚪ no clear change (1.9% faster)", near)

    def test_byte_identical_builds_are_called_noise_whatever_was_measured(self) -> None:
        markdown = render_preview(broad({"a": 1.0}, overall=1.06, base_binary="1"))
        self.assertIn(
            "**This commit did not change the engine:** its build is byte-identical to "
            "the commit before it, so every difference below is measurement noise.",
            markdown,
        )
        self.assertNotIn("looks slower", markdown)

    def test_identical_builds_report_a_moved_test_as_the_runs_own_noise(self) -> None:
        markdown = render_preview(
            broad({"steady": 1.0, "jumpy": 1.117, "also": 0.92}, base_binary="1")
        )
        self.assertIn(
            "**2 of 3 tests still differed by more than 7%** between the two identical "
            "builds (the furthest: `jumpy`, 11.7% slower). That is this run's own noise",
            markdown,
        )
        self.assertNotIn("worth a second look", markdown)
        self.assertNotIn("| Test | Workload |", above_the_fold(markdown))
        quiet = render_preview(broad({"steady": 1.0}, base_binary="1"))
        self.assertIn("**No single test moved by more than 7%**", quiet)

    def test_the_two_builds_are_named_by_what_the_run_compared(self) -> None:
        for mode, subject, other, heading in (
            ("base_owned_harness", "This pull request", "its base", "Against the base"),
            (PUSH, "This commit", "the commit before it", "Against the previous commit"),
            ("manual_main_head_owned_harness", "The selected commit", "the selected base",
             "Against the base"),
        ):
            with self.subTest(mode=mode):
                markdown = render_preview(broad({"a": 1.0}, mode=mode))
                self.assertIn(f"**{subject}: no clear change in speed** compared with {other}.", markdown)
                self.assertIn(f"| Workload | Size | {heading} | Against QuickJS-NG |", markdown)
        # Without the broad lane nothing says which event this was.
        fallback = render_preview(None, external([external_case("x", 1.0, 1.2)]))
        self.assertIn("**The candidate: no clear change in speed** compared with its base.", fallback)


class ReferenceVerdictTests(unittest.TestCase):
    def test_a_group_without_a_base_comparison_still_counts_against_the_reference(self) -> None:
        report = external([external_case("only-reference", None, 0.8, base="timeout")])
        report["suites"][0].update(
            diagnostic_candidate_over_base_geomean_ratio=None,
            diagnostic_comparable_case_geomean_ratio=0.8,
            base_comparable_case_count=0, comparable_case_count=1,
        )
        with_broad = render_preview(broad({"a": 1.0}, reference=1.5), report)
        self.assertIn("| Kraken 1\\.1 |", with_broad)
        self.assertIn("| — | 🟢 1.25× faster |", with_broad)
        self.assertIn(
            "Against QuickJS-NG it is slower on 1 and faster on 1 of 2 workload groups.",
            with_broad,
        )
        # With no base comparison anywhere, the reference still gets its sentence.
        alone = render_preview(None, report)
        self.assertEqual(
            alone.split("\n\n")[1],
            "Against QuickJS-NG it is slower on 0 and faster on 1 of 1 workload groups.",
        )


class ExactNumberTests(unittest.TestCase):
    def test_every_worded_ratio_keeps_its_number_and_interval_in_the_fold(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.0234}, reference=1.5),
            external([
                external_case("ai-astar", 1.015, 1.353),
                external_case("no-base", None, 0.9, base="timeout"),
            ]),
            sentinel({"recursive_call_tree": 0.97}, reference=1.25),
        )
        self.assertNotIn("1.0150", above_the_fold(markdown))
        exact = markdown.split("<summary>Exact ratios and intervals — all 4 tests</summary>", 1)[1]
        exact = exact.split("</details>", 1)[0]
        for row in (
            "| `kraken-1.1/ai-astar` | 1.0150 [0.9845, 1.0454] | 1.3530 [1.3124, 1.3936] |",
            "| `kraken-1.1/no-base` | — | 0.9000 [0.8730, 0.9270] |",
            "| `recursive_call_tree` | 0.9700 [0.8730, 1.0670] | 1.2500 [1.1250, 1.3750] |",
            "| `plain_function_call` | 1.0234 [1.0029, 1.0439] | 1.5000 [1.4700, 1.5300] |",
        ):
            self.assertIn(row, exact)
        # A lane without evidence contributes no rows, and no rows means no section.
        self.assertNotIn("Exact ratios", render_preview(None, None, None))


class OverviewTests(unittest.TestCase):
    def test_every_number_is_said_in_words(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.02}, reference=1.5),
            external([external_case("ai-astar", 1.015, 1.353)], base=1.004, reference=0.825),
            sentinel({"recursive_call_tree": 0.97}, reference=1.0, overall=0.985),
        )
        top = above_the_fold(markdown)
        self.assertIn(
            "| Kraken 1\\.1 | 1 programs | ⚪ no clear change (0.4% slower) | 🟢 1.21× faster |",
            top,
        )
        self.assertIn(
            "| Interpreter basics | 1 tests | ⚪ no clear change (1.5% faster) | "
            "⚪ about the same |",
            top,
        )
        self.assertIn(
            "| Micro-operations | 1 tests | ⚪ no clear change (1.0% slower) | 🔴 1.50× slower |",
            top,
        )
        # Real programs lead; the group the optimizer can shortcut is last.
        self.assertLess(top.index("Kraken"), top.index("Interpreter basics"))
        self.assertLess(top.index("Interpreter basics"), top.index("Micro-operations"))
        self.assertIn(HOW_TO_READ, top)
        # Nothing above the fold asks the reader to decode a sign, a bare
        # ratio, or the harness's own vocabulary.
        body = top.replace(HOW_TO_READ, "")
        self.assertNotRegex(body, r"[+-]\d+\.\d%")
        self.assertNotRegex(body, r"\d×(?! (slower|faster))")
        for jargon in ("candidate", "sentinel", "specializer", "interval", "binary", "geomean"):
            self.assertNotIn(jargon, body.lower())
        self.assertNotIn("(inconclusive)", markdown)
        self.assertEqual(markdown.count("<details>"), markdown.count("</details>"))
        self.assertEqual(markdown.count("<details>"), 5)
        self.assertIn("`" + "1" * 40 + "` / `" + "1" * 64 + "`", markdown)
        self.assertIn("informational only — non-gating — not a fixed-hardware claim", markdown)

    def test_an_absent_lane_is_named_with_its_reason(self) -> None:
        markdown = render_preview(
            broad({"plain_function_call": 1.0}), None, None,
            {"sentinel": "the sentinel lane did not complete within its deadline."},
        )
        self.assertIn("| Interpreter basics | — | — | — |", markdown)
        self.assertIn("| Benchmark programs | — | — | — |", markdown)
        self.assertIn(
            "- **Interpreter basics:** the sentinel lane did not complete within its deadline.",
            markdown,
        )
        self.assertIn("- **Benchmark programs:** no evidence was produced.", markdown)
        self.assertNotIn("<summary>Interpreter basics", markdown)
        self.assertNotIn("<summary>Benchmark programs", markdown)

    def test_no_lane_at_all_still_renders_a_truthful_document(self) -> None:
        markdown = render_preview(None, None, None, {"broad": "the broad lane failed."})
        self.assertIn("- **Micro-operations:** the broad lane failed.", markdown)
        self.assertNotRegex(
            markdown.replace(HOW_TO_READ, ""), r"\d\.\d+×|\d+\.\d% (slower|faster)"
        )
        self.assertNotIn("no clear change in speed", markdown)
        self.assertNotIn("moved by more than", markdown)

    def test_partial_and_unequal_program_coverage_is_spelled_out(self) -> None:
        report = external([external_case("ai-astar", 1.0, 1.3)])
        report["suites"][0].update(
            case_count=14, base_comparable_case_count=13, comparable_case_count=13
        )
        self.assertIn("| Kraken 1\\.1 | 13 of 14 programs |", render_preview(None, report))
        report["suites"][0]["comparable_case_count"] = 12
        self.assertIn(
            "| 13 of 14 programs against the base, 12 against QuickJS-NG |",
            render_preview(None, report),
        )

    def test_a_sharded_broad_lane_says_so_and_shows_no_group_interval(self) -> None:
        lane = broad({"a": 1.0})
        lane["shards"] = 2
        for comparison in lane["comparisons"].values():
            del comparison["ci_lower"], comparison["ci_upper"]
        markdown = render_preview(lane)
        self.assertIn("measured as 2 shards on separate runners", markdown)
        self.assertIn("- Micro-operations, candidate vs base: 1.0100×\n", markdown)


class MovedTestTests(unittest.TestCase):
    def test_only_tests_beyond_the_noise_floor_are_listed_largest_first(self) -> None:
        markdown = render_preview(
            broad({"empty_loop": 1.01, "math_abs": 1.10, "array_read": 1.069}),
            external([
                external_case("ai-astar", 0.80, 1.3),
                external_case("audio-fft", 1.002, 0.6),
            ]),
            sentinel({"recursive_call_tree": 1.09, "string_key_map_churn": 0.95}),
        )
        section = markdown.split("**Tests that moved by more than 7%", 1)[1].split("<details>", 1)[0]
        self.assertTrue(section.startswith(" against the previous commit** (3 of 7)"))
        rows = [line for line in section.splitlines() if line.startswith("| `")]
        self.assertEqual(
            [row.split("|")[1].strip() for row in rows],
            ["`kraken-1.1/ai-astar`", "`math_abs`", "`recursive_call_tree`"],
        )
        self.assertIn(
            "| `kraken-1.1/ai-astar` | Kraken 1\\.1 | 20.0% faster | 22.4% faster to 17.6% faster |",
            section,
        )
        self.assertIn("| `math_abs` | Micro-operations | 10.0% slower | 7.8% slower to 12.2% slower |", section)
        self.assertIn("worth a second look, not a confirmed change", section)
        for word in ("regression", "win", "significant"):
            self.assertNotIn(word, section.lower())

    def test_the_floor_itself_counts_and_the_list_is_capped(self) -> None:
        ratios = {f"case_{index}": 1.20 + index / 100 for index in range(LISTED_TESTS + 2)}
        ratios["at_the_floor"] = TEST_NOISE
        ratios["just_inside"] = TEST_NOISE - 0.001
        markdown = render_preview(broad(ratios))
        section = markdown.split("**Tests that moved", 1)[1].split("<details>", 1)[0]
        self.assertIn(f"({LISTED_TESTS + 3} of {LISTED_TESTS + 4})", section)
        rows = [line for line in section.splitlines() if line.startswith("| `")]
        self.assertEqual(len(rows), LISTED_TESTS)
        self.assertIn("`case_6`", rows[0])
        self.assertNotIn("just_inside", section)

    def test_a_test_is_never_listed_beyond_a_floor_its_own_row_contradicts(self) -> None:
        # 0.934 is "6.6% faster": inside a 7% floor, however the ratio is
        # compared, so it must not appear under "moved by more than 7%".
        inside = render_preview(broad({"just_inside": 0.934, "other": 1.0}))
        self.assertIn("**No single test moved by more than 7%**", inside)
        listed = render_preview(broad({"at_floor": 0.93, "over": 1.0701, "under": 1.0699}))
        section = listed.split("**Tests that moved", 1)[1].split("<details>", 1)[0]
        self.assertIn("| `at_floor` | Micro-operations | 7.0% faster |", section)
        self.assertIn("| `over` | Micro-operations | 7.0% slower |", section)
        self.assertNotIn("`under`", section)
        for row in (line for line in section.splitlines() if line.startswith("| `")):
            shown = float(row.split("|")[3].strip().split("%")[0])
            self.assertGreaterEqual(shown, round((TEST_NOISE - 1) * 100, 6))

    def test_nothing_beyond_the_floor_is_said_in_one_sentence(self) -> None:
        markdown = render_preview(
            broad({"a": 1.05, "b": 0.96}), None, sentinel({"c": 1.03}),
        )
        self.assertIn(
            "**No single test moved by more than 7%** against the previous commit, out of 3.",
            markdown,
        )
        self.assertNotIn("| Test | Workload |", above_the_fold(markdown))

    def test_equal_movements_order_deterministically(self) -> None:
        cases = {"b_case": 1.15, "a_case": 1.15}
        first = render_preview(broad(cases))
        second = render_preview(broad(dict(reversed(list(cases.items())))))
        pick = lambda text: text.split("**Tests that moved", 1)[1].split("Identical builds", 1)[0]
        self.assertEqual(pick(first), pick(second))
        self.assertLess(pick(first).index("a_case"), pick(first).index("b_case"))


class NotComparedTests(unittest.TestCase):
    def test_a_program_without_a_ratio_is_explained_above_the_fold(self) -> None:
        markdown = render_preview(broad({"a": 1.0}), external([
            external_case("ai-astar", 1.0, 1.3),
            external_case(
                "imaging-gaussian-blur", None, None, candidate="timeout", base="timeout"
            ),
        ]), None)
        top = above_the_fold(markdown)
        self.assertIn(
            "- `kraken-1.1/imaging-gaussian-blur` — this commit timed out, the commit "
            "before it timed out, QuickJS-NG finished. It is left out of both Kraken 1\\.1 "
            "comparisons.",
            top,
        )
        self.assertIn("| Kraken 1\\.1 | 1 of 2 programs |", top)
        self.assertIn(
            "| `imaging-gaussian-blur` | — | — | 13,050.1 | — | — | — |", markdown
        )

    def test_a_one_sided_gap_names_the_comparison_it_affects(self) -> None:
        report = external([
            external_case("ai-astar", 1.0, 1.3),
            external_case("no-reference", 1.01, None, **{"quickjs-ng": "timeout"}),
            external_case("no-base", None, 0.9, base="timeout"),
        ])
        report["suites"][0]["comparable_case_count"] = 2
        top = above_the_fold(render_preview(broad({"a": 1.0}), report))
        self.assertIn(
            "- `kraken-1.1/no-reference` — this commit finished, the commit before it "
            "finished, QuickJS-NG timed out. It is left out of the Kraken 1\\.1 comparison "
            "with QuickJS-NG.",
            top,
        )
        self.assertIn(
            "QuickJS-NG finished. It is left out of the Kraken 1\\.1 comparison with the "
            "commit before it.",
            top,
        )
        self.assertNotIn("both Kraken", top)

    def test_hostile_identifiers_and_names_cannot_inject_markup(self) -> None:
        report = external([
            external_case("x`<script>|![a](b)", 1.2, 1.3),
            external_case("y", None, None, candidate="<b>|x"),
        ])
        report["suites"][0]["name"] = "<img src=x>|**bold**"
        markdown = render_preview(None, report, None)
        self.assertNotIn("<script>", markdown)
        self.assertNotIn("<img", markdown)
        self.assertNotIn("<b>", markdown)
        self.assertNotIn("![a](b)", markdown)
        self.assertNotIn("x`<", markdown)


if __name__ == "__main__":
    unittest.main()
