"""Extract conserved main-thread self-PC counts and map executor PCs to DWARF.

Usage: analyze.py PROFILE_OUT DEBUG_BINARY DEBUG_DWARF NG_BINARY NG_DWARF OUTPUT
Optional final arguments: CANONICAL_LINES_BINARY CANONICAL_LINES_DWARF
Requires llvm-objdump and llvm-symbolizer on PATH, plus repository PYTHONPATH.
Canonical-to-debug transfer is allowed only for identical normalized function
instruction sequences; unmatched functions remain explicit. No PC interpolation.
"""
import bisect
from collections import Counter
from functools import lru_cache
import json
from pathlib import Path
import re
import subprocess
import sys

from tools.benchmark.layout_symbols import parse_symbol


def symbols(path):
    result = []
    for line in path.read_text().splitlines():
        match = re.match(r'([0-9a-f]+) [tT] (.+)$', line)
        if match:
            address, raw = int(match[1], 16), match[2]
            parsed = parse_symbol(raw.removeprefix('_')) if raw.startswith('__R') else None
            result.append((address, parsed.canonical if parsed else raw, raw))
    return sorted(result)


def owner(table, address):
    index = bisect.bisect_right(table, (address, chr(0x10ffff), '')) - 1
    return table[index] if index >= 0 else (0, 'unknown', 'unknown')


def parse_sample(path, role):
    text = path.read_text()
    load = int(re.search(r'Load Address:\s+(0x[0-9a-f]+)', text)[1], 16)
    graph = text.split('Call graph:\n', 1)[1]
    nodes, stack = [], []
    for line in graph.splitlines():
        match = re.match(r'^([ +|:!]*)(\d+) (.+)$', line)
        if not match:
            if nodes:
                break
            continue
        depth, count, rest = len(match[1]), int(match[2]), match[3]
        if nodes and depth <= nodes[0]['depth']:
            break  # first/main thread only
        node = {'depth': depth, 'count': count, 'self': count, 'text': rest}
        while stack and nodes[stack[-1]]['depth'] >= depth:
            stack.pop()
        if stack:
            nodes[stack[-1]]['self'] -= count
        stack.append(len(nodes))
        nodes.append(node)
    assert nodes and all(n['self'] >= 0 for n in nodes)
    assert sum(n['self'] for n in nodes) == nodes[0]['count']
    counts, ambiguous = Counter(), 0
    for node in nodes:
        if not node['self'] or f'(in {role}-stripped)' not in node['text']:
            continue
        addresses = re.findall(r'0x[0-9a-f]+', node['text'].rsplit('[', 1)[-1])
        if len(addresses) != 1 or '...' in node['text']:
            ambiguous += node['self']
        else:
            counts[int(addresses[0], 16) - load + 0x100000000] += node['self']
    return nodes[0]['count'], counts, ambiguous


def executor(name):
    if 'compact_fn::wide::activation::run' in name and 'run_typed' not in name:
        return 'wide'
    if 'typed_loop::execute::run' in name:
        return 'typed-wide' if 'WideLoopFrame' in name else 'typed-vm'
    if 'run_current_activation' in name:
        return 'vm'
    if 'compact_fn::activation::run' in name:
        return 'compact'
    if name == '_JS_CallInternal':
        return 'ng'
    return None


@lru_cache(maxsize=None)
def data_ranges(binary):
    text = subprocess.check_output(['llvm-objdump', '--macho', '--data-in-code', str(binary)], text=True)
    return [(int(m[1],16)+0x100000000, int(m[1],16)+0x100000000+int(m[2]))
            for line in text.splitlines() if (m := re.match(r'^(0x[0-9a-f]+)\s+(\d+) ',line))]


def disassemble(binary, start, end):
    text = subprocess.check_output(['llvm-objdump', '--disassemble', '--no-show-raw-insn',
        f'--start-address={start}', f'--stop-address={end}', str(binary)], text=True)
    rows = []
    for line in text.splitlines():
        match = re.match(r'^([0-9a-f]+):\s+(.+)', line)
        if match:
            address = int(match[1], 16)
            if not any(lo <= address < hi for lo,hi in data_ranges(binary)):
                rows.append((address, match[2].split('##')[0].strip()))
    return rows


def normalized(rows, start, end, table):
    def replace(match):
        address = int(match[0], 16)
        if start <= address < end:
            return f'LOCAL+{address-start:x}'
        target = owner(table, address)
        if target[0] and address - target[0] < 0x20000:
            return f'SYM:{target[1]}+{address-target[0]:x}'
        return 'ADDRESS'
    result = []
    for address, operation in rows:
        operation = re.sub(r'\s*<[^>]*>', '', operation)
        operation = re.sub(r'-?0x[0-9a-f]+(?=\(%rip\))', 'RIP', operation)
        operation = re.sub(r'0x1[0-9a-f]{8}', replace, operation)
        result.append((address-start, operation))
    return result


