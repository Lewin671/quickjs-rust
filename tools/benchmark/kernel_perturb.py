"""Archive-only K0 header perturbations and independent layout witnesses.

These are the spike's H+8/C+8 transformations, without its outlining. Never
apply them to a working checkout. Constructors retain their original effects.
"""
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


def perturb(source, kind):
    """Add one initialized native word, rejecting stale constructor coverage."""
    path = source / OBJECT
    text = path.read_text()
    if '_kernel_probe_word' in text:
        raise ValueError('perturbation already installed')
    if kind == 'H8':
        text = replace_exact(text, 'struct ObjectData {',
                             'struct ObjectData {\n    _kernel_probe_word: usize,', 1)
        text = replace_exact(text, 'Self(Rc::new(ObjectData {',
                             'Self(Rc::new(ObjectData {\n            _kernel_probe_word: 0,', 3)
        builder = source / BUILDER
        builder.write_text(replace_exact(builder.read_text(), 'ObjectRef(Rc::new(ObjectData {',
                           'ObjectRef(Rc::new(ObjectData {\n            _kernel_probe_word: 0,', 1))
    elif kind == 'C8':
        text = replace_exact(text, 'struct ObjectColdData {',
                             'struct ObjectColdData {\n    _kernel_probe_word: usize,', 1)
    elif kind != 'R0':
        raise ValueError(f'unknown perturbation: {kind}')
    path.write_text(text)


def install_layout_probe(source):
    """A separate test build witnesses sizes/offsets; restore before release."""
    path = source / OBJECT
    text = path.read_text()
    probe = '\n#[test]\nfn kernel_isolation_layout_probe() {\n'
    for typ in ('ObjectData', 'ObjectColdData'):
        probe += f'println!("LAYOUT {typ} {{}}", std::mem::size_of::<{typ}>());\n'
    for field in FIELDS:
        probe += f'println!("OFFSET {field} {{}}", std::mem::offset_of!(ObjectData, {field}));\n'
    path.write_text(text + probe + '}\n')
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
