"""Complete protected-body decoding, tables and address operand attribution.

Only encoded disp32 RIP operands and E8 call fixups can vary. Internal branch
encodings are protected. Every byte belongs to an instruction, an owned table
or checked tail alignment; unsupported decoder/control-flow shapes fail closed.
"""
import hashlib
import re
import struct

RIP = re.compile(r'(-?0x[0-9a-f]+|0)\(%rip\)')
# Memory operand sizes, not destination register sizes (notably conversions).
WIDTHS = {
    **dict.fromkeys(('movsd', 'addsd', 'subsd', 'mulsd', 'divsd', 'ucomisd',
                     'comisd', 'sqrtsd', 'cvttsd2si', 'cvtsd2ss', 'cmpeqsd',
                     'cmpltsd', 'cmpneqsd', 'minsd', 'maxsd'), 8),
    **dict.fromkeys(('movss', 'addss', 'subss', 'mulss', 'divss', 'ucomiss',
                     'comiss', 'sqrtss', 'cvttss2si', 'cvtss2sd'), 4),
    **dict.fromkeys(('movapd', 'movaps', 'movupd', 'movups', 'movdqa', 'movdqu',
                     'andpd', 'andps', 'orpd', 'orps', 'xorpd', 'xorps', 'pxor',
                     'blendvpd'), 16),
    'movzbl': 1, 'movzbq': 1, 'movsbl': 1, 'movsbq': 1,
    'movzwl': 2, 'movzwq': 2, 'movswl': 2, 'movswq': 2, 'movslq': 4,
}
BASES = ('mov', 'lea', 'add', 'sub', 'adc', 'sbb', 'and', 'or', 'xor', 'cmp',
         'test', 'imul', 'mul', 'idiv', 'div', 'neg', 'not', 'inc', 'dec',
         'shl', 'shr', 'sar', 'sal', 'rol', 'ror', 'bt', 'bts', 'btr', 'bswap',
         'push', 'pop', 'xchg', 'cmpxchg', 'xadd', 'nop')
CONDITIONS = ('a', 'ae', 'b', 'be', 'e', 'ne', 'g', 'ge', 'l', 'le', 'o', 'no',
              'p', 'np', 's', 'ns', 'z', 'nz', 'c', 'nc')
OPS = ({b+s for b in BASES for s in ('', 'b', 'w', 'l', 'q')} | set(WIDTHS) |
       {'j'+c for c in CONDITIONS} | {'set'+c for c in CONDITIONS} |
       {'cmov'+c+s for c in CONDITIONS for s in ('w', 'l', 'q')} |
       {'jmp', 'jmpq', 'callq', 'retq', 'ud2', 'int3', 'leave', 'leaveq',
        'cltq', 'cltd', 'cqto', 'movabsq', 'cvtsi2sd', 'cvtsi2sdl', 'cvtsi2sdq',
        'cvttsd2sil', 'cvttsd2siq', 'cvttss2sil', 'cvttss2siq', 'roundsd'})


def rip_position(raw, displacement):
    """Read prefixes/opcode/ModRM; never locate a disp32 by byte search."""
    i = 0
    while i < len(raw) and (raw[i] in (0x66, 0xf2, 0xf3) or 0x40 <= raw[i] <= 0x4f):
        i += 1
    if i >= len(raw):
        raise ValueError('truncated RIP instruction')
    op = raw[i]
    i += 1
    if op == 0x0f:
        if i >= len(raw):
            raise ValueError('truncated two-byte opcode')
        second = raw[i]
        i += 1
        if second not in {0x10, 0x11, 0x28, 0x29, 0x2a, 0x2c, 0x2e, 0x2f,
                          0x51, 0x54, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5c,
                          0x5d, 0x5e, 0x5f, 0x6f, 0x7f, 0xaf, 0xb6, 0xb7,
                          0xbe, 0xbf, 0xc2}:
            raise ValueError('unknown RIP opcode')
    elif op not in {0x03, 0x0b, 0x13, 0x1b, 0x23, 0x2b, 0x33, 0x39, 0x3b,
                    0x80, 0x81, 0x83, 0x85, 0x88, 0x89, 0x8a, 0x8b, 0x8d,
                    0xc6, 0xc7, 0xf6, 0xf7, 0xff}:
        raise ValueError('unknown RIP opcode')
    if i+5 > len(raw) or raw[i] & 0xc7 != 0x05:
        raise ValueError('RIP operand does not match encoded ModRM')
    position = i+1
    if struct.unpack_from('<i', raw, position)[0] != displacement:
        raise ValueError('RIP displacement disagrees with decoder')
    return position


