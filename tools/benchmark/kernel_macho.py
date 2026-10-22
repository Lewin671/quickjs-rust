"""Final-linked Darwin input for K0. Unknown tables/roles fail closed.

Reuse the spike's section reader and structural v0 parser. LC_DATA_IN_CODE
owns inline table extents; nm owns function extents. Neither a size heuristic
nor a disassembler's accidental decoding of a table selects a protected body.
"""
import bisect
import re
import struct
import subprocess
from collections import Counter

from .layout_symbols import Nested, parse_symbol, unwrap_closures
from .layout_tables import text_symbols
from .layout_variant_audit import sections
from .build_cache_identity import version_identity

ROLES = {'Vm': 'qjs_runtime::bytecode::vm::Vm',
         'WideLoopFrame': 'qjs_runtime::bytecode::compact_fn::wide::loop_frame::WideLoopFrame'}
PREFIX = 'qjs_runtime::bytecode::typed_loop::execute::'


def command(argv):
    return subprocess.check_output([str(a) for a in argv], text=True)


def apple_tool_identity(name):
    """Hash the selected Xcode tool, not /usr/bin's stable dispatch shim."""
    return version_identity(command(['xcrun', '--find', name]).strip())


def select_roles(symbols):
    """Admit original run closures or outlined execute entries, never clones.

    The original build also emits tiny execute closure adapters. Inventory
    them explicitly; they are not dispatch bodies. Outlined entries cannot
    coexist with run dispatch bodies.
    """
    families = {'run': {r: [] for r in ROLES}, 'execute': {r: [] for r in ROLES}}
    adapters = []
    adapter_roles = set()
    for name in symbols.names:
        parsed = parse_symbol(name)
        if parsed is None:
            if 'typed_loop' in name and ('7execute' in name or '3run' in name):
                raise ValueError('undecodable typed executor symbol')
            continue
        for family, inventory in families.items():
            if not parsed.canonical.startswith(PREFIX + family + '::<'):
                continue
            matches = [r for r, typ in ROLES.items() if parsed.canonical == PREFIX + family + '::<' + typ + '>']
            if len(matches) != 1 or parsed.suffix or not symbols.size_of(name):
                raise ValueError('unknown/aliased/cloned typed executor role')
            role = matches[0]
            if family == 'execute' and parsed.wrapped:
                path = parsed.path
                key = (role, path.disambiguator)
                # Audited original-source closure inventory; never infer an
                # adapter from body size. New wrappers need a decoder audit.
                if (not isinstance(path, Nested) or path.parent is not
                        unwrap_closures(path)
                        or key not in {('Vm', 0), ('Vm', 3), ('WideLoopFrame', 0)}
                        or key in adapter_roles):
                    raise ValueError('unrecognized execute closure adapter')
                adapter_roles.add(key)
                adapters.append(name)
            else:
                inventory[role].append(name)
    active = [f for f, inv in families.items() if any(inv.values())]
    if len(active) != 1 or any(len(v) != 1 for v in families[active[0]].values()):
        raise ValueError('missing/duplicate/cloned protected role')
    if active[0] == 'execute' and adapters:
        raise ValueError('unexpected outlined executor adapters')
    return active[0], {r: names[0] for r, names in families[active[0]].items()}, sorted(adapters)


