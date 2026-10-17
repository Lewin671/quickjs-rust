"""Append load-invalid pair rounds without selecting on measured values."""
from __future__ import annotations

import dataclasses
import json
import time

from . import screen
from .layout_identity import write_json
from .pair_disturbance import Monitor, eligibility


def measure(case, candidate, base, pairs, tool, timeout, output, monitor=None, *, iterations=None, pair_offset=0):
    monitor = monitor or Monitor.from_queue()
    calibration, attempts, rounds = [], [], []
    with output.with_suffix('.jsonl').open('x', encoding='utf-8') as stream:
        def run(binary, count, target):
            argv = screen._argv(binary, case, count)
            start = time.time()
            value = screen.sample(tool, argv, timeout)
            row = {'argv': argv, 'sample': dataclasses.asdict(value),
                   'started_at': start, 'ended_at': time.time()}
            target.append(row)
            stream.write(json.dumps(row) + '\n'); stream.flush()
            screen._check_output(case, value.stdout)
            return value

        if case.kind == 'internal' and iterations is None:
            iterations = screen.calibrate(lambda count: run(base, count, calibration))
        missing = pairs
        try:
            while missing:
                for _ in range(missing):
                    index = pair_offset + len(attempts)
                    rows = []
                    for role in (('base', 'candidate') if index % 2 == 0 else ('candidate', 'base')):
                        binary = base if role == 'base' else candidate
                        for count in ((iterations, iterations * 2) if iterations else (None,)):
                            run(binary, count, rows)
                    attempts.append({'index': index, 'started_at': rows[0]['started_at'],
                                     'ended_at': rows[-1]['ended_at'], 'samples': rows})
                snapshot = monitor.snapshot()
                rounds.append({'attempt_count': len(attempts), 'load': snapshot})
                flags = eligibility(attempts, snapshot)
                missing = pairs - sum(flags)
        except Exception as error:
            write_json(output, {'id': case.id, 'kind': case.kind, 'iterations': iterations,
                'calibration': calibration, 'attempts': attempts, 'rounds': rounds, 'pair_index_offset': pair_offset,
                'complete': False, 'error': str(error)})
            raise
    packet = {'id': case.id, 'kind': case.kind, 'iterations': iterations,
              'calibration': calibration, 'attempts': attempts, 'rounds': rounds, 'pair_index_offset': pair_offset,
              'complete': True, 'valid_attempt_indices': [i for i, flag in enumerate(flags) if flag]}
    # Costs are constructed only after load-only eligibility is sealed. Keep
    # correctness failures and short/nonpositive increments fatal, never retry.
    from .screen_replay import replay_attempts
    try:
        costs, ratios, measured = replay_attempts(packet, pairs,
            {'base': str(base), 'candidate': str(candidate)})
        packet.update(costs=costs, ratios=ratios, samples=measured)
    except Exception as error:
        packet.update(complete=False, error=str(error))
        write_json(output, packet)
        raise
    write_json(output, packet)
    return screen.CaseResult(case.id, case.kind, iterations, ratios)
