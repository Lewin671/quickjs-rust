"""Run the unchanged comparison fixtures on every exact diagnostic image."""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

matrix, reference, output = map(Path, sys.argv[1:])
assert 'no-timing' in (output.parent.parent / 'status').read_text()
env = {**os.environ, 'TZ': 'UTC'}


def run(binary, args):
    return subprocess.check_output([str(binary), *args], text=True,
                                   stderr=subprocess.STDOUT, timeout=10, env=env).strip()


def normalize(text):
    rules = [(r'^Number\(([0-9]+)\.0\)$', r'\1'),
             (r'^Number\(([-]?[0-9]+(?:\.[0-9]+)?)\)$', r'\1'),
             (r'^String\("(.*)"\)$', r'\1'),
             (r'^Boolean\((true|false)\)$', r'\1'),
             (r'^Null$', 'null'), (r'^Undefined$', 'undefined')]
    for pattern, replacement in rules:
        text = re.sub(pattern, replacement, text, flags=re.M)
    return text


builds = json.loads((matrix / 'builds.json').read_text())
assert set(builds) == {'r0a','r0b','h8','c8','o-r0','o-h8','o-c8'}
for role, record in builds.items():
    sizes = {parts[1]:int(parts[2]) for line in record['layout']
             if (parts := line.split())[0] == 'LAYOUT'}
    expected = (120,400) if role in ['h8','o-h8'] else (112,408) if role in ['c8','o-c8'] else (112,400)
    assert (sizes['ObjectData'],sizes['ObjectColdData']) == expected,(role,sizes)
    if role not in ['h8','o-h8']:
        assert record['layout'][2:] == builds['r0a']['layout'][2:],role

fixtures = sorted(Path('tests/fixtures/compare-qjs').glob('*.js'))
assert fixtures
result = {'timing': False, 'fixture_count': len(fixtures), 'images': {}}
expected = {}
for fixture in fixtures:
    expression = fixture.read_text().replace('\n', ' ')
    expected[fixture.name] = run(reference, ['--script', '-e', 'console.log(' + expression + ')'])
for role in ['r0a', 'r0b', 'h8', 'c8', 'o-r0', 'o-h8', 'o-c8']:
    rows = {}
    for fixture in fixtures:
        actual = normalize(run(matrix / role / 'qjs',
                               ['--raw', '-e', fixture.read_text().replace('\n', ' ')]))
        assert actual == expected[fixture.name], (role, fixture.name, actual, expected[fixture.name])
        rows[fixture.name] = actual
    result['images'][role] = rows
    print('NG_PASS', role, len(rows), flush=True)
output.write_text(json.dumps(result, indent=2) + '\n')