def memory_width(op):
    if op in WIDTHS:
        return WIDTHS[op]
    if op in ('callq', 'jmpq'):
        return 8
    if op[:-1] in BASES and op[-1] in 'bwlq':
        return {'b': 1, 'w': 2, 'l': 4, 'q': 8}[op[-1]]
    raise ValueError(f'unknown RIP memory width: {op}')


def decode_body(image, allrows, entry, extent):
    end = entry+extent
    ranges = [(a, n, tag) for a, n, tag in image.tables if a < end and a+n > entry]
    if any(a < entry or a+n > end or tag != 4 or n % 4 for a, n, tag in ranges):
        raise ValueError('unknown/overlapping owned table relocation')
    if any(slot < a+n and slot+8 > a for slot in image.pointer_fixups for a, n, _ in ranges):
        raise ValueError('owned tables cannot contain dyld pointer fixups')
    # Linear otool decoding crosses inline table data and loses alignment.
    # The final table may be followed only by verified canonical NOPs; do
    # not trust the accidental instructions decoded across that boundary.
    tail = max((a+n for a, n, _ in ranges), default=end)
    padding = []
    if tail < end:
        raw = image.raw(tail, end-tail)
        nops = ('90', '6690', '0f1f00', '0f1f4000', '0f1f440000',
                '660f1f440000', '0f1f8000000000', '0f1f840000000000',
                '660f1f840000000000')
        remaining = raw
        while remaining:
            nop = next((bytes.fromhex(n) for n in reversed(nops)
                        if remaining.startswith(bytes.fromhex(n))), None)
            if nop is None or len(raw) > 15:
                raise ValueError('unknown tail alignment')
            remaining = remaining[len(nop):]
        padding = [tail-entry, raw.hex()]
    rows = [r for r in allrows if entry <= r[0] < tail and
            not any(a <= r[0] < a+n for a, n, _ in ranges)]
    starts = {r[0] for r in rows}
    if entry not in starts:
        raise ValueError('missing body entry')
    tables = []
    for start, length, _ in ranges:
        bases = sorted({pc+len(raw)+int(m[1], 16) for pc, raw, op, args in rows
                        if op == 'leaq' and (m := RIP.search(args)) and
                        start <= pc+len(raw)+int(m[1], 16) < start+length})
        if not bases or bases[0] != start:
            raise ValueError('unowned table bytes')
        for base, stop in zip(bases, [*bases[1:], start+length]):
            raw = image.raw(base, stop-base)
            if len(raw) % 4:
                raise ValueError('unaligned table boundary')
            targets = [base+d for (d,) in struct.iter_unpack('<i', raw)]
            if not targets or not all(t in starts for t in targets):
                raise ValueError('jump-table target is not an instruction boundary')
            tables.append({'offset': base-entry, 'raw': raw.hex(),
                           'targets': [t-entry for t in targets]})
    by_table = {entry+t['offset']: t for t in tables}
    covered = bytearray(extent)

    def cover(at, width):
        off = at-entry
        if off < 0 or off+width > extent or any(covered[off:off+width]):
            raise ValueError('overlapping/out-of-body instruction or table')
        covered[off:off+width] = b'\x01' * width

    for a, n, _ in ranges:
        cover(a, n)
    if padding:
        cover(tail, end-tail)
    instructions = []
    for i, (pc, raw, op, args) in enumerate(rows):
        if not 1 <= len(raw) <= 15 or op not in OPS or '#bad' in args:
            raise ValueError(f'unknown decoder behavior at {pc-entry:#x}: {op} {args}')
        if image.raw(pc, len(raw)) != raw:
            raise ValueError('decoder bytes disagree with executable')
        cover(pc, len(raw))
        row = {'offset': pc-entry, 'length': len(raw), 'raw': raw.hex(),
               'op': op, 'operands': args, 'fixup': None, 'edges': []}
        match = RIP.search(args)
        if '%rip' in args and (not match or len(RIP.findall(args)) != 1):
            raise ValueError('unknown RIP operand syntax')
        if match:
            disp = int(match[1], 16)
            at = rip_position(raw, disp)
            target = pc+len(raw)+disp
            if op == 'leaq' and target in by_table:
                width = len(bytes.fromhex(by_table[target]['raw']))
                mode = 'owned-table'
            elif op == 'leaq':
                width, mode = image.address_span(target), 'address-span'
            else:
                width, mode = memory_width(op), 'memory'
            referent = image.raw(target, width)
            row['fixup'] = {'kind': 'rip', 'position': at, 'size': 4,
                            'width': width, 'mode': mode,
                            'referent_sha256': hashlib.sha256(referent).hexdigest()}
            if mode != 'owned-table':
                snapshot = image.referent(target, width)
                root = snapshot['content']['nodes']['root']
                row['fixup']['referent_sha256'] = snapshot['sha256']
                row['fixup']['non_pointer_sha256'] = root['non_pointer_sha256']
                row['referent_relocations'] = {key: snapshot[key] for key in (
                    'raw_sha256', 'pointer_values_sha256', 'pointer_count')}
            imported = image.import_identity(target)
            if imported is not None:
                row['fixup']['import_identity'] = imported
            row['operands'] = RIP.sub('RIP', args)
        if op in ('callq', 'jmp', 'jmpq') or op.startswith('j'):
            direct = re.fullmatch('0x[0-9a-f]+', args)
            if direct:
                target = int(args, 16)
                if entry <= target < end:
                    if target not in starts:
                        raise ValueError('branch enters table/middle of instruction')
                    row['edges'] = [target-entry]
                    row['operands'] = f'body+{target-entry:#x}'
                elif op == 'callq' and len(raw) == 5 and raw[0] == 0xe8:
                    if pc+5+struct.unpack_from('<i', raw, 1)[0] != target:
                        raise ValueError('call fixup disagrees with decoder')
                    row['fixup'] = {'kind': 'call', 'position': 1, 'size': 4,
                                    'identity': image.identity(target)}
                    row['operands'] = row['fixup']['identity']
                else:
                    raise ValueError('unknown external branch relocation')
            elif op in ('jmp', 'jmpq'):
                # LLVM's table dispatch: LEA base; signed slot; ADD base; JMP.
                if i < 3:
                    raise ValueError('unknown indirect jump')
                lea, load, add = rows[i-3:i]
                lm = RIP.search(lea[3])
                base_reg = lea[3].split(', ')[-1]
                dest = args.removeprefix('*')
                if not (lea[2] == 'leaq' and lm and load[2] == 'movslq' and
                        re.fullmatch(r'\(' + re.escape(base_reg) + r',%\w+,4\), ' + re.escape(dest), load[3]) and
                        add[2:] == ('addq', f'{base_reg}, {dest}') and
                        all(a[0]+len(a[1]) == b[0] for a, b in zip((lea, load, add), (load, add, rows[i])))):
                    raise ValueError('unknown indirect jump/table dataflow')
                table = by_table.get(lea[0]+len(lea[1])+int(lm[1], 16))
                if table is None:
                    raise ValueError('indirect jump has no owned table')
                row['edges'] = table['targets']
            elif op == 'callq' and not match and not re.fullmatch(r'\*%[a-z0-9]+', args):
                raise ValueError('unknown indirect call operand')
            elif op.startswith('j') and op not in ('jmp', 'jmpq'):
                raise ValueError('unknown conditional branch')
        # Record all local edges, including cold/unreachable blocks, and fallthrough.
        nextpc = pc+len(raw)
        if op not in ('jmp', 'jmpq', 'retq', 'ud2', 'int3') and nextpc in starts:
            row['edges'].append(nextpc-entry)
        instructions.append(row)
    if not all(covered):
        raise ValueError('undecoded body bytes')
    return {'extent': extent, 'instructions': instructions, 'tables': tables,
            'padding': padding, 'stack': [[r['offset'], r['op'], r['operands']]
                                        for r in instructions if re.search(r'%[rs]bp|%rsp', r['operands'])]}
