import json
import tempfile
import unittest
from pathlib import Path

from tools.benchmark.bundle import ARTIFACTS, seal, verify


class BundleTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / "summary.json").write_text('{"state":"success"}\n')
        for name in ARTIFACTS:
            (self.root / name).write_text('{}\n')
        seal(self.root)

    def verify(self):
        return verify(self.root / "summary.json", self.root / "report.json",
                      self.root / "external-report.json", self.root / "sentinel-report.json")

    def test_fresh_sealed_bundle_verifies(self):
        self.verify()

    def test_copied_report_from_another_run_is_detected(self):
        (self.root / "external-report.json").write_text('{"different_run":true}\n')
        with self.assertRaisesRegex(ValueError, "artifact changed"):
            self.verify()

    def test_cross_directory_reports_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "same sealed comparison directory"):
            verify(self.root / "summary.json", self.root / "report.json",
                   self.root.parent / "external-report.json", self.root / "sentinel-report.json")

    def test_cannot_silently_reseal_replaced_evidence(self):
        with self.assertRaisesRegex(ValueError, "already sealed"):
            seal(self.root)

    def test_missing_lane_cannot_be_promoted(self):
        (self.root / "sentinel-report.json").unlink()
        with self.assertRaises(OSError):
            self.verify()
