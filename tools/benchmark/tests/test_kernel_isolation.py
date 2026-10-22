"""K0 negative controls use synthetic bytes, not host compiler output.

The Add 0x29ca -> 0x29be offsets come from T056's four off-path 3-byte
shortenings, recorded by T064. This fixture models that PC drift; its bytes
are synthetic, never claimed to be the historical executable.
"""
from copy import deepcopy
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from tools.benchmark.kernel_decode import decode_body, memory_width, rip_position
from tools.benchmark.kernel_isolation import compare
from tools.benchmark.kernel_macho import Image, ROLES, assembly, select_roles
from tools.benchmark.kernel_perturb import (BUILDER, FIELDS, OBJECT, install_layout_probe,
                                           perturb, validate_layout)
from tools.benchmark.kernel_build import matrix, parse_layout
from tools.benchmark.tests.layout_fixtures import (
    INTERPRETER_CLOSURE, WIDE_CLOSURE, read_symbols)


class Memory:
    """Tiny addressable linked image, including a literal and a call target."""
    def __init__(self, code, literal=b'12345678', tables=()):
        self.regions = [(0x1000, code), (0x2000, literal)]
        self.tables = list(tables)

    def raw(self, address, width):
        for start, data in self.regions:
            if start <= address and address+width <= start+len(data) and width:
                return data[address-start:address-start+width]
        raise ValueError('unmapped referent')

    def identity(self, target):
        if target == 0x3000:
            return 'import:_pow'
        raise ValueError('unknown relocation/call target')

    def import_identity(self, target):
        return None

    def address_span(self, target):
        return len(self.raw(target, 8))


def body(literal=b'12345678', displacement=0xff8, call_disp=0x1ff3):
    load = bytes.fromhex('f20f1005') + struct.pack('<i', displacement)
    call = b'\xe8' + struct.pack('<i', call_disp)
    rows = [(0x1000, load, 'movsd', f'{displacement:#x}(%rip), %xmm0'),
            (0x1008, call, 'callq', f'{0x100d+call_disp:#x}'),
            (0x100d, b'\xc3', 'retq', '')]
    image = Memory(load+call+b'\xc3', literal)
    return decode_body(image, rows, 0x1000, 14)


def report(b=None):
    return {'schema': 1, 'target': 'x86_64-apple-darwin', 'family': 'run',
            'decoder': {'sha256': 'audited-decoder'}, 'adapters': [],
            'roles': {r: deepcopy(b if b is not None else body()) for r in ROLES}}


def table_body(target_offset=16, bad_padding=False):
    code = bytes.fromhex('4c8d050a000000496304884c01c0ffe0c3')
    table = struct.pack('<i', target_offset-17)
    tail = bytes.fromhex('0f1f4000')
    rows = [(0x1000, code[:7], 'leaq', '0xa(%rip), %r8'),
            (0x1007, code[7:11], 'movslq', '(%r8,%rcx,4), %rax'),
            (0x100b, code[11:14], 'addq', '%r8, %rax'),
            (0x100e, code[14:16], 'jmpq', '*%rax'),
            (0x1010, b'\xc3', 'retq', ''),
            # Accidental otool decoding within table/after it is not code.
            (0x1011, b'\xff', '.byte', '0xff #bad opcode'),
            (0x1016, b'\x1f', '.byte', '0x1f #bad opcode')]
    image = Memory(code+table+(b'bad!' if bad_padding else tail), tables=[(0x1011, 4, 4)])
    return decode_body(image, rows, 0x1000, 25)


