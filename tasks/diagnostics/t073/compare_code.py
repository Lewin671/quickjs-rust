"""Reproduce the T073 exact-image executor listing comparison.

Requires LLVM's llvm-objdump on PATH. This compares instruction/listing text,
normalizing local branch destinations to function offsets, external numeric
addresses to a marker, and RIP-relative displacements. It preserves symbolic
callee names, operands, registers and relative internal branch destinations.
It is not an audit of referenced literal bytes or linked-text byte identity.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

SYMBOL = '__RNCINvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute3runNtNtNtNtB8_10compact_fn4wide10loop_frame13WideLoopFrameEs_0Ba_'


def inspect(binary):
    listing = subprocess.check_output(['llvm-objdump', '--macho', '--disassemble',
                                     '--dis-symname', SYMBOL, str(binary)], text=True)
    lines = listing.split(SYMBOL + ':\n', 1)[1].strip().splitlines()
    start = int(lines[0].split(':')[0], 16)
    end = int(lines[-1].split(':')[0], 16) + len(lines[-1].split('\t')[1].split())
    def normalize(line):
        operation = '\t'.join(line.split('\t')[2:])
        def address(match):
            value = int(match.group(0), 16)
            return f'@+{value-start:x}' if start <= value < end else '@outside'
        operation = re.sub(r'0x100[0-9a-f]+', address, operation)
        return re.sub(r'0x[0-9a-f]+(?=\(%rip\))', '@rip', operation)
    normalized = '\n'.join(map(normalize, lines))
    return {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
            'address': hex(start), 'bytes': end - start, 'listing_lines': len(lines),
            'normalized_sha256': hashlib.sha256(normalized.encode()).hexdigest()}, normalized


values = [inspect(Path(name)) for name in sys.argv[1:]]
assert values, 'pass two or more exact-image paths'
print(json.dumps({'symbol': SYMBOL, 'images': [v[0] for v in values],
                  'equal_normalized_listings': all(v[1] == values[0][1] for v in values)}, indent=2))