class Image:
    def __init__(self, binary):
        self.data = binary.read_bytes()
        self.sections = sections(binary)
        if struct.unpack_from('<I', self.data, 12)[0] != 2:
            raise ValueError('K0 requires a final linked executable')
        self.symbols = text_symbols(binary)
        self.names = {}
        # Include data symbol boundaries for conservative address-only LEAs.
        for line in command(['nm', '-n', binary]).splitlines():
            parts = line.split()
            if len(parts) == 3 and re.fullmatch('[0-9a-fA-F]+', parts[0]):
                # Exact symbol spelling retains closure/clone identities.
                # Canonical role matching must not erase a callee's wrappers.
                self.names.setdefault(int(parts[0], 16), []).append(
                    parts[2])
        self.name_starts = sorted(self.names)
        self.imports = {}
        for line in command(['otool', '-Iv', binary]).splitlines():
            match = re.fullmatch(r'(0x[0-9a-f]+)\s+\d+\s+(\S+)', line.strip())
            if match:
                self.imports[int(match[1], 16)] = match[2]
        self.tables = self.data_in_code()

    def section(self, address, width=1):
        found = [s for s in self.sections if s[0] <= address and address + width <= s[0] + s[1]]
        if width < 1 or len(found) != 1:
            raise ValueError(f'unmapped/ambiguous referent {address:#x} width {width}')
        return found[0]

    def raw(self, address, width):
        start, _, _, offset = self.section(address, width)
        at = offset + address - start
        if at + width > len(self.data):
            raise ValueError('referent is not file-backed')
        return self.data[at:at+width]

    def identity(self, address):
        if address in self.imports:
            return 'import:' + self.imports[address]
        if address in self.names and len(self.names[address]) == 1:
            return 'symbol:' + self.names[address][0]
        raise ValueError(f'unknown relocation/call target {address:#x}')

    def address_span(self, address):
        """LEA has no dereference width: protect through the next data boundary.

        This conservative upper bound can reject unrelated data movement; it
        never truncates an opaque address to an arbitrary 8-byte prefix.
        """
        section = self.section(address)
        if section[2] == '__TEXT/__text':
            raise ValueError('unowned RIP code address')
        end = section[0] + section[1]
        i = bisect.bisect_right(self.name_starts, address)
        if i < len(self.name_starts):
            end = min(end, self.name_starts[i])
        return end - address

    def data_in_code(self):
        cursor, records = 32, []
        for _ in range(struct.unpack_from('<I', self.data, 16)[0]):
            kind, size = struct.unpack_from('<II', self.data, cursor)
            if kind == 0x19:
                count = struct.unpack_from('<I', self.data, cursor+64)[0]
                for i in range(count):
                    if struct.unpack_from('<I', self.data, cursor+72+i*80+60)[0]:
                        raise ValueError('unknown unresolved section relocation')
            if kind == 0x29:
                if size != 16:
                    raise ValueError('invalid data-in-code command')
                offset, length = struct.unpack_from('<II', self.data, cursor+8)
                if length % 8 or offset + length > len(self.data):
                    raise ValueError('truncated data-in-code')
                for i in range(offset, offset+length, 8):
                    at, width, tag = struct.unpack_from('<IHH', self.data, i)
                    mapped = [s for s in self.sections if s[3] <= at < s[3] + s[1]]
                    if len(mapped) != 1 or not width:
                        raise ValueError('invalid data-in-code range')
                    section = mapped[0]
                    address = section[0] + at - section[3]
                    self.raw(address, width)
                    records.append((address, width, tag))
            cursor += size
        ordered = sorted(records)
        if any(a+n > b for (a, n, _), (b, _, _) in zip(ordered, ordered[1:])):
            raise ValueError('overlapping data-in-code')
        return ordered


def assembly(binary, decoder='otool'):
    """Use LLVM-backed Apple otool, with explicit raw instruction lengths."""
    listing = command([decoder, '-tvj', binary])
    rows = []
    for line in listing.splitlines():
        match = re.match(r'^([0-9a-f]{16})\s+([0-9a-f]+)\s+(\S+)\s*(.*)', line)
        if match:
            pc, raw, op, operands = match.groups()
            rows.append((int(pc, 16), bytes.fromhex(raw), op, operands.split('##')[0].strip()))
        elif re.match(r'^[0-9a-f]{16}\s', line):
            raise ValueError('unknown disassembler row format')
    if not rows or any(n != 1 for n in Counter(r[0] for r in rows).values()):
        raise ValueError('empty/duplicate disassembly')
    return listing, rows
