"""Synthetic dyld streams and pointer graphs for the K0 contract amendment."""
import struct
import unittest

from tools.benchmark.kernel_decode import decode_body
from tools.benchmark.kernel_isolation import compare
from tools.benchmark.kernel_macho import Image
from tools.benchmark.kernel_referents import Referents
from tools.benchmark.kernel_relocations import Stream, decode_stream, pointer_fixups
from tools.benchmark.tests.test_kernel_isolation import report


def uleb(value):
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7f) | 0x80)
        value >>= 7
    return bytes(out + bytes([value]))


def image(target=0x4000, destination=0x4000, payload=b'contents', kind='rebase'):
    """One metadata pointer and two equally sized, byte-identical symbols."""
    result = Image.__new__(Image)
    data = bytearray(0x400)
    data[0x100:0x108] = bytes.fromhex('488d3df90f0000c3')
    data[0x200:0x210] = struct.pack('<Q', destination) + b'raw-data'
    data[0x300:0x310] = payload*2
    result.data = bytes(data)
    result.sections = [(0x1000, 8, '__TEXT/__text', 0x100),
                       (0x2000, 16, '__DATA/__const', 0x200),
                       (target, 16, '__TEXT/__const', 0x300)]
    result.names = {0x1000: ['_protected'], 0x2000: ['_metadata'],
                    target: ['_target_a'], target+8: ['_target_b']}
    result.name_starts = sorted(result.names)
    result.unbacked, result.imports, result.tables = set(), {}, []
    result.pointer_fixups = {0x2000: {'kind': kind}}
    result.referents = Referents(result)
    return result


def decoded(img):
    rows = [(0x1000, img.raw(0x1000, 7), 'leaq', '0xff9(%rip), %rdi'),
            (0x1007, b'\xc3', 'retq', '')]
    return report(decode_body(img, rows, 0x1000, 8))


class ReferentTests(unittest.TestCase):
    def test_moved_pointer_with_identical_symbol_and_target_passes(self):
        a = image()
        b = image(target=0x5000, destination=0x5000)
        self.assertNotEqual(a.raw(0x2000, 16), b.raw(0x2000, 16))
        self.assertEqual(a.referent(0x2000, 16)['sha256'], b.referent(0x2000, 16)['sha256'])
        result = compare(decoded(a), decoded(b))
        self.assertTrue(result['pass'])
        self.assertIn('referent-pointer-relocation', result['roles']['Vm']['allowed_fixups'][0]['changes'])

    def test_retarget_to_different_symbol_with_equal_bytes_fails(self):
        a, b = image(), image(destination=0x4008)
        self.assertEqual(a.raw(0x4000, 8), b.raw(0x4008, 8))
        self.assertFalse(compare(decoded(a), decoded(b))['pass'])

    def test_target_byte_change_with_unchanged_identity_fails(self):
        self.assertFalse(compare(decoded(image()), decoded(image(payload=b'changed!')))['pass'])

    def test_non_pointer_bytes_stay_raw(self):
        a, b = image(), image()
        data = bytearray(b.data)
        data[0x208] ^= 1
        b.data = bytes(data)
        self.assertFalse(compare(decoded(a), decoded(b))['pass'])

    def test_unresolvable_pointer_and_unknown_relocation_fail_closed(self):
        for img in (image(destination=0xdead), image(kind='unknown')):
            with self.subTest(img=img), self.assertRaises(ValueError):
                decoded(img)

    def test_partial_pointer_slot_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'partially covered'):
            image().referent(0x2001, 7)

    def test_lea_inside_known_symbol_ends_at_that_symbol(self):
        img = image()
        self.assertEqual(img.address_span(0x4004), 4)
        self.assertEqual(img.referent_target(0x4004),
                         ({'section': '__TEXT/__const', 'symbol': '_target_a', 'offset': 4}, 4))

    def test_anonymous_lea_uses_next_boundary_without_previous_section_symbol(self):
        img = image()
        del img.names[0x2000]
        img.name_starts = sorted(img.names)
        self.assertEqual(img.address_span(0x2004), 12)
        self.assertIsNone(img.symbol_extent(0x2004))

    def test_import_binding_identity_cannot_be_replaced_by_equal_zero_bytes(self):
        a, b = image(), image()
        for img, library in ((a, 'lib-a'), (b, 'lib-b')):
            img.pointer_fixups[0x2000] = {'kind': 'bind', 'library': library,
                                         'symbol': '_foo', 'addend': 0, 'flags': 0}
        self.assertNotEqual(a.referent(0x2000, 16)['sha256'], b.referent(0x2000, 16)['sha256'])

    def test_depth_budget_rejects_new_unchecked_target_nodes(self):
        img = image()
        img.sections = [(0x2000, 64, '__DATA/__const', 0x200)]
        img.names = {0x2000+n*16: [f'_node{n}'] for n in range(4)}
        img.name_starts = sorted(img.names)
        data = bytearray(img.data)
        for n in range(3):
            struct.pack_into('<Q', data, 0x200+n*16, 0x2010+n*16)
        img.data = bytes(data)
        img.pointer_fixups = {0x2000+n*16: {'kind': 'rebase'} for n in range(3)}
        img.referents = Referents(img)
        with self.assertRaisesRegex(ValueError, 'depth budget'):
            img.referent(0x2000, 16)

    def test_cycles_check_every_node_without_infinite_recursion(self):
        img = image()
        img.sections = [(0x2000, 32, '__DATA/__const', 0x200)]
        img.names = {0x2000: ['_a'], 0x2010: ['_b']}
        img.name_starts = sorted(img.names)
        data = bytearray(img.data)
        struct.pack_into('<Q', data, 0x200, 0x2010)
        struct.pack_into('<Q', data, 0x210, 0x2000)
        img.data = bytes(data)
        img.pointer_fixups = {0x2000: {'kind': 'rebase'}, 0x2010: {'kind': 'rebase'}}
        img.referents = Referents(img)
        before = img.referent(0x2000, 16)
        self.assertEqual(len(before['content']['nodes']), 2)
        data[0x218] ^= 1
        img.data = bytes(data)
        img.referents = Referents(img)
        self.assertNotEqual(before['sha256'], img.referent(0x2000, 16)['sha256'])


