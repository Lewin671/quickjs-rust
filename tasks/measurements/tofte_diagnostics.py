"""Install/restore T038 temporary event counters in a linked scratch worktree.

The diagnostic binary must never be timed. This edits only the listed source
files, records byte backups, and refuses restore if another edit intervened.
It does not commit source changes or touch excluded performance-owner files.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

FILES = (
    'diagnostics.rs', 'bytecode/vm/rare_ops.rs', 'bytecode/vm/general_ops.rs',
    'bytecode/vm_string_append.rs', 'bytecode/vm_frame_init.rs', 'bytecode/vm_call.rs',
    'date/accessors.rs', 'date/constructor.rs', 'string/constructor.rs',
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def install(root: Path, backups: Path) -> None:
    if not (root / '.git').is_file():
        raise ValueError('instrument only an isolated linked scratch worktree')
    if backups.exists():
        raise ValueError('backup directory already exists; restore it or choose a fresh path')
    base = root / 'crates/qjs-runtime/src'
    original = {name: (base / name).read_bytes() for name in FILES}
    if b'pricing_closures' in original['diagnostics.rs']:
        raise ValueError('counters already installed')
    changed = {name: data.decode() for name, data in original.items()}
    fields = ['pricing_closures', 'pricing_concat', 'pricing_number_concat',
              'pricing_overlay_slots', 'pricing_selected_eval_slots',
              'pricing_selected_eval_calls', 'pricing_string_fast_construct',
              'pricing_array_literal', 'pricing_date_construct', 'pricing_string_construct']

    def replace(name: str, before: str, after: str, expected: int = 1) -> None:
        if changed[name].count(before) != expected:
            raise ValueError(f'{name}: base source changed; re-audit the instrumentation')
        changed[name] = changed[name].replace(before, after)

    replace('bytecode/vm/rare_ops.rs',
            'let (home_object, super_constructor) = if *lexical_this {',
            'crate::diagnostics::count!(pricing_closures);\n'
            '                let (home_object, super_constructor) = if *lexical_this {')
    replace('bytecode/vm/general_ops.rs',
            'Op::NewArray { elements } => self.new_array(elements)?,',
            'Op::NewArray { elements } => { crate::diagnostics::count!(pricing_array_literal); '
            'self.new_array(elements)?; },')
    replace('bytecode/vm_string_append.rs', '    let right_len = match &right {',
            '    crate::diagnostics::count!(pricing_concat);\n'
            '    if matches!(left, Value::Number(_)) || matches!(right, Value::Number(_)) '
            '{ crate::diagnostics::count!(pricing_number_concat); }\n'
            '    let right_len = match &right {')
    replace('bytecode/vm_frame_init.rs',
            'for (slot, local) in self.bytecode.locals.iter().enumerate() {',
            'for (slot, local) in self.bytecode.locals.iter().enumerate() {\n'
            '            crate::diagnostics::count!(pricing_overlay_slots);', expected=2)
    replace('bytecode/vm_call.rs',
            'fn selected_direct_eval_call_env(&self, eval_bytecode: &super::Bytecode) -> CallEnv {',
            'fn selected_direct_eval_call_env(&self, eval_bytecode: &super::Bytecode) -> CallEnv {\n'
            '        crate::diagnostics::count!(pricing_selected_eval_calls);')
    for before in ('for index in 0..self.locals.len() {',
                   'for (index, upvalue) in self.local_upvalues.iter().enumerate() {'):
        replace('bytecode/vm_call.rs', before,
                before + '\n            crate::diagnostics::count!(pricing_selected_eval_slots);')
    for directory, field in (('date', 'pricing_date_construct'), ('string', 'pricing_string_construct')):
        replace(f'{directory}/constructor.rs', '    if !is_construct {',
                f'    crate::diagnostics::count!({field});\n    if !is_construct {{')
    replace('string/constructor.rs', '    let text = match argument_values.first()? {',
            '    crate::diagnostics::count!(pricing_string_fast_construct);\n'
            '    let text = match argument_values.first()? {')
    accessors = changed['date/accessors.rs']
    for match in list(re.finditer(r'pub\(crate\) fn (native_date_prototype_\w+)\(', accessors))[::-1]:
        brace = accessors.index('{', match.end())
        field = 'pricing_' + match[1].removeprefix('native_date_prototype_')
        fields.append(field)
        accessors = accessors[:brace+1] + f'\n    crate::diagnostics::count!({field});' + accessors[brace+1:]
    changed['date/accessors.rs'] = accessors
    replace('diagnostics.rs', 'declare_counters! {',
            'declare_counters! {\n' + ''.join(f'    {field},\n' for field in fields))
    backups.mkdir(parents=True)
    manifest = {}
    for name in FILES:
        backup = backups / name
        backup.parent.mkdir(parents=True, exist_ok=True)
        backup.write_bytes(original[name])
        data = changed[name].encode()
        manifest[name] = {'original': digest(original[name]), 'instrumented': digest(data)}
    (backups / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    for name in FILES:
        (base / name).write_text(changed[name])


def restore(root: Path, backups: Path) -> None:
    if not (root / '.git').is_file():
        raise ValueError('restore only in a linked scratch worktree')
    base = root / 'crates/qjs-runtime/src'
    manifest = json.loads((backups / 'manifest.json').read_text())
    if set(manifest) != set(FILES):
        raise ValueError('backup manifest path boundary changed')
    for name, hashes in manifest.items():
        if digest((base / name).read_bytes()) != hashes['instrumented']:
            raise ValueError(f'{name}: intervening edit; refusing to overwrite')
        if digest((backups / name).read_bytes()) != hashes['original']:
            raise ValueError(f'{name}: backup changed')
    for name in manifest:
        (base / name).write_bytes((backups / name).read_bytes())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('install', 'restore'))
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--backups', type=Path, required=True)
    args = parser.parse_args()
    {'install': install, 'restore': restore}[args.mode](args.root.resolve(), args.backups.resolve())


if __name__ == '__main__':
    main()
