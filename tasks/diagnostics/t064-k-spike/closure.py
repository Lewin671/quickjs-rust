"""Attribute every outlined-body difference; normalization never grants a pass."""
import argparse
import collections
import json
import re
import struct
import subprocess
from pathlib import Path

from tools.benchmark.layout_symbols import parse_symbol
from tools.benchmark.layout_variant_audit import sections

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--input', type=Path, required=True)
p.add_argument('--audit', type=Path, required=True)
a = p.parse_args()


def image(role, frame):
    binary = a.input / role / 'qjs'
    report = json.loads((a.audit / (role + '.hot-paths.json')).read_text())
    body = report['bodies'][frame]
    entry = int(body['entry'], 16)
    code = (a.audit / (role + '.' + frame.replace('::', '-') + '.bin')).read_bytes()
    seq = []
    for line in (a.audit / (role + '.asm')).read_text().splitlines():
        m = re.match(r'^([0-9a-f]{16})\s+(\S+)\s*(.*)', line)
        if m:
            seq.append((int(m[1], 16), m[2], m[3].split('##')[0].strip()))
    rows = {}
    for i, (pc, op, args) in enumerate(seq[:-1]):
        off = pc-entry
        if 0 <= off < len(code):
            rows[off] = (op, args, code[off:seq[i+1][0]-entry])
    symbols = {}
    for line in (a.audit / (role + '.nm')).read_text().splitlines():
        parts = line.split()
        if len(parts) == 3 and re.fullmatch('[0-9a-f]+', parts[0]):
            parsed = parse_symbol(parts[2])
            symbols[int(parts[0], 16)] = parsed.canonical if parsed else parts[2]
    # Calls into Mach-O import stubs have no ordinary text nm symbol.
    indirect = subprocess.check_output(['otool', '-Iv', str(binary)], text=True)
    for line in indirect.splitlines():
        m = re.match(r'(0x[0-9a-f]+)\s+\d+\s+(\S+)$', line)
        if m:
            symbols[int(m[1], 16)] = m[2]
    return rows, symbols, sections(binary), entry, binary.read_bytes()


result = {}
for candidate in ['o-h8', 'o-c8']:
    result[candidate] = {}
    for frame in ['WideLoopFrame', 'vm::Vm']:
        left, names, segments, entry, data = image('o-r0', frame)
        right, other_names, other_segments, other_entry, other_data = image(candidate, frame)
        assert left.keys() == right.keys(), 'instruction boundaries moved'
        differences = []
        for off, (op, operands, raw) in left.items():
            other_op, other_operands, other_raw = right[off]
            if raw == other_raw:
                continue
            assert op == other_op and len(raw) == len(other_raw)
            m = re.search(r'(-?0x[0-9a-f]+)\(%rip\)', operands)
            if m:
                displacement = int(m[1], 16)
                needle = struct.pack('<i', displacement)
                assert raw.count(needle) == 1
                pos = raw.index(needle)
                assert raw[:pos] == other_raw[:pos] and raw[pos+4:] == other_raw[pos+4:]
                target = entry+off+len(raw)+displacement
                segment = next(s for s in segments if s[0] <= target < s[0]+s[1])
                kind, identity = 'rip', segment[2] + '+' + hex(target-segment[0])
                other_displacement = int(re.search(r'(-?0x[0-9a-f]+)\(%rip\)',other_operands)[1],16)
                if segment[2] == '__TEXT/__const':
                    other_target = other_entry+off+len(other_raw)+other_displacement
                    other_segment = next(s for s in other_segments if s[0] <= other_target < s[0]+s[1])
                    assert other_segment[2] == segment[2] and target-segment[0] == other_target-other_segment[0]
                    width = 16 if op in ['movapd','movaps'] else 8
                    index = segment[3]+target-segment[0]
                    other_index = other_segment[3]+other_target-other_segment[0]
                    assert data[index:index+width] == other_data[other_index:other_index+width]
            else:
                assert op == 'callq' and raw[0] == other_raw[0] == 0xe8
                target = int(operands, 16)
                identity = names[target]
                assert other_names[int(other_operands, 16)] == identity
                kind = 'call'
            differences.append({'offset': hex(off), 'kind': kind, 'target': identity,
                                'base': [operands, raw.hex()],
                                'candidate': [other_operands, other_raw.hex()]})
        result[candidate][frame] = {
            'all_instruction_boundaries_equal': True,
            'all_non_address_bytes_equal': True,
            'differences': differences,
            'histogram': dict(collections.Counter((d['target'] if d['kind'] == 'call'
                               else d['target'].split('+')[0]) for d in differences))}
(a.audit / 'closure.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({c: {f: {'differences': len(v['differences']), 'histogram': v['histogram']}
                     for f,v in d.items()} for c,d in result.items()}, indent=2))
