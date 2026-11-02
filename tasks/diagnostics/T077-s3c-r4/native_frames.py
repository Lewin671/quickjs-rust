#!/usr/bin/env python3
"""Save native prologues on both production architectures, never type sizes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from tools.benchmark.layout_pin import text_symbols
from tools.benchmark.layout_symbols import parse_symbol

TARGETS = (
    'function::call::call_direct_leaf_function',
    'function::call::call_direct_indexed_fallback',
    'function::call::try_construct_direct_leaf_function',
    'function::call::construct_direct_fallback',
    'bytecode::vm::eval_direct_call_fallback',
    '::run_completion',
    '::settle_stacked_exit',
    '::run_current_activation',
)

def collect(binary, architecture):
    rows = []
    for raw in text_symbols(binary).names:
        if raw.startswith('__RNC'):
            continue
        parsed = parse_symbol(raw)
        if not parsed or not any(key in parsed.canonical for key in TARGETS):
            continue
        output = subprocess.check_output(['xcrun', 'llvm-objdump', '-d',
            '--disassemble-symbols=' + raw, str(binary)], text=True)
        lines = [line for line in output.splitlines() if re.match(r'^\s*[0-9a-f]+:', line)]
        prologue = []
        size = 0
        for line in lines[:32]:
            prologue.append(line)
            if architecture == 'arm64':
                sub = re.search(r'\bsub\s+sp, sp, #0x([0-9a-f]+)(?:, lsl #(\d+))?', line)
                save = re.search(r'\[sp, #-0x([0-9a-f]+)\]!', line)
                if sub:
                    size += int(sub[1], 16) << int(sub[2] or 0)
                elif save:
                    size += int(save[1], 16)
                if re.search(r'\b(?:bl|blr|b|br|ret)\s', line):
                    break
            else:
                sub = re.search(r'\bsubq?\s+\$(0x[0-9a-f]+|\d+), %rsp', line)
                if sub:
                    size += int(sub[1], 0)
                elif re.search(r'\bpushq?\s', line):
                    size += 8
                if re.search(r'\b(?:callq?|jmpq?|retq?)\b', line):
                    break
        rows.append({'symbol': parsed.canonical, 'raw': raw,
                     'native_frame_bytes_from_prologue': size, 'prologue': prologue})
    return {'architecture': architecture,
            'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
            'x86_return_address_bytes_excluded': 8 if architecture == 'x86_64' else 0,
            'symbols': rows,
            'not_emitted': [key for key in TARGETS if not any(key in r['symbol'] for r in rows)]}

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--architecture', choices=['arm64', 'x86_64'], required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.write_text(json.dumps(collect(a.binary, a.architecture), indent=2) + '\n')

if __name__ == '__main__':
    main()
