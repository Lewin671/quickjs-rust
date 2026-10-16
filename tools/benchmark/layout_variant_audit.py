"""Static x86_64 Mach-O body/placement audit for the layout panel.

Resolve PC-relative references to symbols/sections so relocation bytes may
change while instructions, targets and symbol-relative offsets remain exact.
Only the last function's trailing section filler may differ.
"""
from __future__ import annotations

import bisect
import json
import re
import struct

from .layout_identity import command, digest

def sections(binary):
    """Read section address ranges from a thin x86_64 Mach-O executable."""
    data = binary.read_bytes()
    if len(data) < 32 or struct.unpack_from('<II', data) != (0xfeedfacf, 0x1000007):
        raise ValueError('layout audit requires a thin x86_64 Mach-O executable')
    cursor, result = 32, []
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        if cursor + 8 > len(data):
            raise ValueError('truncated Mach-O load command')
        kind, size = struct.unpack_from('<II', data, cursor)
        if size < 8 or cursor + size > len(data):
            raise ValueError('invalid Mach-O load command size')
        if kind == 0x19:  # LC_SEGMENT_64
            if size < 72:
                raise ValueError('truncated Mach-O segment')
            count = struct.unpack_from('<I', data, cursor + 64)[0]
            if 72 + count * 80 > size:
                raise ValueError('truncated Mach-O section table')
            for i in range(count):
                section = cursor + 72 + i * 80
                name = data[section:section+16].rstrip(b'\0').decode()
                segment = data[section+16:section+32].rstrip(b'\0').decode()
                address, length, offset = struct.unpack_from('<QQI', data, section+32)
                result.append((address, length, segment + '/' + name, offset))
        cursor += size
    return result


def inspect(binary, out, tag):
    listing = command(['nm', '-n', binary])
    (out / (tag + '.nm')).write_text(listing)
    rows = []
    for line in listing.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[1] in ('t', 'T') and parts[2] != '__mh_execute_header':
            rows.append((int(parts[0], 16), parts[2]))
    addresses = [address for address, _ in rows]
    names = [name for _, name in rows]
    if not rows or len(rows) != len(set(names)) or len(rows) != len(set(addresses)):
        raise ValueError('layout audit needs unique text names and addresses')
    segments = sections(binary)
    text = next((s for s in segments if s[2] == '__TEXT/__text'), None)
    if text is None:
        raise ValueError('layout audit cannot find __TEXT/__text')

    def target(address):
        if text[0] <= address < text[0] + text[1]:
            index = bisect.bisect_right(addresses, address) - 1
            if index >= 0:
                return names[index] + '+' + hex(address - addresses[index])
        for start, size, name, _ in segments:
            if start <= address < start + size:
                return name + '+' + hex(address - start)
        if 0x100000000 <= address < text[0]:
            return 'HEADER+' + hex(address - 0x100000000)
        return None

    assembly = command(['otool', '-tv', binary])
    (out / (tag + '.asm')).write_text(assembly)
    instructions = []
    for line in assembly.splitlines():
        match = re.match(r'^([0-9a-f]{16})\s+([^\s]+)\s*(.*)', line)
        if match:
            instructions.append((int(match[1], 16), match[2], match[3].split('##')[0].strip()))
    if not instructions:
        raise ValueError('layout audit found no disassembled instructions')
    bodies = {name: [] for name in names}
    for i, (address, operation, operands) in enumerate(instructions):
        index = bisect.bisect_right(addresses, address) - 1
        if index < 0:
            continue
        following = instructions[i+1][0] if i+1 < len(instructions) else text[0] + text[1]
        operands = re.sub(r'(-?0x[0-9a-f]+)\(%rip\)',
            lambda m: 'RIP[' + (target(following + int(m[1], 16)) or 'RAW:' + m[1]) + ']', operands)
        branch = operation.startswith(('j', 'call', 'loop'))
        if operation in ('rep', 'repne', 'repe', 'bnd'):
            branch = bool(re.match(r'(?:j\w*|call\w*|loop\w*)\s+', operands))
        if branch:
            operands = re.sub(r'(?<![\w$])0x[0-9a-f]+$',
                lambda m: target(int(m[0], 16)) or 'RAW_REL:' + hex(int(m[0], 16) - following), operands)
        bodies[names[index]].append(operation + ' ' + operands)
    (out / (tag + '.normalized.json')).write_text(json.dumps(bodies, indent=2) + '\n')
    return {'sha256': digest(binary), 'bytes': binary.stat().st_size,
            'addresses': dict(zip(names, addresses)),
            'sizes': {name: end-start for (start, name), (end, _) in zip(rows, rows[1:])},
            'bodies': bodies, 'sections': segments}


def audit(natural, variant, pad):
    original = natural['addresses']
    actual = variant['addresses']
    filtered = {n: a for n, a in actual.items() if n != '_qjs_layout_noise_pad'}
    if list(filtered) != list(original):
        raise ValueError('layout audit: function inventory/order changed')
    shift = actual[next(iter(original))] - original[next(iter(original))]
    if not pad <= shift < pad + 32 or shift % 16:
        raise ValueError('layout audit: invalid uniform shift')
    if any(actual[n] != a + shift for n, a in original.items()):
        raise ValueError('layout audit: nonuniform function displacement')
    def strip_tail(body):
        result = list(body)
        while result and result[-1].startswith(('nop', 'int3')):
            result.pop()
        return result
    last = list(original)[-1]
    for name, body in natural['bodies'].items():
        other = variant['bodies'][name]
        if name == last:
            body, other = strip_tail(body), strip_tail(other)
        if body != other:
            raise ValueError(f'layout audit: body drift in {name}')
    return {'functions': len(original), 'pad_bytes': pad,
            'actual_uniform_shift': shift, 'body_drift': 0, 'address_drift': 0}
