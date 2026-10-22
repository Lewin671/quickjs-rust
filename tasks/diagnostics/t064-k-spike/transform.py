"""Generate the fixed T064 diagnostic matrix; never edit production sources.

The outlining moves complete non-scalar match arms, including their guards and
effects, unchanged into one cold helper. Failure still deopts at the caller's
current op. Scalar dispatch, admission and residency limits are untouched.
"""
import re
from pathlib import Path

BASE = '06e199fec9e9e51742cdae7633c5dc758c326752'
EXECUTE = Path('crates/qjs-runtime/src/bytecode/typed_loop/execute.rs')
OBJECT = Path('crates/qjs-runtime/src/value/object.rs')
BUILDER = Path('crates/qjs-runtime/src/value/object/ordered_data_builder.rs')
PURE = {'Move', 'ToNumeric', 'Binary', 'Unary', 'Update',
        'JumpIfFalsy', 'Jump', 'Exit', 'Leave'}


def outline(text):
    text = text.replace('fn execute<', '#[inline(never)]\nfn execute<', 1)
    start = text.index('        match *op {', text.index('fn execute<'))
    end = text.index('\n        }\n    }\n}', start)
    body = text[start + len('        match *op {\n'):end]
    starts = list(re.finditer(r'^            TypedOp::(\w+)', body, re.M))
    assert len(starts) == 29, len(starts)
    kept, moved = [], []
    for i, match in enumerate(starts):
        arm = body[match.start():starts[i+1].start() if i+1 < len(starts) else len(body)]
        if match[1] in PURE:
            kept.append(arm)
        else:
            arm = arm.replace('deopt_here!(op);', 'return false;')
            arm = arm.replace('continue;', 'return true;')
            moved.append(arm)
    call = '''            _ => {
                if !object_service(vm, program, registers, receivers, boxed,
                    sloppy_global_writes, &mut shape_caches, &intrinsics, op) {
                    deopt_here!(op);
                }
            }'''
    text = text[:start] + '        match *op {\n' + ''.join(kept) + call + text[end:]
    helper = '''
// Diagnostic boundary only: all boxed/array/object work stays outside dispatch.
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn object_service<F: LoopFrame>(
    vm: &mut F,
    program: &TypedLoopProgram,
    registers: &mut [Typed; REGISTER_FILE],
    receivers: &[crate::ArrayRef],
    boxed: &mut [Value],
    sloppy_global_writes: &[crate::bytecode::vm_bindings::TypedLoopSloppyGlobalWrite],
    shape_caches: &mut [super::ShapeWays],
    intrinsics: &Intrinsics,
    op: &TypedOp,
) -> bool {
    match *op {
''' + ''.join(moved) + '''
        _ => unreachable!("scalar op stays in dispatch"),
    }
    true
}
'''
    text = text.replace('\n/// Loads the frame slots', helper + '\n/// Loads the frame slots', 1)
    # Exit/deopt publication must not inline an object-valued frame store.
    for name in ['write_back', 'deopt', 'materialize_stack']:
        old = 'fn ' + name + '<'
        assert text.count(old) == 1
        text = text.replace(old, '#[cold]\n#[inline(never)]\n' + old)
    return text


def generate(source, outlined, perturbation):
    if outlined:
        path = source / EXECUTE
        path.write_text(outline(path.read_text()))
    path = source / OBJECT
    text = path.read_text()
    if perturbation == 'H8':
        text = text.replace('struct ObjectData {',
                            'struct ObjectData {\n    _diagnostic_word: usize,', 1)
        assert text.count('Self(Rc::new(ObjectData {') == 3
        text = text.replace('Self(Rc::new(ObjectData {',
                            'Self(Rc::new(ObjectData {\n            _diagnostic_word: 0,')
        builder = source / BUILDER
        value = builder.read_text()
        assert value.count('ObjectRef(Rc::new(ObjectData {') == 1
        builder.write_text(value.replace('ObjectRef(Rc::new(ObjectData {',
            'ObjectRef(Rc::new(ObjectData {\n            _diagnostic_word: 0,'))
    elif perturbation == 'C8':
        text = text.replace('struct ObjectColdData {',
                            'struct ObjectColdData {\n    _diagnostic_word: usize,', 1)
    else:
        assert perturbation == 'R0'
    path.write_text(text)


def layout_probe(source):
    """Only installed for a separate test build, removed before canonical build."""
    fields = ['properties', 'property_revision', 'layout_revision',
              'index_property_count', 'prototype', 'cold', 'symbol_brand',
              'module_namespace_exotic']
    test = '\n#[test]\nfn diagnostic_t064_layout() {\n'
    for typ in ['ObjectData', 'ObjectColdData']:
        test += f'println!("LAYOUT {typ} {{}}", std::mem::size_of::<{typ}>());\n'
    for field in fields:
        test += f'println!("OFFSET {field} {{}}", std::mem::offset_of!(ObjectData, {field}));\n'
    test += '}\n'
    path = source / OBJECT
    path.write_text(path.read_text() + test)
