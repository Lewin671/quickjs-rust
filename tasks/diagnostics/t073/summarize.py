"""Descriptive T073 summaries; 1.5x-median tail labels never select samples."""
import collections
import hashlib
import json
from pathlib import Path
import statistics
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from tools.benchmark.screen import parse_macos_time

for name in sys.argv[1:]:
    path = Path(name)
    groups = collections.defaultdict(list)
    for line in path.read_text().splitlines():
        row = json.loads(line)
        result = row['result']
        metrics = parse_macos_time(result['stderr'], cycles=False)
        groups[row.get('mode', 'unspecified'), row['pad'], row['role'], row['count']].append((result['duration_ns'], metrics['user_ns'], metrics['instructions']))
    print(path, 'sha256', hashlib.sha256(path.read_bytes()).hexdigest())
    for key, samples in sorted(groups.items()):
        wall, user, instructions = zip(*samples)
        def spread(values, scale):
            return [round(f(values) / scale, 6) for f in [min, statistics.median, max]]
        print(key, 'n', len(samples), 'wall_s', spread(wall, 1e9),
              'user_s', spread(user, 1e9), 'instructions_G', spread(instructions, 1e9),
              'tails', sum(x > 1.5 * statistics.median(user) for x in user))
