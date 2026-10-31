"""Recreate only the ten public development bundles for the counter receipt."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from tools.benchmark import development
from tools.benchmark.external_preview import _bundle_source

manifest = development.load_development_manifest(ROOT / 'benchmarks/development-preview.json')
assert manifest.sha256 == '2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13'
expected = {row['id']: row for row in json.loads(Path(__file__).with_name('base-counts.json').read_text())['cases']}
cache = ROOT / 'target/s3b-development-cache'
development.fetch_sources(manifest, cache)
programs = ROOT / 'target/s3b-development-programs'
programs.mkdir(parents=True, exist_ok=True)
rows = []
for suite in manifest.suites:
    for case in suite.cases:
        identity = f'development/{suite.id}/{case.id}'
        source = _bundle_source(suite, case, cache)
        digest = hashlib.sha256(source.encode()).hexdigest()
        assert digest == expected[identity]['sha256'], identity
        path = programs / (identity.replace('/', '--') + '.js')
        path.write_text(source)
        rows.append({'id': identity, 'path': str(path.relative_to(ROOT)), 'sha256': digest, 'expected': expected[identity]['expected']})
(ROOT / 'target/s3b-development-inputs.json').write_text(json.dumps({'manifest_sha256': manifest.sha256, 'cases': rows}, indent=2) + '\n')
print('verified ten development input hashes; no held-out inputs resolved')
