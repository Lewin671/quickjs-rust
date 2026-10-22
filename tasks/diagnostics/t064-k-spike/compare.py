"""Compare unmasked selected hot bytes, relative PCs and emitted table closure."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--input', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
roles = ['r0a', 'r0b', 'h8', 'c8', 'o-r0', 'o-h8', 'o-c8']
reports = {}
for role in roles:
    with (args.output / (role + '.log')).open('w') as log:
        subprocess.run([sys.executable, str(Path(__file__).with_name('decode.py')),
                        str(args.input / role / 'qjs'), str(args.output), '--tag', role],
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    reports[role] = json.loads((args.output / (role + '.hot-paths.json')).read_text())


def fingerprint(rows):
    return hashlib.sha256(bytes.fromhex(''.join(r['raw_hex'] for r in rows))).hexdigest()


def compare(left, right):
    result = {}
    for frame, a in reports[left]['bodies'].items():
        b = reports[right]['bodies'][frame]
        paths = {}
        for name, p in a['paths'].items():
            old, new = p['rows'], b['paths'][name]['rows']
            raw_equal = [r['raw_hex'] for r in old] == [r['raw_hex'] for r in new]
            offsets_equal = [r['offset'] for r in old] == [r['offset'] for r in new]
            referents_equal = [r.get('referent_hex') for r in old] == [r.get('referent_hex') for r in new]
            diffs = []
            for i in range(max(len(old), len(new))):
                l = old[i] if i < len(old) else None
                r = new[i] if i < len(new) else None
                if l is None or r is None or any(l.get(k) != r.get(k) for k in
                        ['offset', 'raw_hex', 'rip_target_offset', 'referent_hex']):
                    diffs.append({'index': i, 'base': l, 'candidate': r})
            paths[name] = {'raw_equal': raw_equal, 'relative_pcs_equal': offsets_equal, 'rip_referents_equal': referents_equal,
                           'counts': [len(old), len(new)],
                           'byte_sha256': [fingerprint(old), fingerprint(new)],
                           'first_difference': diffs[0] if diffs else None}
        def body_sha(role):
            p = args.output / (role + '.' + frame.replace('::', '-') + '.bin')
            return hashlib.sha256(p.read_bytes()).hexdigest()
        result[frame] = {'entries': [a['entry'], b['entry']],
                         'extents': [a['extent'], b['extent']],
                         'stack_reserves': [a['stack_reserve'], b['stack_reserve']],
                         'service_calls': [a['service_calls'],b['service_calls']],
                         'body_sha256': [body_sha(left), body_sha(right)],
                         'whole_body_equal': body_sha(left) == body_sha(right),
                         'tables_equal': a['tables'] == b['tables'],
                         'tables': [a['tables'], b['tables']], 'paths': paths,
                         'strict_selected_invariant': a['tables'] == b['tables'] and
                             a['stack_reserve'] == b['stack_reserve'] and all(
                                 p['raw_equal'] and p['relative_pcs_equal'] and p['rip_referents_equal']
                                 for p in paths.values())}
    return result


pairs = [('r0a', 'r0b'), ('r0a', 'h8'), ('r0a', 'c8'),
         ('o-r0', 'o-h8'), ('o-r0', 'o-c8')]
result = {'base': '06e199fec9e9e51742cdae7633c5dc758c326752',
          'scope': 'selected finite numeric/boolean paths, dispatch/table closure; not full CFG',
          'timing': False, 'attempts_consumed': 0,
          'images': {r: reports[r]['sha256'] for r in roles},
          'sections': {r: reports[r]['sections'] for r in roles},
          'comparisons': {l + ':' + r: compare(l, r) for l, r in pairs}}
result['aa_reproduces'] = all(v['whole_body_equal'] and v['strict_selected_invariant']
                             for v in result['comparisons']['r0a:r0b'].values())
result['outlined_invariant'] = result['aa_reproduces'] and all(
    v['strict_selected_invariant'] for pair in ['o-r0:o-h8', 'o-r0:o-c8']
    for v in result['comparisons'][pair].values())
(args.output / 'comparison.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'aa_reproduces': result['aa_reproduces'],
                  'outlined_invariant': result['outlined_invariant'],
                  'pairs': {k: {f: {'invariant': v['strict_selected_invariant'],
                                   'body_equal': v['whole_body_equal'],
                                   'extents': v['extents'],
                                   'raw_equal': {n: p['raw_equal'] for n,p in v['paths'].items()},
                                   'pcs_equal': {n: p['relative_pcs_equal'] for n,p in v['paths'].items()}}
                               for f,v in d.items()} for k,d in result['comparisons'].items()}}, indent=2))
