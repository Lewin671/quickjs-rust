"""K0 address-operand contract, final-linked x86_64 Darwin artifacts only.

Reports permit displacement-only RIP changes after referent checks and E8
call/import fixups with equal structural identities. No normalized mnemonic
comparison can turn changed instruction boundaries/non-address bytes into a pass.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from .kernel_decode import decode_body
from .kernel_macho import Image, ROLES, apple_tool_identity, assembly, select_roles
from .kernel_referents import CONTRACT


def fingerprint(binary, output=None):
    image = Image(binary)
    family, roles, adapters = select_roles(image.symbols)
    image.protected_bodies = [(image.symbols.addresses[s],
                               image.symbols.addresses[s]+image.symbols.sizes[s])
                              for s in roles.values()]
    decoder = apple_tool_identity('otool')
    listing, rows = assembly(binary, decoder['path'])
    result = {'schema': 3, 'referent_contract': CONTRACT.copy(), 'target': 'x86_64-apple-darwin', 'family': family,
              'sha256': hashlib.sha256(image.data).hexdigest(), 'adapters': adapters,
              'decoder': decoder, 'roles': {}}
    for role, symbol in roles.items():
        entry, extent = image.symbols.addresses[symbol], image.symbols.sizes[symbol]
        result['roles'][role] = {'entry': entry, 'symbol': symbol,
                                 **decode_body(image, rows, entry, extent)}
    if output:
        output.with_suffix('.asm').write_text(listing)
        output.write_text(json.dumps(result, indent=2) + '\n')
    return result


def non_address(row):
    raw = bytearray.fromhex(row['raw'])
    fixup = row['fixup']
    if fixup:
        if fixup['kind'] not in ('rip', 'call') or fixup['size'] != 4:
            raise ValueError('unknown relocation')
        at = fixup['position']
        if at < 1 or at+4 > len(raw):
            raise ValueError('invalid relocation position')
        raw[at:at+4] = b'\0' * 4
    return raw


def compare(left, right):
    """Compare validated fingerprints; return all role-level failures."""
    for report in (left, right):
        if (report['schema'] != 3 or report.get('referent_contract') != CONTRACT or
                report['target'] != 'x86_64-apple-darwin'):
            raise ValueError('unknown artifact schema/target')
        if set(report['roles']) != set(ROLES):
            raise ValueError('missing/extra protected role')
    if (left['family'], left['decoder'], left['adapters']) != (right['family'], right['decoder'], right['adapters']):
        raise ValueError('executor family or decoder identity changed')
    result = {'pass': True, 'roles': {}}
    for role, a in left['roles'].items():
        b = right['roles'][role]
        failures, allowed = [], []
        for key in ('extent', 'tables', 'padding', 'stack'):
            if a[key] != b[key]:
                failures.append(key + ' changed')
        old, new = a['instructions'], b['instructions']
        boundaries = lambda rows: [(r['offset'], r['length']) for r in rows]
        if boundaries(old) != boundaries(new):
            failures.append('instruction boundaries / relative PCs changed')
        else:
            for l, r in zip(old, new):
                # Validate even unchanged bytes: unknown fixups never pass A/A.
                lb, rb = non_address(l), non_address(r)
                reasons = []
                if (l['op'], l['operands'], l['edges'], lb) != (r['op'], r['operands'], r['edges'], rb):
                    reasons.append('non-address bytes / CFG changed')
                if l['fixup'] != r['fixup']:
                    lf, rf = l['fixup'], r['fixup']
                    if (lf and rf and lf['kind'] == rf['kind'] == 'rip' and
                            lf['width'] == rf['width'] and
                            'non_pointer_sha256' in lf and 'non_pointer_sha256' in rf):
                        if lf['non_pointer_sha256'] != rf['non_pointer_sha256']:
                            reasons.append('RIP non-pointer referent bytes changed')
                        elif lf['referent_sha256'] != rf['referent_sha256']:
                            reasons.append('RIP pointer target identity/content changed')
                        else:
                            reasons.append('RIP import identity or mode changed')
                    else:
                        reasons.append('RIP referent/width or call identity changed')
                if reasons:
                    failures.append(f"+{l['offset']:#x}: " + '; '.join(reasons))
                elif l['raw'] != r['raw'] or l.get('referent_relocations') != r.get('referent_relocations'):
                    changes = []
                    if l['raw'] != r['raw']:
                        changes.append('instruction-displacement')
                    if l.get('referent_relocations') != r.get('referent_relocations'):
                        changes.append('referent-pointer-relocation')
                    allowed.append({'offset': l['offset'], 'kind': l['fixup']['kind'],
                                    'class': 'allowed-displacement', 'changes': changes,
                                    'base': l['raw'], 'candidate': r['raw'],
                                    'verification': l['fixup']})
        result['roles'][role] = {'pass': not failures, 'failures': failures,
                                 'allowed_fixups': allowed,
                                 'instructions': [len(old), len(new)]}
        result['pass'] &= not failures
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference', type=Path)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    try:
        left = fingerprint(args.reference, args.output / 'reference.json')
        right = fingerprint(args.candidate, args.output / 'candidate.json')
        result = compare(left, right)
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        result = {'pass': False, 'error': str(error)}
    (args.output / 'comparison.json').write_text(json.dumps(result, indent=2) + '\n')
    print('K0 PASS' if result['pass'] else 'K0 FAIL', args.output / 'comparison.json')
    if 'error' in result:
        print(result['error'])
    return 0 if result['pass'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
