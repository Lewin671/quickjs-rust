#!/usr/bin/env python3
"""Paired whole-process wall/RSS diagnostics for the supplied review programs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import threading
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', action='append', required=True, help='LABEL=PATH')
parser.add_argument('--pairs', type=int, default=3)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--case', action='append')
args = parser.parse_args()
if args.output.exists():
    parser.error(f'output already exists: {args.output}')


def competing_processes():
    snapshot = subprocess.run(['ps', '-axo', 'pid=,ppid=,stat=,comm=,args='],
        capture_output=True, text=True, check=True, timeout=5)
    processes = {}
    for line in snapshot.stdout.splitlines():
        fields = line.split(None, 4)
        if len(fields) == 5 and fields[0].isdigit() and fields[1].isdigit():
            pid, parent, state, command, arguments = fields
            processes[int(pid)] = (int(parent), state, Path(command).name, arguments)
    busy = []
    names = {'cargo', 'rustc', 'clang', 'clang++', 'cc', 'gcc', 'ld', 'lld',
             'cmake', 'make', 'qjs'}
    runners = {'test262-baseline.sh', 'test262-subset.sh', 'compare-qjs.sh', 'check.sh'}
    for pid, (_, state, name, arguments) in processes.items():
        argv = arguments.split()
        runner = name in {'bash', 'sh'} and len(argv) > 1 and Path(argv[1]).name in runners
        if (name not in names and not runner) or state.startswith('Z'):
            continue
        ancestor = pid
        seen = set()
        while ancestor in processes and ancestor not in seen and ancestor != os.getpid():
            seen.add(ancestor)
            ancestor = processes[ancestor][0]
        if ancestor != os.getpid():
            busy.append({'pid': pid, 'name': name})
    return busy


load_events = []
stopped = threading.Event()


def watch_load():
    previous = []
    while not stopped.is_set():
        try:
            busy = competing_processes()
        except (OSError, subprocess.SubprocessError, ValueError) as error:
            load_events.append({'monitor_error': str(error)})
            return
        if busy and busy != previous:
            load_events.append({'monotonic_s': time.monotonic(), 'processes': busy})
        previous = busy
        stopped.wait(0.5)


# Require an initially quiet host; a later disturbance invalidates the whole
# diagnostic, without replacing individual samples or selecting faster results.
for _ in range(10):
    busy = competing_processes()
    if busy:
        raise SystemExit(f'host busy before collection: {busy}')
    time.sleep(0.5)
monitor = threading.Thread(target=watch_load, daemon=True)
monitor.start()
programs = Path(__file__).parent
cases = args.case or ['mem1', 'mem2', 'mem3', 'mem4', 'mem5', 'mem6', 'mem7',
                      'rep', 'casc1', 'casc2', 'ch', 'deepcls']
binaries = dict(item.split('=', 1) for item in args.binary)
report = {'binaries': {label: {'path': str(Path(path).resolve()),
    'sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest()}
    for label, path in binaries.items()}, 'pairs': args.pairs, 'cases': {},
    'load_monitor_interval_s': 0.5}
for case in cases:
    program = programs / f'{case}.js'
    results = {label: [] for label in binaries}
    for pair in range(args.pairs):
        labels = list(binaries)
        if pair % 2:
            labels.reverse()
        for label in labels:
            start = time.monotonic()
            run = subprocess.run(['/usr/bin/time', '-l', binaries[label], str(program)],
                capture_output=True, text=True, timeout=240)
            elapsed = time.monotonic() - start
            rss = re.search(r'(\d+)\s+maximum resident set size', run.stderr)
            if run.returncode or not rss:
                raise RuntimeError(f'{case}/{label}: exit {run.returncode}\n{run.stderr}')
            results[label].append({'wall_s': elapsed, 'rss_bytes': int(rss[1]),
                                   'stdout': run.stdout.strip()})
    entry = {'sha256': hashlib.sha256(program.read_bytes()).hexdigest(),
             'samples': results, 'median': {}}
    for label, samples in results.items():
        entry['median'][label] = {metric: statistics.median(s[metric] for s in samples)
                                 for metric in ['wall_s', 'rss_bytes']}
    report['cases'][case] = entry
    print(case, json.dumps(entry['median']), flush=True)
stopped.set()
monitor.join()
report['load_events'] = load_events
report['valid_quiet_host'] = not load_events
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2) + '\n')
if load_events:
    raise SystemExit('collection disturbed; retain this report and replace the whole run')
