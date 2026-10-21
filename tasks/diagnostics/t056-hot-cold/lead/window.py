"""Own-process deadline guard: never collect through iMac local 03:00-05:00."""
import datetime
import os
import signal
import subprocess
import sys
import time
import zoneinfo

# The uv Python on the iMac has no time.tzset(); read the system zone directly so
# an inherited TZ (e.g. UTC) cannot shift the 03:00-05:00 window.
with open('/etc/localtime', 'rb') as _zone_file:
    HOST_ZONE = zoneinfo.ZoneInfo.from_file(_zone_file, key='host')


def host_local(stamp):
    return datetime.datetime.fromtimestamp(stamp, HOST_ZONE)


def deadline_for(now):
    if 3 <= host_local(now).hour < 5:
        raise ValueError('iMac local 03:00-05:00 is excluded')
    # Epoch scan respects the host's DST transitions; stop 15 minutes before
    # its next 03:00, normally 02:45 local. A 26-hour search covers DST fall-back.
    for minutes in range(1, 26 * 60 + 1):
        stamp = (int(now) // 60 + minutes) * 60
        local = host_local(stamp)
        if local.hour == 3 and local.minute == 0:
            return stamp - 15 * 60
    raise ValueError('cannot determine next iMac quiet-window boundary')


def tree(root):
    rows = [tuple(map(int, line.split())) for line in subprocess.check_output(['ps', '-axo', 'pid=,ppid='], text=True).splitlines()]
    owned = {root}
    while True:
        expanded = owned | {pid for pid, parent in rows if parent in owned}
        if expanded == owned:
            return owned
        owned = expanded


def signal_owned(pids, signum):
    for pid in sorted(pids, reverse=True):
        try:
            os.kill(pid, signum)
        except ProcessLookupError:
            pass


def stop_tree(root):
    owned = set()
    for _ in range(8):
        current = tree(root)
        signal_owned(current, signal.SIGSTOP)
        if current <= owned:
            break
        owned |= current
    signal_owned(owned, signal.SIGTERM)
    signal_owned(owned, signal.SIGCONT)
    time.sleep(0.25)
    signal_owned(owned, signal.SIGKILL)


def run(body, headroom, arguments):
    now = time.time()
    deadline = deadline_for(now)
    if deadline - now < headroom:
        raise ValueError('insufficient iMac local window: need %.1f minutes before %s' %
                         (headroom / 60, host_local(deadline)))
    print('iMac local start', host_local(now), 'deadline', host_local(deadline), flush=True)
    proc = subprocess.Popen(['bash', '-eo', 'pipefail', '-c', body, '--', *arguments], start_new_session=True)
    try:
        code = proc.wait(timeout=max(0, deadline - time.time()))
    except subprocess.TimeoutExpired:
        print('error: iMac deadline; whole job incomplete, retain partial evidence but do not decide from it', file=sys.stderr, flush=True)
        stop_tree(proc.pid)
        proc.wait()
        return 124
    return code if code >= 0 else 128 - code
