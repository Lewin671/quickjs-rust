"""Archive-only K0 header perturbations and independent layout witnesses.

These are the spike's H+8/C+8 layout changes, with line-preserving insertions
and no outlining. Never apply them to a working checkout. Constructors retain
their original effects; the canonical sources preserve every pre-existing line.
"""
import hashlib
from pathlib import Path

OBJECT = Path('crates/qjs-runtime/src/value/object.rs')
BUILDER = Path('crates/qjs-runtime/src/value/object/ordered_data_builder.rs')
FIELDS = ('properties', 'property_revision', 'layout_revision',
          'index_property_count', 'prototype', 'cold', 'symbol_brand',
          'module_namespace_exotic')


def replace_exact(text, old, new, count):
    if text.count(old) != count:
        raise ValueError(f'perturbation source shape changed: {old!r}')
    return text.replace(old, new)


INSERTIONS = (' _kernel_probe_word: usize,', ' _kernel_probe_word: 0,')


def assert_line_preserving(before, after, insertions=INSERTIONS):
    """Remove only declared inline insertions, checking each original line."""
    if any('\n' in text or '\r' in text or not text for text in insertions):
        raise ValueError('probe insertions must stay on an existing source line')
    old, new = before.splitlines(keepends=True), after.splitlines(keepends=True)
    if len(old) != len(new) or before.count('\n') != after.count('\n'):
        raise ValueError('perturbation changed source line count')
    for number, (left, right) in enumerate(zip(old, new), 1):
        for insertion in insertions:
            right = right.replace(insertion, '')
        if left != right:
            raise ValueError(f'pre-existing source line {number} changed or moved')
    return {'before_lines': len(old), 'after_lines': len(new),
            'preexisting_lines_preserved': True,
            'original_sha256': hashlib.sha256(before.encode('utf-8')).hexdigest()}


def perturb(source, kind):
    """Add an initialized native word without moving any pre-existing line."""
    original = {path: (source / path).read_bytes().decode('utf-8') for path in (OBJECT, BUILDER)}
    edited = original.copy()
    text = original[OBJECT]
    if '_kernel_probe_word' in text:
        raise ValueError('perturbation already installed')
    if kind == 'H8':
        text = replace_exact(text, 'struct ObjectData {',
                             'struct ObjectData {' + INSERTIONS[0], 1)
        text = replace_exact(text, 'Self(Rc::new(ObjectData {',
                             'Self(Rc::new(ObjectData {' + INSERTIONS[1], 3)
        edited[BUILDER] = replace_exact(original[BUILDER], 'ObjectRef(Rc::new(ObjectData {',
                                        'ObjectRef(Rc::new(ObjectData {' + INSERTIONS[1], 1)
    elif kind == 'C8':
        text = replace_exact(text, 'struct ObjectColdData {',
                             'struct ObjectColdData {' + INSERTIONS[0], 1)
    elif kind != 'R0':
        raise ValueError(f'unknown perturbation: {kind}')
    edited[OBJECT] = text
    # Validate every changed/unchanged input before writing either file.
    witness = {str(path): assert_line_preserving(original[path], edited[path]) for path in original}
    for path, contents in edited.items():
        (source / path).write_bytes(contents.encode('utf-8'))
    return witness


def install_layout_probe(source):
    """A separate test build witnesses sizes/offsets; restore before release."""
    path = source / OBJECT
    text = path.read_bytes().decode('utf-8')
    probe = '\n#[test]\nfn kernel_isolation_layout_probe() {\n'
    for typ in ('ObjectData', 'ObjectColdData'):
        probe += f'println!("LAYOUT {typ} {{}}", std::mem::size_of::<{typ}>());\n'
    for field in FIELDS:
        probe += f'println!("OFFSET {field} {{}}", std::mem::offset_of!(ObjectData, {field}));\n'
    path.write_bytes((text + probe + '}\n').encode('utf-8'))
    return text


def validate_layout(base, candidate, kind):
    """No control is admitted merely because its source changed."""
    if kind not in ('R0', 'H8', 'C8'):
        raise ValueError('unknown perturbation kind')
    if set(base) != {'ObjectData', 'ObjectColdData', *FIELDS} or set(candidate) != set(base):
        raise ValueError('incomplete layout witness')
    sizes = ('ObjectData', 'ObjectColdData')
    expected = (8, 0) if kind == 'H8' else (0, 8) if kind == 'C8' else (0, 0)
    if tuple(candidate[t] - base[t] for t in sizes) != expected:
        raise ValueError('optimized-away or invalid perturbation: size delta is not H+8/C+8')
    if kind in ('R0', 'C8') and any(candidate[f] != base[f] for f in FIELDS):
        raise ValueError('unexpected header field drift')
    if kind == 'H8' and not any(candidate[f] != base[f] for f in FIELDS):
        raise ValueError('header perturbation did not move any observed field')
