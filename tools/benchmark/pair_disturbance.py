"""Load-only pair eligibility for the existing site's timestamped monitor.

The collector retains every attempt; replay reconstructs eligibility before
reading costs. The monitor itself, CPU threshold and exemptions stay external.
"""
from __future__ import annotations

import datetime
import hashlib
import os
import time
from pathlib import Path

PROTOCOL = {'version': 1, 'guard_seconds': 2, 'timestamp_bucket_seconds': 1,
            'settle_seconds': 8, 'cpu_threshold': 200, 'invalid_fraction_cap': .2,
            'heavy_cluster_seconds': 60, 'sample_gap_seconds': 6}


def heavy_samples(text, anchor):
    """Decode UTC-second buckets, including midnight, summing all intruders."""
    day = int(anchor // 86400) * 86400
    totals = {}
    previous = anchor - 86400
    for line in text.splitlines():
        fields = line.split()
        if len(fields) != 3:
            raise ValueError('malformed load monitor row')
        clock = datetime.time.fromisoformat(fields[0])
        stamp = day + clock.hour * 3600 + clock.minute * 60 + clock.second
        if stamp < anchor - 43200:
            stamp += 86400
        if stamp > anchor + 86400 or stamp < previous:
            raise ValueError('load monitor timestamps out of range/order')
        cpu = float(fields[1])
        if not 30 <= cpu < float('inf'):
            raise ValueError('invalid load monitor CPU')
        previous = stamp
        totals[stamp] = totals.get(stamp, 0) + cpu
    return sorted(t for t, cpu in totals.items() if cpu >= PROTOCOL['cpu_threshold'])


def invalid_window(start, end, heavy):
    if not 0 < start <= end:
        raise ValueError('invalid pair window')
    guard = PROTOCOL['guard_seconds']
    return [t for t in heavy if t <= end + guard and t + 1 >= start - guard]


def check_cap(validity, heavy):
    if validity and validity.count(False) / len(validity) > PROTOCOL['invalid_fraction_cap']:
        raise ValueError('DISTURBED: invalid pairs exceed 20% cap')
    first = previous = None
    for stamp in heavy:
        if previous is None or stamp - previous > PROTOCOL['sample_gap_seconds']:
            first = stamp
        if stamp - first >= PROTOCOL['heavy_cluster_seconds']:
            raise ValueError('DISTURBED: heavy load cluster spans >=60 seconds')
        previous = stamp


class Monitor:
    def __init__(self, path):
        self.path = path
        self.anchor = time.time()

    @classmethod
    def from_queue(cls):
        out = os.environ.get('BENCH_OUT')
        if not out:
            raise ValueError('layout measurement requires queue BENCH_OUT/load monitor')
        return cls(Path(out).parent / 'intruders.log')

    def snapshot(self):
        # Observe beyond the last timed window and guard, allowing one full
        # monitor period plus integer-second timestamp quantization.
        time.sleep(PROTOCOL['settle_seconds'])
        text = self.path.read_text()
        return {'protocol': PROTOCOL, 'anchor': self.anchor, 'text': text,
                'sha256': hashlib.sha256(text.encode()).hexdigest(), 'sealed_at': time.time()}


def eligibility(attempts, snapshot):
    if snapshot['protocol'] != PROTOCOL or hashlib.sha256(snapshot['text'].encode()).hexdigest() != snapshot['sha256']:
        raise ValueError('load snapshot protocol/hash mismatch')
    if attempts and snapshot['sealed_at'] < attempts[-1]['ended_at'] + PROTOCOL['settle_seconds']:
        raise ValueError('load snapshot does not cover guard/monitor completion')
    heavy = heavy_samples(snapshot['text'], snapshot['anchor'])
    flags = [not invalid_window(a['started_at'], a['ended_at'], heavy) for a in attempts]
    check_cap(flags, heavy)
    return flags


def verify_run(packets, final):
    """Bind each packet to the same append-only run log and enforce global cap."""
    flags = []
    for packet in packets:
        for round_ in packet['rounds']:
            snap = round_['load']
            if snap['anchor'] != final['anchor'] or not final['text'].startswith(snap['text']):
                raise ValueError('case load history differs from sealed run log')
        current = eligibility(packet['attempts'], final)
        if [i for i, flag in enumerate(current) if flag] != packet['valid_attempt_indices']:
            raise ValueError('sealed run load changes pair eligibility')
        flags.extend(current)
    heavy = heavy_samples(final['text'], final['anchor'])
    check_cap(flags, heavy)
    return {'attempts': len(flags), 'valid': sum(flags), 'invalid': flags.count(False),
            'heavy_samples': len(heavy)}


def quieting_preflight():
    """Record user-managed host quieting; never change OS services here."""
    import subprocess
    indexing = subprocess.check_output(['mdutil', '-s', '/'], text=True)
    disabled = subprocess.check_output(['launchctl', 'print-disabled', f'gui/{os.getuid()}'], text=True)
    if 'Indexing disabled.' not in indexing:
        raise ValueError('Spotlight indexing is not disabled: ' + indexing.strip())
    for service in ('com.apple.mediaanalysisd', 'com.apple.photoanalysisd'):
        if f'"{service}" => true' not in disabled:
            raise ValueError('expected user-approved disabled service: ' + service)
    return {'indexing': indexing, 'disabled_services': disabled}
