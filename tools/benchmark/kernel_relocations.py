"""Decode classic Mach-O dyld pointer fixups, without guessing from values.

The opcode definitions are in Apple's SDK mach-o/loader.h (dyld_info_command,
REBASE_OPCODE_* and BIND_OPCODE_*). Only 64-bit pointer fixups are admitted.
Chained/threaded, weak-binding and unknown forms require a new audited decoder.
All offsets/counts and slot ownership are checked before referent traversal.
"""
import struct


class Stream:
    def __init__(self, data):
        self.data, self.at = data, 0

    def byte(self):
        if self.at >= len(self.data):
            raise ValueError('truncated dyld opcode stream')
        value = self.data[self.at]
        self.at += 1
        return value

    def leb(self, signed=False):
        value, shift = 0, 0
        while True:
            byte = self.byte()
            value |= (byte & 0x7f) << shift
            shift += 7
            if shift > 70:
                raise ValueError('overflowing dyld LEB128')
            if not byte & 0x80:
                if signed and byte & 0x40:
                    value -= 1 << shift
                if not (-(1 << 63) <= value < 1 << 63 if signed else 0 <= value < 1 << 64):
                    raise ValueError('overflowing dyld LEB128')
                return value

    def string(self):
        start = self.at
        while self.byte():
            pass
        return self.data[start:self.at-1].decode('utf-8')


def decode_stream(data, segments, libraries, *, bind=False, lazy=False, max_slots=1_000_000):
    """Return slot -> binding or rebase; reject unsupported state/opcodes."""
    stream, result = Stream(data), {}
    segment, offset, typ, ordinal, symbol, addend, flags = None, 0, 1, None, None, 0, 0

    def emit(count=1, skip=0):
        nonlocal offset
        if count+len(result) > max_slots or segment is None or typ != 1:
            raise ValueError('invalid/unsupported dyld pointer state/count')
        base, size = segments[segment]
        for _ in range(count):
            if offset < 0 or offset+8 > size:
                raise ValueError('dyld pointer outside segment')
            address = base+offset
            if address in result:
                raise ValueError('duplicate dyld pointer slot')
            entry = {'kind': 'rebase'}
            if bind:
                if not symbol or ordinal is None or not 1 <= ordinal <= len(libraries):
                    raise ValueError('unresolved dyld import identity')
                entry = {'kind': 'bind', 'library': libraries[ordinal-1],
                         'symbol': symbol, 'addend': addend, 'flags': flags}
            result[address] = entry
            offset += 8+skip

    while stream.at < len(data):
        byte = stream.byte()
        op, imm = byte & 0xf0, byte & 0x0f
        if op == 0:
            if imm:
                raise ValueError('unknown dyld DONE opcode')
            if lazy:
                segment, offset, typ, ordinal, symbol, addend, flags = None, 0, 1, None, None, 0, 0
                continue
            if any(data[stream.at:]):
                raise ValueError('nonzero bytes after dyld DONE')
            return result
        if not bind:
            if op in (0x30, 0x60, 0x70, 0x80) and imm:
                raise ValueError('unknown rebase opcode immediate')
            if op == 0x10:
                typ = imm
                if typ != 1:
                    raise ValueError('unknown rebase relocation type')
            elif op == 0x20:
                segment, offset = imm, stream.leb()
                if segment >= len(segments):
                    raise ValueError('invalid dyld segment index')
            elif op == 0x30:
                offset = (offset+stream.leb()) & ((1 << 64)-1)
            elif op == 0x40:
                offset += imm*8
            elif op == 0x50:
                emit(imm)
            elif op == 0x60:
                emit(stream.leb())
            elif op == 0x70:
                emit(skip=stream.leb())
            elif op == 0x80:
                count, skip = stream.leb(), stream.leb()
                emit(count, skip)
            else:
                raise ValueError('unknown dyld rebase opcode')
        else:
            if op in (0x20, 0x60, 0x80, 0x90, 0xa0, 0xc0) and imm:
                raise ValueError('unknown bind opcode immediate')
            if op == 0x10:
                ordinal = imm
            elif op == 0x20:
                ordinal = stream.leb()
            elif op == 0x30:
                raise ValueError('unsupported special dyld ordinal')
            elif op == 0x40:
                flags, symbol = imm, stream.string()
                if flags & ~1:
                    raise ValueError('unknown dyld symbol flags')
            elif op == 0x50:
                typ = imm
                if typ != 1:
                    raise ValueError('unknown bind relocation type')
            elif op == 0x60:
                addend = stream.leb(signed=True)
            elif op == 0x70:
                segment, offset = imm, stream.leb()
                if segment >= len(segments):
                    raise ValueError('invalid dyld segment index')
            elif op == 0x80:
                # ld64 encodes backwards additions as unsigned 64-bit deltas.
                offset = (offset+stream.leb()) & ((1 << 64)-1)
            elif op == 0x90:
                emit()
            elif op == 0xa0:
                emit(skip=stream.leb())
            elif op == 0xb0:
                emit(skip=imm*8)
            elif op == 0xc0:
                count, skip = stream.leb(), stream.leb()
                emit(count, skip)
            else:
                raise ValueError('unknown/threaded dyld bind opcode')
    if data and not lazy:
        raise ValueError('missing dyld DONE')
    return result


def pointer_fixups(data, sections):
    """Read checked load commands, preserving segment and dylib ordinals."""
    cursor, segments, libraries, info = 32, [], [], None
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        if cursor+8 > len(data):
            raise ValueError('truncated Mach-O fixup command')
        kind, size = struct.unpack_from('<II', data, cursor)
        if size < 8 or cursor+size > len(data):
            raise ValueError('invalid Mach-O fixup command')
        if kind == 0x19:
            if size < 72:
                raise ValueError('truncated dyld segment')
            segments.append(struct.unpack_from('<QQ', data, cursor+24))
        elif kind in (0xc, 0x80000018, 0x8000001f, 0x80000023, 0x20):
            if size < 24:
                raise ValueError('truncated dyld library')
            start = struct.unpack_from('<I', data, cursor+8)[0]
            end = data.find(b'\0', cursor+start, cursor+size)
            if not 24 <= start < size or end < 0:
                raise ValueError('invalid dyld library identity')
            libraries.append(data[cursor+start:end].decode('utf-8'))
        elif kind in (0x22, 0x80000022):
            if size != 48 or info is not None:
                raise ValueError('invalid/duplicate dyld info')
            info = struct.unpack_from('<10I', data, cursor+8)
        elif kind == 0x80000034:
            raise ValueError('unsupported chained dyld fixups')
        cursor += size
    if info is None:
        raise ValueError('missing dyld relocation inventory')
    result = {}
    for index in range(0, 8, 2):
        offset, width = info[index:index+2]
        if index == 4 and width:
            raise ValueError('unsupported weak dyld binding')
        if offset+width > len(data):
            raise ValueError('truncated dyld relocation inventory')
        values = decode_stream(data[offset:offset+width], segments, libraries,
                               bind=index != 0, lazy=index == 6, max_slots=len(data)//8)
        for address, value in values.items():
            matches = [s for s in sections if s[0] <= address and address+8 <= s[0]+s[1]]
            if len(matches) != 1:
                raise ValueError('unmapped dyld pointer slot')
            if address in result and not (index == 6 and result[address]['kind'] == 'rebase'):
                raise ValueError('ambiguous dyld pointer slot')
            result[address] = value
    starts = sorted(result)
    if any(a+8 > b for a, b in zip(starts, starts[1:])):
        raise ValueError('overlapping dyld pointer slots')
    return result
