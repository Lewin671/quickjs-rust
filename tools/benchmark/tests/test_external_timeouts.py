"""Pinned reference hang limits never depend on candidate/base outcomes."""
import copy
import json
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.external_timeouts import derive, validate
from tools.benchmark.external_preview import load_manifest, ExternalPreviewError

ROOT = Path(__file__).resolve().parents[3]


class TimeoutTests(unittest.TestCase):
    def test_manifest_has_complete_frozen_derivation(self):
        data = json.loads((ROOT/'benchmarks/external-preview.json').read_text())
        manifest = load_manifest(ROOT/'benchmarks/external-preview.json')
        policy = data['measurement']['timeout_policy']
        limits = validate(policy,[s.id+'/'+c.id for s in manifest.suites for c in s.cases])
        self.assertEqual(len(limits),45)
        self.assertEqual(set(limits.values()),{60})
        for s in manifest.suites:
            for c in s.cases:
                self.assertEqual(manifest.measurement.timeout_for(s.id,c.id),60)

    def test_derive_uses_only_pinned_complete_ng(self):
        provenance = {'reference_binary_sha256':'a'*64,'blocks':30}
        rows = [{'phase':'measurement','role':'quickjs-ng','status':'ok',
                 'binary_sha256':'a'*64,'suite_id':'s','case_id':'c','block':b,
                 'duration_ns':20_000_000_000} for b in range(30)]
        data = derive(rows,provenance=provenance)
        self.assertEqual(data['cases']['s/c']['timeout_seconds'],80)
        rows += [{'phase':'measurement','role':'candidate','status':'timeout'}]
        self.assertEqual(derive(rows,provenance=provenance),data)
        with self.assertRaises(ValueError):
            derive(rows[1:],provenance=provenance)
        rows[0]['status']='timeout'
        with self.assertRaises(ValueError):
            derive(rows,provenance=provenance)

    def test_derivation_or_inventory_tampering_is_rejected(self):
        policy = json.loads((ROOT/'benchmarks/external-preview.json').read_text())['measurement']['timeout_policy']
        for change in ('limit','median','case','reference'):
            bad = copy.deepcopy(policy)
            key = next(iter(bad['cases']))
            if change=='limit': bad['cases'][key]['timeout_seconds']=59
            if change=='median': bad['cases'][key]['reference_median_ns']=-1
            if change=='case': del bad['cases'][key]
            if change=='reference': bad['reference']['reference_revision']='bad'
            with self.assertRaises(ValueError): validate(bad,policy['cases'])

    def test_runner_uses_each_case_limit_and_timeout_still_fails(self):
        from tools.benchmark.external_preview import PreviewRun, Measurement
        from tools.benchmark.process import ProcessResult
        from types import SimpleNamespace
        run=object.__new__(PreviewRun)
        run.cases=[(SimpleNamespace(id='s'),SimpleNamespace(id='c'),1,Path('/bundle'))]
        run.binaries={'candidate':Path('/candidate')};run.binary_hashes={'candidate':'a'*64}
        run.blocks=30;run.lane=SimpleNamespace(label='test',status=None)
        run.manifest=SimpleNamespace(measurement=Measurement(30,60,'m','p','o',0,{'s/c':88}))
        with patch('tools.benchmark.external_preview.run_process') as process, \
             patch('tools.benchmark.external_preview._record',return_value={}), \
             patch('tools.benchmark.external_preview._print_sample_progress'):
            run._samples(0,'measurement',[['candidate']])
            self.assertEqual(process.call_args.args[1],88)
        from tools.benchmark.external_preview import _sample_status
        result=ProcessResult('now',1,2,1,-9,True,'','',False,False)
        self.assertEqual(_sample_status(result,'candidate')[0],'timeout')


if __name__=='__main__':unittest.main()
