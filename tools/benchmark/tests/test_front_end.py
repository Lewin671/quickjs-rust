from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from tools.benchmark import front_end


class FrontEndTests(unittest.TestCase):
    def test_wrap_puts_the_bundle_in_a_function_that_is_never_called(self):
        with tempfile.TemporaryDirectory() as work:
            bundle = Path(work) / "suite--case.js"
            bundle.write_text("var x = 1;\nprint(x);\n", encoding="utf-8")
            wrapped = front_end._wrap(bundle, Path(work)).read_text(encoding="utf-8")
        self.assertTrue(wrapped.startswith("function __qjsFrontEndOnly__() {\n"))
        self.assertIn("var x = 1;\nprint(x);\n", wrapped)
        self.assertTrue(wrapped.endswith("\n}\n1;\n"))

    def test_only_the_reference_runs_as_a_script(self):
        engine, reference = Path("/bin/qjs"), Path("/bin/ng")
        self.assertEqual(front_end._flags(engine, reference), ["--raw"])
        self.assertEqual(front_end._flags(reference, reference), ["--script"])
        self.assertEqual(front_end._flags(engine, None), ["--raw"])

    def test_exactly_one_comparison_is_required(self):
        self.assertEqual(front_end.main(["--binary", "/bin/qjs"]), 2)
        self.assertEqual(
            front_end.main(["--binary", "/bin/qjs", "--reference", "/a", "--base", "/b"]), 2
        )


if __name__ == "__main__":
    unittest.main()
