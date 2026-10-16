"""Frozen layout policy/measurement contract carried by schema-3 unit plans."""
from __future__ import annotations

from .performance_schema import _keys, _object, _sha256, _string, _unique_strings, _opportunity_id


def validate(unit):
    gate = _object(unit['layout_gate'], 'layout_gate')
    _keys(gate, {'policy_path', 'policy_sha256', 'mechanism_class', 'memory_wall_targets',
                 'panel_required', 'panel_pairs', 'held_out_pairs', 'formal_blocks'}, 'layout_gate')
    _string(gate['policy_path'], 'layout_gate.policy_path')
    from pathlib import PurePosixPath
    path = PurePosixPath(gate['policy_path'])
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('policy path must be repository-relative')
    _sha256(gate['policy_sha256'], 'layout_gate.policy_sha256')
    if gate['mechanism_class'] not in ('cpu-work', 'instruction-blind'):
        raise ValueError('layout mechanism_class must be cpu-work or instruction-blind')
    raw_ids = gate['memory_wall_targets']
    if not isinstance(raw_ids, list):
        raise ValueError('memory_wall_targets must be an array')
    ids = _unique_strings(raw_ids, 'layout memory targets', _opportunity_id) if raw_ids else []
    if not set(ids) <= set(unit['fast_gate']['target_ids']):
        raise ValueError('memory wall targets must be declared targets')
    if type(gate['panel_required']) is not bool:
        raise ValueError('layout panel_required must be boolean')
    if gate['mechanism_class'] == 'instruction-blind' and (not ids or not gate['panel_required']):
        raise ValueError('instruction-blind mechanisms require memory wall targets and K=6')
    if gate['panel_pairs'] != 5 or gate['held_out_pairs'] != 30 or gate['formal_blocks'] not in (30, 60):
        raise ValueError('freeze 5-pair K=6, 30-pair held-out and 30/60-block formal budgets')
    return gate
