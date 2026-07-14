"""Deterministic balanced measurement planning."""

from __future__ import annotations

import random
from dataclasses import dataclass


@dataclass(frozen=True)
class PlannedSample:
    block: int
    order: int
    role: str
    case_id: str


def role_orders(roles: list[str], blocks: int, seed: int) -> list[list[str]]:
    if not roles or blocks < 1:
        raise ValueError("at least one role and block are required")
    canonical = [role for role in ("candidate", "base", "quickjs-ng") if role in roles]
    if len(canonical) != len(roles) or len(set(roles)) != len(roles):
        raise ValueError("roles must be unique benchmark roles")
    randomizer = random.Random(seed)
    randomizer.shuffle(canonical)
    offset = randomizer.randrange(len(canonical))
    return [
        canonical[(index + offset + block) % len(canonical):]
        + canonical[:(index + offset + block) % len(canonical)]
        for block in range(blocks)
        for index in [0]
    ]


def measurement_plan(roles: list[str], case_ids: list[str], blocks: int, seed: int) -> list[PlannedSample]:
    if not case_ids or len(set(case_ids)) != len(case_ids):
        raise ValueError("case ids must be non-empty and unique")
    result: list[PlannedSample] = []
    for block, order_roles in enumerate(role_orders(roles, blocks, seed)):
        case_order = list(case_ids)
        random.Random(seed ^ (block * 0x9E3779B1)).shuffle(case_order)
        order = 0
        for role in order_roles:
            for case_id in case_order:
                result.append(PlannedSample(block=block, order=order, role=role, case_id=case_id))
                order += 1
    return result