class ContractTests(unittest.TestCase):
    def test_aa(self):
        self.assertTrue(compare(report(), report())['pass'])

    def test_only_rip_and_call_displacements_are_allowed(self):
        a, b = report(), report()
        for role in ROLES:
            rows = b['roles'][role]['instructions']
            for i in (0, 1):
                raw = bytearray.fromhex(rows[i]['raw'])
                raw[rows[i]['fixup']['position']] ^= 0x10
                rows[i]['raw'] = raw.hex()
        result = compare(a, b)
        self.assertTrue(result['pass'])
        self.assertEqual([r['kind'] for r in result['roles']['Vm']['allowed_fixups']], ['rip', 'call'])

    def test_three_byte_insertion_is_not_normalized_away(self):
        a, b = report(), report()
        b['roles']['Vm']['instructions'][1]['offset'] += 3
        self.assertFalse(compare(a, b)['pass'])

    def test_historical_add_29ca_to_29be_is_a_failure(self):
        def historical_shape(short):
            cmp = bytes.fromhex('48837f7f01' if short else '4883bf8000000001')
            parts = [(cmp, 'cmpq', '$1, 0x7f(%rdi)' if short else '$1, 0x80(%rdi)')]*4
            parts += [(b'\x90', 'nop', '')]*(0x29ca-32)
            parts += [(bytes.fromhex('f20f1100'), 'movsd', '%xmm0, (%rax)'), (b'\xc3', 'retq', '')]
            code, rows, pc = b'', [], 0x1000
            for raw, op, operands in parts:
                rows.append((pc, raw, op, operands))
                code += raw
                pc += len(raw)
            return decode_body(Memory(code), rows, 0x1000, len(code))
        a, b = report(historical_shape(False)), report(historical_shape(True))
        self.assertEqual(a['roles']['Vm']['instructions'][-2]['offset'], 0x29ca)
        self.assertEqual(b['roles']['Vm']['instructions'][-2]['offset'], 0x29be)
        self.assertEqual(a['roles']['Vm']['instructions'][-2]['raw'], b['roles']['Vm']['instructions'][-2]['raw'])
        result = compare(a, b)
        self.assertFalse(result['pass'])
        self.assertTrue(any('relative PCs' in f for f in result['roles']['Vm']['failures']))

    def test_hot_opcode_change(self):
        a, b = report(), report()
        b['roles']['Vm']['instructions'][0]['raw'] = 'f20f100d' + 'f80f0000'
        self.assertFalse(compare(a, b)['pass'])

    def test_referent_bytes_checked_even_when_instruction_unchanged(self):
        self.assertFalse(compare(report(), report(body(b'changed!')))['pass'])

    def test_referent_width_change(self):
        a, b = report(), report()
        b['roles']['Vm']['instructions'][0]['fixup']['width'] = 4
        self.assertFalse(compare(a, b)['pass'])

    def test_jump_table_target_and_position_changes(self):
        a = report(table_body())
        for key, value in [('targets', [0]), ('offset', 18), ('raw', '00000000')]:
            with self.subTest(key=key):
                b = deepcopy(a)
                b['roles']['Vm']['tables'][0][key] = value
                self.assertFalse(compare(a, b)['pass'])

    def test_stack_change(self):
        a, b = report(), report()
        b['roles']['Vm']['stack'] = [[0, 'subq', '$0x180, %rsp']]
        self.assertFalse(compare(a, b)['pass'])

    def test_missing_role(self):
        b = report()
        del b['roles']['Vm']
        with self.assertRaisesRegex(ValueError, 'role'):
            compare(report(), b)

    def test_unknown_relocation_fails_aa(self):
        a = report()
        a['roles']['Vm']['instructions'][0]['fixup']['kind'] = 'absolute64'
        with self.assertRaisesRegex(ValueError, 'unknown relocation'):
            compare(a, deepcopy(a))

    def test_unknown_target_decoder_and_family(self):
        for key, value in [('schema', 9), ('target', 'aarch64-apple-darwin'),
                           ('decoder', {}), ('family', 'future'), ('adapters', ['clone'])]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                a, b = report(), report()
                b[key] = value
                compare(a, b)


class DecoderTests(unittest.TestCase):
    def test_literal_operand_width(self):
        self.assertEqual(body()['instructions'][0]['fixup']['width'], 8)
        self.assertEqual(memory_width('andpd'), 16)
        self.assertEqual(memory_width('movzbl'), 1)
        with self.assertRaisesRegex(ValueError, 'width'):
            memory_width('unknown')

    def test_rip_modrm_and_displacement_are_validated(self):
        raw = bytes.fromhex('f20f1005f80f0000')
        self.assertEqual(rip_position(raw, 0xff8), 4)
        for changed, displacement in [(raw, 1), (raw[:3]+b'\x04'+raw[4:], 0xff8),
                                      (b'\x0f\xff'+raw[2:], 0xff8)]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                rip_position(changed, displacement)

    def test_all_owned_table_targets_are_instruction_starts(self):
        self.assertEqual(table_body()['tables'][0]['targets'], [16])
        with self.assertRaisesRegex(ValueError, 'instruction boundary'):
            table_body(15)

    def test_table_tail_is_independently_verified(self):
        self.assertEqual(table_body()['padding'], [21, '0f1f4000'])
        with self.assertRaisesRegex(ValueError, 'tail alignment'):
            table_body(bad_padding=True)

    def test_unknown_opcode_gap_and_indirect_branch_fail_closed(self):
        for code, rows in [(b'\x0f\x31', [(0x1000, b'\x0f\x31', 'rdtsc', '')]),
                           (b'\x90\xc3', [(0x1000, b'\x90', 'nop', '')]),
                           (b'\xff\xe0', [(0x1000, b'\xff\xe0', 'jmpq', '*%rax')])]:
            with self.subTest(code=code), self.assertRaises(ValueError):
                decode_body(Memory(code), rows, 0x1000, len(code))

    def test_unknown_call_target(self):
        with self.assertRaisesRegex(ValueError, 'unknown relocation'):
            body(call_disp=0x1000)

    def test_disassembler_unknown_format_fails_closed(self):
        with patch('tools.benchmark.kernel_macho.command', return_value='0000000000001000 mystery'):
            with self.assertRaises(ValueError):
                assembly(Path('qjs'))

    def test_unresolved_section_relocation(self):
        image = Image.__new__(Image)
        data = bytearray(184)
        struct.pack_into('<I', data, 16, 1)
        struct.pack_into('<II', data, 32, 0x19, 152)
        struct.pack_into('<I', data, 96, 1)
        struct.pack_into('<I', data, 164, 1)
        image.data = bytes(data)
        with self.assertRaisesRegex(ValueError, 'unknown unresolved'):
            image.data_in_code()


    def test_equal_got_bytes_do_not_hide_import_retargeting(self):
        with patch.object(Memory, 'import_identity', return_value='import:_pow'):
            a = report(body(b'\0'*8))
        with patch.object(Memory, 'import_identity', return_value='import:_fmod'):
            b = report(body(b'\0'*8))
        self.assertFalse(compare(a, b)['pass'])

    def test_unknown_import_slot_and_zero_fill_referents_fail_closed(self):
        image = Image.__new__(Image)
        image.imports = {}
        image.sections = [(0x2000, 8, '__DATA_CONST/__got', 0)]
        image.unbacked = {0x2000}
        image.data = b'header!!'
        with self.assertRaisesRegex(ValueError, 'unknown import-slot'):
            image.import_identity(0x2000)
        with self.assertRaisesRegex(ValueError, 'zero-fill'):
            image.raw(0x2000, 8)


