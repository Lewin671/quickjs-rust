"""Retain authenticated resource costs for instruction-blind formal decisions."""
from __future__ import annotations

import json
from pathlib import Path

from .layout_identity import ROOT, digest
from .resource_schema import load_resource_manifest
from .resource_analysis_schema import load_resource_analysis
from .resource_validation import validate_resource_run
from .resource_artifact import build_resource_report


def verify(report_path, raw_path, policy, engines):
    if report_path is None or raw_path is None:
        return None, ['instruction-blind formal evidence requires resource report and raw samples']
    manifest = load_resource_manifest(ROOT / 'benchmarks/resources.json')
    analysis = load_resource_analysis(ROOT / 'benchmarks/resource-analysis.json', manifest)
    validated = validate_resource_run(raw_path, manifest)
    report = json.loads(report_path.read_text())
    if build_resource_report(validated, manifest, analysis) != report:
        raise ValueError('resource report differs from raw replay')
    if report['run']['host']['node'] != policy['host_node']:
        raise ValueError('resource evidence host differs from policy')
    actual = {e['role']: e for e in report['run']['engines']}
    for role in ('candidate', 'base'):
        engine = actual[role]
        if (engine['binary_sha256'] != engines[role]['binary_sha256']
                or engine['receipt']['source']['revision'] != engines[role]['source_revision']):
            raise ValueError('resource evidence engine differs from formal bundle')
    missing = []
    if not report['coverage']['comparison_input_complete'] or report['coverage']['invalid_blocks']:
        missing.append('resource evidence incomplete')
    return {'report_sha256': digest(report_path), 'raw_sha256': digest(raw_path),
            'report': report}, missing
