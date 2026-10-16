"""Exercise scan ownership and failed relinks without collecting timings."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


class LayoutScanTests(unittest.TestCase):
    def run_scan(self, order_name, stable):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            for name in ("layout-scan.sh", "lib.sh"):
                shutil.copyfile(ROOT / "scripts" / name, scripts / name)
            orders = root / "crates/qjs-cli"
            orders.mkdir(parents=True)
            arm = orders / "hot-functions.order"
            x86 = orders / "hot-functions-x86_64.order"
            arm.write_text("original arm\n")
            x86.write_text("original x86\n")
            selected = orders / order_name
            fake_bin = root / "bin"
            fake_bin.mkdir()
            python = fake_bin / "python3"
            python.write_text(f"#!{sys.executable}\n" + '''
import os
from pathlib import Path
import sys
args = sys.argv[1:]
if args[0] == "-c":
    print("0x9a0" if "executor_offsets" in args[1] else os.environ["SCAN_ORDER"])
elif args[1] == "tools.benchmark.build_cache":
    target = Path(args[args.index("--target-dir") + 1]) / "test-target/release/qjs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("binary")
    target.chmod(0o755)
    print(target)
elif args[1] == "tools.benchmark.layout_pin":
    order = Path(args[args.index("--order") + 1])
    assert order == Path(os.environ["SCAN_ORDER"])
    order.write_text("pinned\\n" if os.environ["SCAN_STABLE"] == "1"
                     else order.read_text() + "growing\\n")
elif args[1] == "tools.benchmark.screen":
    Path("screen-called").touch()
else:
    raise AssertionError(args)
''')
            python.chmod(0o755)
            cargo = fake_bin / "cargo"
            cargo.write_text("#!/bin/sh\nmkdir -p target/release\nprintf binary > target/release/qjs\n")
            cargo.chmod(0o755)
            env = dict(os.environ, PATH=str(fake_bin) + os.pathsep + os.environ["PATH"],
                       CARGO=str(cargo), SCAN_ORDER=str(selected),
                       SCAN_STABLE="1" if stable else "0")
            result = subprocess.run(["bash", str(scripts / "layout-scan.sh"),
                                     "--offsets", "0x0"], cwd=root, env=env,
                                    capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0 if stable else 1, result.stderr)
            self.assertEqual(arm.read_text(), "original arm\n")
            self.assertEqual(x86.read_text(), "original x86\n")
            self.assertEqual((root / "screen-called").exists(), stable)
            self.assertEqual((root / "target/layout-scan/qjs-0x0").exists(), stable)
            if not stable:
                self.assertIn("no screen was run", result.stderr)

    def test_native_order_is_restored(self):
        for name in ("hot-functions.order", "hot-functions-x86_64.order"):
            with self.subTest(order=name):
                self.run_scan(name, stable=True)

    def test_unstable_layout_never_enters_screen(self):
        self.run_scan("hot-functions-x86_64.order", stable=False)
