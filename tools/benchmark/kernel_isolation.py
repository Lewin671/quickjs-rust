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


def fingerprint(binary, output=None):
    image = Image(binary)
    family, roles, adapters = select_roles(image.symbols)
    decoder = apple_tool_identity('otool')
    listing, rows = assembly(binary, decoder['path'])
    result = {'schema': 1, 'target': 'x86_64-apple-darwin', 'family': family,
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
        if report['schema'] != 1 or report['target'] != 'x86_64-apple-darwin':
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
                    reasons.append('RIP referent/width or call identity changed')
                if reasons:
                    failures.append(f"+{l['offset']:#x}: " + '; '.join(reasons))
                elif l['raw'] != r['raw']:
                    allowed.append({'offset': l['offset'], 'kind': l['fixup']['kind'],
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