class DyldTests(unittest.TestCase):
    def test_rebase_opcode_families_and_scaled_offsets(self):
        stream = bytes.fromhex('112000513008416002700880020800')
        slots = decode_stream(stream, [(0x1000, 0x100)], [])
        self.assertEqual(list(slots), [0x1000+n for n in (0, 24, 32, 40, 56, 72)])
        self.assertTrue(all(v == {'kind': 'rebase'} for v in slots.values()))

    def test_bind_opcode_families_and_addend(self):
        stream = bytes.fromhex('1140') + b'_foo\0' + bytes.fromhex('51607f700090a008b1c0020880089000')
        slots = decode_stream(stream, [(0x1000, 0x100)], ['lib-a'], bind=True)
        self.assertEqual(list(slots), [0x1000+n for n in (0, 8, 24, 40, 56, 80)])
        self.assertEqual(slots[0x1000], {'kind': 'bind', 'library': 'lib-a',
                                       'symbol': '_foo', 'addend': -1, 'flags': 0})

    def test_ld64_unsigned_backward_addition(self):
        stream = b'\x11\x40_foo\0\x70\x10\x90\x80' + uleb((1 << 64)-24) + b'\x90\0'
        self.assertEqual(list(decode_stream(stream, [(0x1000, 64)], ['lib-a'], bind=True)), [0x1010, 0x1000])

    def test_lazy_entries_reset_binding_state(self):
        entry = b'\x11\x40_foo\0\x70\0\x90\0'
        self.assertEqual(len(decode_stream(entry, [(0x1000, 64)], ['lib-a'], bind=True, lazy=True)), 1)
        with self.assertRaisesRegex(ValueError, 'unresolved'):
            decode_stream(entry+b'\x70\x08\x90\0', [(0x1000, 64)], ['lib-a'], bind=True, lazy=True)

    def test_unknown_type_opcode_truncated_and_out_of_segment(self):
        for stream in (b'\x12\0', b'\x90\0', b'\x11\x20', b'\x11\x21\0\x51\0',
                       b'\x11\x20\x7f\x51\0', b'\x11\x20\0\x51', b'\0\x11'):
            with self.subTest(stream=stream), self.assertRaises(ValueError):
                decode_stream(stream, [(0x1000, 64)], [])
        with self.assertRaisesRegex(ValueError, 'threaded'):
            decode_stream(b'\xd0', [(0x1000, 64)], ['lib-a'], bind=True)

    def test_leb_truncation_overflow_and_resource_budget(self):
        for value in (b'\x80', b'\xff'*12):
            with self.subTest(value=value), self.assertRaises(ValueError):
                Stream(value).leb()
        with self.assertRaisesRegex(ValueError, 'count'):
            decode_stream(bytes.fromhex('112000600a00'), [(0x1000, 0x100)], [], max_slots=3)

    def test_load_command_inventory_and_unknown_chained_relocations(self):
        data = bytearray(152)
        struct.pack_into('<I', data, 16, 2)
        struct.pack_into('<II', data, 32, 0x19, 72)
        struct.pack_into('<QQ', data, 56, 0x1000, 64)
        struct.pack_into('<II', data, 104, 0x80000022, 48)
        code = bytes.fromhex('1120005100')
        struct.pack_into('<II', data, 112, len(data), len(code))
        data += code
        self.assertEqual(pointer_fixups(bytes(data), [(0x1000, 64, '__DATA/__const', 152)]),
                         {0x1000: {'kind': 'rebase'}})
        struct.pack_into('<I', data, 104, 0x80000034)
        with self.assertRaisesRegex(ValueError, 'chained'):
            pointer_fixups(bytes(data), [])