def symbolize(dwarf, addresses):
    addresses = sorted(set(addresses))
    result = subprocess.run(['llvm-symbolizer', f'--obj={dwarf}', '--inlines', '--output-style=JSON'],
        input='\n'.join(hex(a) for a in addresses) + '\n', text=True, capture_output=True, check=True)
    values = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(values) == len(addresses)
    return {a: v['Symbol'] for a, v in zip(addresses, values)}


def main():
    root, debug_binary, dwarf, ng_binary, ng_dwarf, output = map(Path, sys.argv[1:7])
    lines_binary, lines_dwarf = map(Path, sys.argv[7:9]) if len(sys.argv) == 9 else (debug_binary, dwarf)
    output.mkdir(parents=True, exist_ok=True)
    tables = {role: symbols(root / (role + '-nm.txt')) for role in ('canonical', 'debug', 'ng')}
    # The NG DWARF retains its symbol table, so generate the index beside outputs.
    (output / 'ng-debug-nm.txt').write_bytes(subprocess.check_output(['nm', '-n', str(ng_dwarf)]))
    ng_table = symbols(output / 'ng-debug-nm.txt')
    (output / 'lines-nm.txt').write_bytes(subprocess.check_output(['nm', '-n', str(lines_binary)]))
    lines_table = symbols(output / 'lines-nm.txt')
    debug_by_name = {row[1]: (i, row) for i, row in enumerate(lines_table)}
    ng_by_name = {row[1]: (i, row) for i, row in enumerate(ng_table)}
    mappings, audits, assembly = {}, [], {}
    for role in ('canonical', 'debug', 'ng'):
        table = tables[role]
        for i, (start, name, _) in enumerate(table[:-1]):
            family = executor(name)
            if not family:
                continue
            end = table[i+1][0]
            rows = disassemble(root / (role + '-stripped'), start, end)
            assembly[(role, name)] = dict(rows)
            if role == 'debug':
                mappings[(role, name)] = (start, dwarf, end-start)
                continue
            lookup, target_table, target_dwarf = (ng_by_name, ng_table, ng_dwarf) if role == 'ng' else (debug_by_name, lines_table, lines_dwarf)
            if name not in lookup:
                audits.append({'role': role, 'symbol': name, 'mapped': False, 'reason': 'missing symbol'})
                continue
            j, target = lookup[name]
            target_end = target_table[j+1][0]
            target_binary = ng_binary if role == 'ng' else lines_binary
            target_rows = disassemble(target_binary, target[0], target_end)
            left = normalized(rows, start, end, table)
            right = normalized(target_rows, target[0], target_end, target_table)
            equal = left == right
            prefix = 0
            for lrow, rrow in zip(left, right):
                if lrow != rrow:
                    break
                prefix = lrow[0] + 1
            audits.append({'role': role, 'symbol': name, 'family': family, 'canonical_start': hex(start),
                'debug_start': hex(target[0]), 'canonical_bytes': end-start, 'debug_bytes': target_end-target[0],
                'instructions': len(rows), 'debug_instructions': len(target_rows), 'mapped': equal, 'verified_prefix_bytes': prefix})
            (output / f'{role}-{family}-normalized.txt').write_text('\n'.join(f'{o:x} {v}' for o,v in left)+'\n')
            (output / f'{role}-{family}-debug-normalized.txt').write_text('\n'.join(f'{o:x} {v}' for o,v in right)+'\n')
            mappings[(role, name)] = (target[0], target_dwarf, end-start if equal else prefix)
    data = []
    for invocation in json.loads((root / 'profiles.json').read_text()):
        role = invocation['role']
        total, counts, ambiguous = parse_sample(root / invocation['raw_path'], role)
        rows = []
        for address, count in counts.items():
            start, name, raw = owner(tables[role], address)
            family = executor(name)
            if family:
                mapped = mappings.get((role, name))
                if mapped and address-start >= mapped[2]:
                    mapped = None
                rows.append({'pc': hex(address), 'offset': address-start, 'symbol': name, 'family': family,
                    'count': count, 'mapped_dwarf': str(mapped[1]) if mapped else None, 'debug_pc': hex(mapped[0]+address-start) if mapped else None,
                    'instruction': assembly[(role, name)].get(address, '?')})
        data.append({'id': invocation['id'], 'role': role, 'repeat': invocation['repeat'], 'total': total,
                     'ambiguous_self': ambiguous, 'executors': rows})
    source = {}
    for image in (dwarf, lines_dwarf, ng_dwarf):
        addresses = [int(r['debug_pc'],16) for p in data for r in p['executors'] if r['mapped_dwarf'] == str(image)]
        if addresses and str(image) not in source:
            source[str(image)] = symbolize(image, addresses)
    for profile in data:
        for row in profile['executors']:
            row['frames'] = source[row['mapped_dwarf']].get(int(row['debug_pc'],16), []) if row['debug_pc'] else []
    (output / 'codegen-audit.json').write_text(json.dumps(audits, indent=2)+'\n')
    (output / 'self-pcs.json').write_text(json.dumps(data, indent=2)+'\n')
    print(json.dumps(audits, indent=2))
    print('profiles', len(data), 'ambiguous self', sum(p['ambiguous_self'] for p in data))


if __name__ == '__main__':
    main()
