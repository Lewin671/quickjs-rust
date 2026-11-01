"""Node comparisons ignore network DNS suffixes without weakening host fields."""
import unittest

from tools.benchmark.layout_identity import node_identity, same_host, same_node


class NodeIdentityTests(unittest.TestCase):
    def test_suffix_and_case_variants_share_one_short_name(self):
        variants = ('iMac', 'iMac.local', 'iMac.lan', 'IMAC.LAN', 'imac.lab.example.org')
        for a in variants:
            self.assertEqual(node_identity(a), 'imac')
            for b in variants:
                with self.subTest(a=a, b=b):
                    self.assertTrue(same_node(a, b))

    def test_different_short_names_remain_different(self):
        for a, b in (('iMac.lan', 'mini.lan'), ('iMac.local', 'iMac2.local'),
                     ('iMac', 'mini.IMAC')):
            with self.subTest(a=a, b=b):
                self.assertFalse(same_node(a, b))
                self.assertFalse(same_node(b, a))

    def test_missing_or_empty_nodes_do_not_establish_identity(self):
        for value in (None, '', '.lan', 42):
            with self.subTest(value=value):
                self.assertFalse(same_node(value, value))
                self.assertFalse(same_node(value, 'iMac.local'))
                self.assertFalse(same_node('iMac.lan', value))

    def test_host_metadata_preserves_every_other_field(self):
        host = {'node': 'iMac.local', 'system': 'Darwin', 'machine': 'x86_64',
                'platform': 'macOS-1', 'processor': 'cpu', 'python': '3.14'}
        changed = {**host, 'node': 'IMAC.lan'}
        self.assertTrue(same_host(host, changed))
        self.assertEqual(host['node'], 'iMac.local')
        self.assertEqual(changed['node'], 'IMAC.lan')
        for key in host:
            with self.subTest(field=key):
                self.assertFalse(same_host(host, {**changed, key: 'other'}))
                self.assertFalse(same_host(host, {k: v for k, v in changed.items() if k != key}))
        self.assertFalse(same_host(host, {**changed, 'extra': 'field'}))

    def test_host_identity_without_node_stays_exact(self):
        self.assertTrue(same_host({'recipe': {}}, {'recipe': {}}))
        self.assertFalse(same_host({'recipe': {}}, {'recipe': {'other': True}}))
        self.assertFalse(same_host({}, {'node': 'iMac'}))


if __name__ == '__main__':
    unittest.main()
