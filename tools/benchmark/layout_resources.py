"""Retain authenticated resource costs for instruction-blind formal decisions."""
from __future__ import annotations

import json
from pathlib import Path

from . import host_manifest
from .layout_identity import ROOT, digest, same_node
from .resource_schema import load_resource_manifest
from .resource_analysis_schema import load_resource_analysis
from .resource_validation import validate_resource_run
from .resource_artifact import build_resource_report

RESOURCE_MANIFEST = Path('benchmarks/resources.json')
RESOURCE_ANALYSIS = Path('benchmarks/resource-analysis.json')


def resource_manifest_for(report, policy, root=ROOT, facts=None):
    """The qualified resource manifest a report names.

    The frozen manifest is used whenever the evidence names it, so existing
    arm64 evidence verifies byte for byte. Otherwise the host's manifest is
    derived from the policy host and the live recipe, and the evidence must
    name exactly those bytes: a run from a different host or toolchain is not
    interchangeable with this one.
    """
    frozen = load_resource_manifest(root / RESOURCE_MANIFEST)
    named = report['measurement_contract']['manifest_sha256']
    if named == frozen.sha256:
        return frozen
    host = host_manifest.host_platform(
        policy['host']['system'], policy['host']['machine'])
    try:
        path = host_manifest.resource_manifests(root, host, facts)[0]
    except host_manifest.HostManifestError as error:
        raise ValueError(
            f'cannot derive the resource manifest for {host}: {error}; '
            'run this decision on the host that produced the evidence') from error
    manifest = load_resource_manifest(path)
    if manifest.sha256 != named:
        raise ValueError(
            f'resource evidence names manifest {named}, not the qualified '
            f'manifest for {host} ({manifest.sha256}); re-derive and remeasure '
            'with the current host recipe')
    return manifest


def verify(report_path, raw_path, policy, engines):
    if report_path is None or raw_path is None:
        return None, ['instruction-blind formal evidence requires resource report and raw samples']
    report = json.loads(report_path.read_text())
    manifest = resource_manifest_for(report, policy)
    analysis = load_resource_analysis(ROOT / RESOURCE_ANALYSIS, manifest)
    validated = validate_resource_run(raw_path, manifest)
    if build_resource_report(validated, manifest, analysis) != report:
        raise ValueError('resource report differs from raw replay')
    if not same_node(report['run']['host']['node'], policy['host_node']):
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
