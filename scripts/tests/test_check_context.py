import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "check-context.py"
_spec = importlib.util.spec_from_file_location("check_context", SCRIPT)
check_context = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(check_context)

RESUME = """# T001: Example

- Status: active
- Implementation: partial
- Verified at: abc1234 (2026-01-01, check.sh)
- Evidence: tasks/archive/T001-log.md
- Unresolved: none
- Next action: write the parser
"""


class ContextCheckTests(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)
        self.write("AGENTS.md", "# Agents\n\nSee [tasks](tasks/README.md).\n")
        (self.root / "CLAUDE.md").symlink_to("AGENTS.md")
        self.write("README.md", "# Project\n")
        self.write("scripts/README.md", "# Scripts\n")
        self.write("docs/architecture.md", "# Architecture\n\n## Call Dispatch\n")
        self.write("tasks/README.md", "# Tasks\n\n- T001-example.md\n")
        self.write("tasks/T001-example.md", RESUME)

    def tearDown(self):
        self._dir.cleanup()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def errors(self):
        return check_context.run(self.root)

    def test_a_consistent_tree_passes(self):
        self.assertEqual(self.errors(), [])

    def test_claude_must_link_to_agents(self):
        (self.root / "CLAUDE.md").unlink()
        self.write("CLAUDE.md", "# Copy\n")
        self.assertIn("CLAUDE.md must be a symlink to AGENTS.md", self.errors())

    def test_a_document_over_budget_is_reported(self):
        limit = check_context.BUDGETS["AGENTS.md"]
        self.write("AGENTS.md", "line\n" * (limit + 1))
        self.assertTrue(any("AGENTS.md" in e and "budget" in e for e in self.errors()))

    def test_archived_records_are_exempt_from_budget_and_links(self):
        self.write(
            "tasks/archive/T000-old.md",
            "[gone](../../docs/removed.md)\n" + "line\n" * 5000,
        )
        self.write("tasks/README.md", "# Tasks\n\nT001-example.md T000-old.md\n")
        self.assertEqual(self.errors(), [])

    def test_a_missing_link_target_is_reported(self):
        self.write("docs/architecture.md", "See [x](missing.md).\n")
        self.assertTrue(any("missing.md" in e for e in self.errors()))

    def test_titled_and_reference_links_are_checked(self):
        self.write("README.md", '[guide](missing.md "Guide")\n')
        self.assertTrue(any("missing.md" in e for e in self.errors()))
        self.write("README.md", "See [guide][g].\n\n[g]: gone.md 'Guide'\n")
        self.assertTrue(any("gone.md" in e for e in self.errors()))
        self.write(
            "README.md",
            '[a](docs/architecture.md "A") and [b][b].\n\n[b]: <scripts/README.md>\n',
        )
        self.assertEqual(self.errors(), [])

    def test_angle_bracket_destinations_keep_their_spaces(self):
        self.write("docs/two words.md", "# Two\n")
        self.write(
            "README.md",
            "[a](<docs/two words.md>) and [b][b].\n\n[b]: <docs/two words.md> 'B'\n",
        )
        self.assertEqual(self.errors(), [])
        self.write("README.md", "[a](<docs/no such file.md>)\n")
        self.assertTrue(any("docs/no such file.md" in e for e in self.errors()))

    def test_a_reference_destination_may_sit_on_the_next_line(self):
        self.write("README.md", "See [g][g].\n\n[g]:\n    gone.md\n")
        self.assertTrue(any("gone.md" in e for e in self.errors()))
        self.write("README.md", "See [g][g].\n\n[g]:\n    docs/architecture.md\n")
        self.assertEqual(self.errors(), [])

    def test_a_label_with_trailing_spaces_still_takes_the_next_line(self):
        self.write("README.md", "See [g][g].\n\n[g]:   \n    gone.md\n")
        self.assertTrue(any("gone.md" in e for e in self.errors()))

    def test_parentheses_in_destinations_are_kept(self):
        self.write("docs/guide(v2).md", "# Guide\n")
        self.write(
            "README.md",
            "[a](docs/guide(v2).md) [b](docs/guide(v2).md 'B (two)') [c][c]\n\n"
            "[c]: docs/guide(v2).md\n",
        )
        self.assertEqual(self.errors(), [])
        self.write("README.md", "[c][c]\n\n[c]: docs/gone(v2).md\n")
        self.assertTrue(any("docs/gone(v2).md" in e for e in self.errors()))

    def test_image_targets_are_checked_and_code_spans_are_not(self):
        self.write("README.md", "![shot](docs/missing.png)\n")
        self.assertTrue(any("docs/missing.png" in e for e in self.errors()))
        self.write("README.md", "Write `[text](target.md)` or ``[a](`b`.md)``.\n")
        self.assertEqual(self.errors(), [])

    def test_an_empty_destination_is_a_same_document_link(self):
        self.write("README.md", "[self][self] and [top]()\n\n[self]: <>\n")
        self.assertEqual(self.errors(), [])

    def test_backticks_in_a_reference_label_are_literal(self):
        self.write("README.md", "See [`guide`].\n\n[`guide`]: docs/missing.md\n")
        self.assertTrue(any("docs/missing.md" in e for e in self.errors()))

    def test_nested_parentheses_in_a_destination_are_followed(self):
        self.write("README.md", "[guide](docs/missing(v(2)).md)\n")
        self.assertTrue(any("docs/missing(v(2)).md" in e for e in self.errors()))
        self.write("docs/kept(v(2)).md", "# Kept\n")
        self.write("README.md", "[guide](docs/kept(v(2)).md) and (an aside)\n")
        self.assertEqual(self.errors(), [])

    def test_escaped_backticks_do_not_open_a_code_span(self):
        self.write("README.md", "\\` [guide](docs/missing.md) \\`\n")
        self.assertTrue(any("docs/missing.md" in e for e in self.errors()))

    def test_an_unclosed_backtick_does_not_hide_a_link(self):
        self.write("README.md", "A stray ` then [guide](docs/missing.md)\n")
        self.assertTrue(any("docs/missing.md" in e for e in self.errors()))

    def test_escaped_characters_in_a_destination_are_unescaped(self):
        self.write("docs/a_b.md", "# A\n")
        self.write("README.md", "[a](docs/a\\_b.md)\n")
        self.assertEqual(self.errors(), [])

    def test_a_code_span_may_wrap_across_lines(self):
        self.write("README.md", "Write `[text]\n(target.md)` and `[a](b.md)\nmore`.\n")
        self.assertEqual(self.errors(), [])
        self.write("README.md", "`code`\n\nthen [a](gone.md)\nmore` text\n")
        self.assertTrue(any("README.md:3" in e and "gone.md" in e for e in self.errors()))

    def test_a_code_span_does_not_reach_across_list_items(self):
        self.write(
            "README.md",
            "- literal `\n- [guide](docs/missing.md)\n- literal `\n",
        )
        self.assertTrue(any("README.md:2" in e and "docs/missing.md" in e for e in self.errors()))

    def test_text_after_a_destination_must_be_a_title(self):
        self.write("README.md", "[outer](<README.md>suffix [guide](docs/missing.md))\n")
        self.assertTrue(any("docs/missing.md" in e for e in self.errors()))
        self.write("README.md", "[a](docs/architecture.md (a title)) [b](README.md\n'B')\n")
        self.assertEqual(self.errors(), [])

    def test_a_backslash_before_a_letter_is_kept(self):
        self.write("README.md", "[guide](docs/archi\\tecture.md)\n\n[r]: docs/archi\\tecture.md\n")
        found = [e for e in self.errors() if "archi\\tecture.md" in e]
        self.assertEqual(len(found), 2)

    def test_footnotes_are_not_links(self):
        self.write("README.md", "Text.[^1]\n\n[^1]: This is explanatory text.\n")
        self.assertEqual(self.errors(), [])

    def test_anchors_follow_heading_slugs(self):
        self.write("README.md", "[ok](docs/architecture.md#call-dispatch)\n")
        self.assertEqual(self.errors(), [])
        self.write("README.md", "[bad](docs/architecture.md#loop-dispatch)\n")
        self.assertTrue(any("no heading for anchor" in e for e in self.errors()))

    def test_links_inside_code_fences_and_urls_are_ignored(self):
        self.write(
            "README.md",
            "[site](https://example.com/x.md)\n\n```\n[x](nowhere.md)\n```\n",
        )
        self.assertEqual(self.errors(), [])

    def test_an_active_task_needs_every_resume_field(self):
        self.write("tasks/T001-example.md", RESUME.replace("- Evidence:", "- Proof:"))
        self.assertTrue(any("missing Evidence" in e for e in self.errors()))

    def test_resume_fields_must_not_be_empty(self):
        self.write(
            "tasks/T001-example.md",
            RESUME.replace("- Unresolved: none", "- Unresolved:"),
        )
        self.assertTrue(any("leaves Unresolved empty" in e for e in self.errors()))

    def test_a_resume_value_may_continue_on_indented_lines(self):
        self.write(
            "tasks/T001-example.md",
            RESUME.replace(
                "- Unresolved: none", "- Unresolved:\n  the wide tier still exits"
            ),
        )
        self.assertEqual(self.errors(), [])

    def test_a_task_status_must_be_a_known_value(self):
        self.write(
            "tasks/T001-example.md",
            RESUME.replace("Status: active", "Status: mostly done"),
        )
        self.assertTrue(any("Status must start with" in e for e in self.errors()))

    def test_every_task_file_is_listed_in_the_index(self):
        self.write("tasks/T002-other.md", RESUME)
        self.assertTrue(any("does not list tasks/T002-other.md" in e for e in self.errors()))

    def test_every_frozen_plan_is_listed_in_the_unit_index(self):
        self.write("tasks/performance-units/README.md", "# Plans\n\n- listed\n")
        self.write("tasks/performance-units/listed.json", "{}\n")
        self.assertEqual(self.errors(), [])
        self.write("tasks/performance-units/unlisted.json", "{}\n")
        self.assertTrue(any("does not list unlisted.json" in e for e in self.errors()))

    def test_a_longer_name_does_not_stand_in_for_a_plan_or_task(self):
        self.write("tasks/performance-units/README.md", "# Plans\n\n- wide-followup\n")
        self.write("tasks/performance-units/wide.json", "{}\n")
        self.assertTrue(any("does not list wide.json" in e for e in self.errors()))
        for stand_in in ("archive/XT001-example.md", "T001-example.md.old.md"):
            self.write("tasks/README.md", f"# Tasks\n\n- {stand_in}\n")
            self.assertTrue(
                any("does not list tasks/T001-example.md" in e for e in self.errors())
            )
        self.write("tasks/README.md", "# Tasks\n\nSee T001-example.md.\n")
        for entry in ("`wide`.", "[Plan](wide.json)", "`wide.json`"):
            self.write("tasks/performance-units/README.md", f"# Plans\n\n{entry}\n")
            self.assertEqual(self.errors(), [], entry)


if __name__ == "__main__":
    unittest.main()
