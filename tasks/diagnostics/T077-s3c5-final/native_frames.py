#!/usr/bin/env python3
"""Record arm64 driver prologues; type sizes are not native frame sizes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from tools.benchmark.layout_pin import text_symbols
from tools.benchmark.layout_symbols import parse_symbol

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--debug', type=Path, required=True)
parser.add_argument('--release', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
targets = ['qjs_runtime::bytecode::compact_fn::activation::run_frames',
           'qjs_runtime::bytecode::compact_fn::wide::activation::run',
           'qjs_runtime::bytecode::compact_fn::wide::activation::run_root']
result = {}
for profile, binary in [('debug', args.debug), ('release', args.release)]:
    rows = []
    for raw in text_symbols(binary).names:
        if raw.startswith('__RNC'):
            continue
        parsed = parse_symbol(raw)
        if not parsed or parsed.canonical not in targets:
            continue
        output = subprocess.check_output(['xcrun', 'llvm-objdump', '-d',
            '--disassemble-symbols=' + raw, str(binary)], text=True)
        lines = [line for line in output.splitlines() if re.match(r'^[0-9a-f]+:', line)][:24]
        size = 0
        for line in lines:
            subtraction = re.search(r'\bsub\s+sp, sp, #0x([0-9a-f]+)(?:, lsl #(\d+))?', line)
            save = re.search(r'\[sp, #-0x([0-9a-f]+)\]!', line)
            if subtraction:
                size += int(subtraction[1], 16) << int(subtraction[2] or 0)
            elif save:
                size += int(save[1], 16)
        rows.append({'symbol': parsed.canonical, 'raw': raw,
                     'native_frame_bytes_from_prologue': size, 'prologue': lines})
    result[profile] = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'symbols': rows, 'not_emitted': sorted(set(targets) - {r['symbol'] for r in rows})}
args.output.write_text(json.dumps(result, indent=2) + '\n')
