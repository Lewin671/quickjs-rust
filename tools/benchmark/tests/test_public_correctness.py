import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.benchmark.public_correctness import check_cases, invocations, MODES
from tools.benchmark.screen import CaseSpec


class PublicCorrectnessTests(unittest.TestCase):
    def test_counts_use_manifest_contract_and_cap_doubling(self):
        case = CaseSpec('broad/example', 'internal', Path('not-read'), 'example')
        contract = SimpleNamespace(initial_iterations=4, max_iterations=6,
                                   expected_operations=lambda n: n * 2,
                                   expected_checksum=lambda n: n * (n + 1) // 2)
        rows = invocations(case, {case.id: contract})
        self.assertEqual([n for n, _ in rows], [4, 6])
        self.assertEqual(json.loads(rows[1][1].split(' ', 1)[1])['checksum'], 21)

    def test_all_modes_check_outputs_and_retain_nonzero_wrong_and_timeout_failures(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / 'binary'; binary.write_text('binary')
            workload = root / 'program.js'; workload.write_text('program')
            ids = ['external/suite/ok', 'development/pkg/wrong', 'external/suite/throw',
                   'external/suite/timeout']
            cases = [CaseSpec(name, 'whole', workload, expected_stdout='ok\n') for name in ids]
            calls = []

            def run(argv, **kwargs):
                index = len(calls) % 4
                calls.append((argv, kwargs['env']))
                if index == 3:
                    raise subprocess.TimeoutExpired(argv, 120)
                return subprocess.CompletedProcess(argv, int(index == 2),
                                                   'wrong\n' if index == 1 else 'ok\n', 'error')

            with patch.dict(os.environ, {'QJS_FORCE_EVAL_EXPOSURE': '1', 'QJS_DISABLE_IC': 'stale'}):
                result = check_cases(cases, {}, {m: binary for m in MODES}, root, runner=run)
            self.assertEqual((result['passed'], result['failed'], result['invocations']), (3, 9, 12))
            self.assertEqual(len((root / 'cases.jsonl').read_text().splitlines()), 12)
            self.assertNotIn('QJS_DISABLE_IC', calls[0][1])
            self.assertNotIn('QJS_FORCE_EVAL_EXPOSURE', calls[0][1])
            self.assertEqual(calls[4][1]['QJS_DISABLE_IC'], '1')
            self.assertEqual(calls[8][1]['QJS_VERIFY_IC'], '1')
            self.assertFalse(json.loads((root / 'request.json').read_text())['timing'])

    def test_held_out_and_missing_expected_outputs_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'only public'):
                check_cases([CaseSpec('held-out/pkg/case', 'whole', root / 'never-read')], {}, {}, root)
            with self.assertRaisesRegex(ValueError, 'missing expected'):
                invocations(CaseSpec('external/suite/case', 'whole', root / 'never-read'), {})