class InventoryTests(unittest.TestCase):
    def test_exact_roles(self):
        family, roles, _ = select_roles(read_symbols([(INTERPRETER_CLOSURE, 32), (WIDE_CLOSURE, 32)]))
        self.assertEqual(family, 'run')
        self.assertEqual(set(roles), set(ROLES))

    def test_missing_duplicate_alias_and_clone(self):
        for rows in [[(INTERPRETER_CLOSURE, 32)],
                     [(INTERPRETER_CLOSURE, 32), (WIDE_CLOSURE, 32), (WIDE_CLOSURE, 32)],
                     [(INTERPRETER_CLOSURE, 32), (WIDE_CLOSURE, 0), (WIDE_CLOSURE+'.1', 32)],
                     [(INTERPRETER_CLOSURE, 32), (WIDE_CLOSURE, 32), (WIDE_CLOSURE+'.llvm.1', 32)]]:
            with self.subTest(rows=rows), self.assertRaisesRegex(ValueError, 'role'):
                select_roles(read_symbols(rows))


class PerturbationTests(unittest.TestCase):
    def layout(self):
        return {'ObjectData': 112, 'ObjectColdData': 400, **dict.fromkeys(FIELDS, 16)}

    def test_layout_witness_rejects_optimized_away_or_missing_probe(self):
        a = self.layout()
        for kind in ('H8', 'C8'):
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                validate_layout(a, a, kind)
        with self.assertRaises(ValueError):
            validate_layout({}, {}, 'R0')

    def test_layout_delta_and_field_movement(self):
        a = self.layout()
        b = {**a, 'ObjectData': 120, 'symbol_brand': 24}
        validate_layout(a, b, 'H8')
        validate_layout(a, {**a, 'ObjectColdData': 408}, 'C8')
        with self.assertRaises(ValueError):
            validate_layout(a, {**a, 'ObjectData': 120}, 'H8')
        with self.assertRaises(ValueError):
            validate_layout(a, b, 'C8')

    def test_transforms_cover_all_constructors_and_probe_is_separate(self):
        for kind in ('H8', 'C8'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                (root / BUILDER).parent.mkdir(parents=True)
                (root / OBJECT).write_text('struct ObjectData {\nstruct ObjectColdData {\n' +
                                           'Self(Rc::new(ObjectData {\n'*3)
                (root / BUILDER).write_text('ObjectRef(Rc::new(ObjectData {')
                perturb(root, kind)
                self.assertEqual((root / OBJECT).read_text().count('_kernel_probe_word'), 4 if kind == 'H8' else 1)
                original = install_layout_probe(root)
                self.assertNotIn('fn kernel_isolation_layout_probe', original)
                self.assertIn('size_of::<ObjectData>', (root / OBJECT).read_text())
                with self.assertRaises(ValueError):
                    perturb(root, kind)

    def test_stale_constructor_shape_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / OBJECT).parent.mkdir(parents=True)
            (root / OBJECT).write_text('struct ObjectData {')
            with self.assertRaisesRegex(ValueError, 'source shape changed'):
                perturb(root, 'H8')

    def test_matrix_and_layout_parser(self):
        self.assertEqual(len(matrix(False, 'H8')), 2)
        self.assertEqual(matrix(True, 'C8'), [('r0a', 'R0'), ('r0b', 'R0'), ('h8', 'H8'), ('c8', 'C8')])
        self.assertEqual(parse_layout('noise\nLAYOUT ObjectData 112\nOFFSET cold 8'), {'ObjectData': 112, 'cold': 8})
        with self.assertRaises(ValueError):
            parse_layout('LAYOUT ObjectData 112\nLAYOUT ObjectData 112')


