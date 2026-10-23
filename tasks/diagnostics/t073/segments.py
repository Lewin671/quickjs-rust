"""Supplemental live rusage segments distinguish startup mode from mid-run drift.

Poll the exact images; preserve all trajectories. This changes the observation
path and is never screen/acceptance evidence. Mach ticks are converted using
the live timebase. End instructions are read before reaping, CPU via wait4.
"""
import ctypes
import dataclasses
import json
import os
from pathlib import Path
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from tools.benchmark import counters, screen

out = Path(sys.argv[1]); out.mkdir()
class Timebase(ctypes.Structure):
    _fields_ = [('numer', ctypes.c_uint32), ('denom', ctypes.c_uint32)]
base = Timebase()
assert ctypes.CDLL(None).mach_timebase_info(ctypes.byref(base)) == 0
scale = base.numer / base.denom
job = Path(os.environ['BENCH_QUEUE']) / '20261023T154044Z-t072-screen-attempt2/out/attempt2'
binaries = {role: json.loads((job / directory / 'identity.json').read_text())['variants'][0]['path']
            for role, directory in [('base', 'base000'), ('candidate', 'cand000')]}
case = screen.resolve_cases(['broad/local_read'], screen.DEFAULT_CACHE, out)[0]
(out / 'request.json').write_text(json.dumps({'timebase': [base.numer, base.denom], 'binaries': binaries, 'count': 46137344, 'poll_seconds': .005}, indent=2) + '\n')
with (out / 'samples.jsonl').open('x') as stream:
    for repeat in range(int(sys.argv[2])):
        for role in (['base', 'candidate'] if repeat % 2 == 0 else ['candidate', 'base']):
            with (out / 'stdout').open('w+') as stdout, (out / 'stderr').open('w+') as stderr:
                start = time.monotonic_ns()
                child = subprocess.Popen(screen._argv(Path(binaries[role]), case, 46137344), stdout=stdout, stderr=stderr)
                trace = []
                while True:
                    exited = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                    info = counters._RusageInfoV4()
                    error = counters._PROC_PID_RUSAGE(child.pid, 4, ctypes.byref(info))
                    if not error:
                        trace.append([time.monotonic_ns() - start, round(info.ri_user_time * scale), info.ri_instructions])
                    if exited is not None:
                        break
                    if time.monotonic_ns() - start > 30e9:
                        child.kill(); raise RuntimeError('segment timeout')
                    time.sleep(.005)
                pid, status, usage = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
                end = time.monotonic_ns()
                stdout.seek(0); text = stdout.read()
                assert child.returncode == 0
                screen._check_output(case, text)
                row = {'repeat': repeat, 'role': role, 'wall_ns': end - start,
                       'user_ns': round(usage.ru_utime * 1e9), 'system_ns': round(usage.ru_stime * 1e9),
                       'minor_faults': usage.ru_minflt, 'major_faults': usage.ru_majflt,
                       'voluntary_switches': usage.ru_nvcsw, 'involuntary_switches': usage.ru_nivcsw,
                       'trace': trace, 'stdout': text}
                stream.write(json.dumps(row) + '\n'); stream.flush()
                print(repeat, role, row['wall_ns'], row['user_ns'], trace[-1][2], flush=True)
