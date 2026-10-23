"""Code-ASLR-only diagnostic, if vmmap confirms the stack still randomizes.

Copy the frozen candidate twice, clear MH_PIE in one copy, ad-hoc sign both,
and verify complete __text bytes are unchanged. Originals remain read-only.
No resulting executable is a production or acceptance candidate.
"""
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time


def text_section(data):
    magic, cpu, _, _, commands, _, _, _ = struct.unpack_from('<8I', data)
    assert magic == 0xfeedfacf and cpu == 0x1000007
    offset = 32
    for _ in range(commands):
        command, size = struct.unpack_from('<II', data, offset)
        if command == 0x19:
            count = struct.unpack_from('<I', data, offset + 64)[0]
            for index in range(count):
                section = offset + 72 + index * 80
                if data[section:section + 16].rstrip(b'\0') == b'__text' and data[section + 16:section + 32].rstrip(b'\0') == b'__TEXT':
                    length = struct.unpack_from('<Q', data, section + 40)[0]
                    start = struct.unpack_from('<I', data, section + 48)[0]
                    return start, length, hashlib.sha256(data[start:start + length]).hexdigest()
        offset += size
    raise RuntimeError('no text section')


out = Path(sys.argv[1]).resolve(); out.mkdir()
job = Path(os.environ['BENCH_QUEUE']) / '20261023T154044Z-t072-screen-attempt2/out/attempt2'
original = Path(json.loads((job / 'cand000/identity.json').read_text())['variants'][0]['path'])
data = original.read_bytes()
assert hashlib.sha256(data).hexdigest() == 'f6d5220992f3dd7d05afaad4af2e169a01258294e7995347d28f81e4ca74a5e3'
expected = text_section(data)
identity = {'kind': 'pie-header-control', 'original_text': expected}
observed = {'base': [], 'candidate': []}
for role, directory, enabled in [('base', 'pie-on', True), ('candidate', 'pie-no', False)]:
    binary = out / directory / 'qjs'; binary.parent.mkdir()
    modified = bytearray(data)
    flags = struct.unpack_from('<I', modified, 24)[0]
    assert flags & 0x200000
    if not enabled:
        struct.pack_into('<I', modified, 24, flags & ~0x200000)
    binary.write_bytes(modified); binary.chmod(0o755)
    subprocess.run(['codesign', '--force', '--sign', '-', str(binary)], check=True)
    actual = binary.read_bytes()
    assert text_section(actual) == expected
    assert bool(struct.unpack_from('<I', actual, 24)[0] & 0x200000) == enabled
    subprocess.run([str(binary), '-e', '1;'], check=True, capture_output=True)
    identity[role] = {'binary': str(binary), 'sha256': hashlib.sha256(actual).hexdigest(), 'pie': enabled}
    for trial in range(2):
        child = subprocess.Popen([str(binary), '--raw', 'benchmarks/workloads/broad-micro.js', 'local_read', '200000000'], stdout=subprocess.DEVNULL)
        time.sleep(.05)
        mapping = subprocess.run(['/usr/bin/vmmap', '-w', str(child.pid)], capture_output=True, text=True, timeout=20)
        (out / f'{directory}-{trial}.maps').write_text(mapping.stdout + mapping.stderr)
        assert child.wait(timeout=30) == 0
        code = re.search(r'^__TEXT\s+([0-9a-f]+)-[0-9a-f]+.*qjs', mapping.stdout, re.M)
        stack = re.search(r'^Stack\s+([0-9a-f]+-[0-9a-f]+) ', mapping.stdout, re.M)
        observed[role].append({'text': code.group(1) if code else None, 'stack': stack.group(1) if stack else None, 'status': mapping.returncode})
        print(directory, trial, 'vmmap', mapping.returncode, observed[role][-1], flush=True)
identity['mappings'] = observed
identity['independent_code_control'] = (all(x['status'] == 0 and x['text'] and x['stack'] for rows in observed.values() for x in rows)
    and len({x['text'] for x in observed['candidate']}) == 1
    and len({x['stack'] for x in observed['candidate']}) == 2
    and len({x['text'] for x in observed['base']}) == 2)
print('independent code control', identity['independent_code_control'], flush=True)
(out / 'identity.json').write_text(json.dumps(identity, indent=2) + '\n')

if len(sys.argv) > 2 and identity['independent_code_control']:
    subprocess.run([sys.executable, 'tasks/diagnostics/t073/collect.py', '--repeats', sys.argv[2],
                    '--counts', '46137344', '--binary-identity', str(out / 'identity.json'),
                    '--output', str(out / 'samples')], check=True)