class BuildTests(unittest.TestCase):
    def test_unchanged_canonical_image_is_rejected_despite_test_witness(self):
        from tools.benchmark.kernel_build import validate_probe
        base = {'sha256': 'same', 'layout': PerturbationTests().layout()}
        candidate = deepcopy(base)
        candidate['layout'].update(ObjectData=120, symbol_brand=24)
        with self.assertRaisesRegex(ValueError, 'canonical image unchanged'):
            validate_probe(base, candidate, 'H8')

    def test_build_requires_non_timing_queue_and_known_toolchain(self):
        import os
        from tools.benchmark.kernel_build import admit_host
        with patch.dict(os.environ, {}, clear=True), self.assertRaisesRegex(ValueError, 'imac-bench'):
            admit_host(Path('/unused'))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'status').write_text('running no-timing')
            with patch.dict(os.environ, {'BENCH_OUT': str(root / 'out')}, clear=True), \
                    patch('tools.benchmark.kernel_build.run', return_value='release: unknown'), \
                    self.assertRaisesRegex(ValueError, 'unknown host/toolchain'):
                admit_host(root / 'out' / 'k0')

    def test_aa_failure_stops_before_perturbations(self):
        from tools.benchmark.kernel_build import execute
        base = {'sha256': 'same', 'layout': PerturbationTests().layout()}
        bad = report()
        bad['roles']['Vm']['instructions'][0]['raw'] = 'f20f100df80f0000'
        with tempfile.TemporaryDirectory() as tmp, \
                patch('tools.benchmark.kernel_build.admit_host', return_value={}), \
                patch('tools.benchmark.kernel_build.run', return_value='commit'), \
                patch('tools.benchmark.kernel_build.build_one', side_effect=[(base, report()), (base, bad)]) as build:
            with self.assertRaisesRegex(ValueError, 'A/A failed'):
                execute(Path(tmp), 'HEAD', Path(tmp) / 'k0', True, 'H8')
            self.assertEqual(build.call_count, 2)
            self.assertTrue((Path(tmp) / 'k0' / 'comparisons.json').is_file())

    def test_existing_output_is_untouched_on_rejection(self):
        from tools.benchmark.kernel_build import main
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            result = root / 'result.json'
            result.write_text('existing evidence')
            with patch('tools.benchmark.kernel_build.admit_host', return_value={}), \
                    patch('sys.argv', ['kernel_build', '--output', str(root)]):
                self.assertEqual(main(), 2)
            self.assertEqual(result.read_text(), 'existing evidence')

    def test_full_matrix_continues_after_h_control_failure(self):
        from tools.benchmark.kernel_build import execute
        base = {'sha256': 'a', 'layout': PerturbationTests().layout()}
        hot = {'sha256': 'h', 'layout': {**base['layout'], 'ObjectData': 120, 'symbol_brand': 24}}
        cold = {'sha256': 'c', 'layout': {**base['layout'], 'ObjectColdData': 408}}
        bad = report()
        bad['roles']['Vm']['instructions'][0]['raw'] = 'f20f100df80f0000'
        with tempfile.TemporaryDirectory() as tmp, \
                patch('tools.benchmark.kernel_build.admit_host', return_value={}), \
                patch('tools.benchmark.kernel_build.run', return_value='commit'), \
                patch('tools.benchmark.kernel_build.build_one') as build:
            output = Path(tmp) / 'k0'
            def fake_build(*args):
                for name in ('source', 'canonical-target', 'layout-target'):
                    (output / name).mkdir(exist_ok=True)
                return fake_build.records.pop(0)
            fake_build.records = [(base, report()), (base, report()), (hot, bad), (cold, report())]
            build.side_effect = fake_build
            result = execute(Path(tmp), 'HEAD', output, True, 'H8')
            self.assertFalse(result['pass'])
            self.assertTrue(result['comparisons']['r0b']['pass'])
            self.assertFalse(result['comparisons']['h8']['pass'])
            self.assertTrue(result['comparisons']['c8']['pass'])
            self.assertEqual(build.call_count, 4)
            self.assertFalse((output / 'canonical-target').exists())
