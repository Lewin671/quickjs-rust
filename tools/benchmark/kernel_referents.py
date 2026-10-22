"""Bounded, relocation-aware referent graphs for the amended K0 contract.

Only dyld-declared native pointer slots are normalized; all other bytes stay
raw. Data/protected-code edges carry identity and bounded target contents;
confirmed code outside the protected set is an identity-only leaf.
Breadth-first traversal checks every reachable node once, including cycles.
Unknown classification/partial slots or new nodes beyond MAX_DEPTH fail closed.
Import leaves require exact dyld bindings proven to be code by an x86 stub.
"""
import bisect
import hashlib
import json
import struct

MAX_DEPTH = 2
CONTRACT = {'pointer_slots': 'dyld64-v2', 'max_depth': MAX_DEPTH,
            'external_code': 'identity-only', 'protected_code': 'full'}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


class Referents:
    def __init__(self, image):
        self.image = image
        self.starts = sorted(image.pointer_fixups)
        self.cache, self.nodes = {}, {}

    def node(self, address, width):
        key = (address, width)
        if key in self.nodes:
            return self.nodes[key]
        image = self.image
        raw = image.raw(address, width)
        normalized = bytearray(raw)
        left = bisect.bisect_right(self.starts, address-8)
        right = bisect.bisect_left(self.starts, address+width)
        slots, values = [], []
        for slot in self.starts[left:right]:
            if slot < address or slot+8 > address+width:
                raise ValueError('partially covered referent pointer slot')
            fixup = image.pointer_fixups[slot]
            value = struct.unpack('<Q', image.raw(slot, 8))[0]
            at = slot-address
            normalized[at:at+8] = b'\0'*8
            target = None
            if fixup['kind'] == 'bind':
                classification = image.classify_import(fixup)
                identity = {'import': fixup['symbol'], 'library': fixup['library'],
                            'offset': fixup['addend'], 'flags': fixup['flags']}
            elif fixup['kind'] == 'rebase':
                identity, target_width = image.referent_target(value)
                classification = image.classify_target(value)
                if classification in ('data', 'protected-code'):
                    target = (value, target_width)
                elif classification != 'external-code':
                    raise ValueError('unknown referent target classification')
            else:
                raise ValueError('unknown referent pointer relocation')
            slots.append({'offset': at, 'identity': identity, 'target': target,
                          'classification': classification})
            values.append({'offset': at, 'value': value})
        result = {'width': width, 'non_pointer_sha256': hashlib.sha256(normalized).hexdigest(),
                  'pointers': slots, 'values': values, 'raw_sha256': hashlib.sha256(raw).hexdigest()}
        self.nodes[key] = result
        return result

    def snapshot(self, address, width):
        root = (address, width)
        if root in self.cache:
            return self.cache[root]
        queue, seen, nodes, values = [(root, 0)], {root: 'root'}, {}, {}
        index = 0
        while index < len(queue):
            physical, depth = queue[index]
            index += 1
            node = self.node(*physical)
            pointers = []
            for slot in node['pointers']:
                child = slot['target']
                edge = {'offset': slot['offset'], 'identity': slot['identity'],
                        'classification': slot['classification']}
                if child is None:
                    edge['target'] = {'identity_only': True}
                else:
                    if child not in seen:
                        if depth >= MAX_DEPTH:
                            raise ValueError('referent pointer depth budget exhausted')
                        seen[child] = digest({'identity': slot['identity'], 'width': child[1]})
                        queue.append((child, depth+1))
                    edge['target'] = {'node': seen[child], 'width': child[1]}
                pointers.append(edge)
            name = seen[physical]
            if name in nodes:
                raise ValueError('ambiguous referent graph identity')
            nodes[name] = {'width': node['width'], 'non_pointer_sha256': node['non_pointer_sha256'],
                           'pointers': pointers}
            values[name] = node['values']
        content = {'root': 'root', 'nodes': nodes}
        result = {'sha256': digest(content), 'content': content,
                  'raw_sha256': self.nodes[root]['raw_sha256'],
                  'pointer_values_sha256': digest(values),
                  'pointer_count': sum(len(n['pointers']) for n in nodes.values())}
        self.cache[root] = result
        return result
