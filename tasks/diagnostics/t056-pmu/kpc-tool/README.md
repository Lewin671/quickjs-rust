# Headless Monterey Intel PMU helper v3

[Current record](../../../archive/T056-kpc-counting.md);
[v2/PET history](../../../archive/T056-kpc-setup-failure.md).
Diagnostic tooling only; engine, recipe, frozen plans and last attempt unchanged.

v2 is installed and its real root config/count setup works. It cannot reliably
acquire ktrace from launchd-managed tailspind. v3 removes all KDEBUG/PET/sampler
calls and uses KPC all-CPU counting. **Tailspin can be re-enabled now**; no daemon
stop or OS setting change is required. A foreign forced/programmed PMU user is
refused without reset. Actual v3 root getter/cleanup awaits install and smoke;
source analysis and nonroot tests are not hardware proof.

## Counter scope and lifecycle

Six production C modules load only KPC/KPEP APIs. KPEP USER-only configurable
classes2 supplies cycles/instructions plus two extras in four slots. No assumed
fixed-control ABI, thread counting, PET actions/timers, ktrace ownership or trace
buffer. `aggregate.c` remains only for the v2 parser regression test, not linked
into the installed executable. The approved21 events and18 exclusions remain.

Fork the child before claiming counters; it drops credentials and waits at a
pipe gate. Parent snapshots the original programmable config/counting mask,
claims forced counters, programs audited events, verifies exact readback and
starts counting. Read all CPUs before releasing the gate and after observing
child exit. Then take another pair of snapshots around a child-free sleep of
that child's observed duration. Cleanup on success/failure clears counting,
restores the original config and any pre-existing unforced fixed-only mask,
releases force and checks released/restored state. Foreign programmable/thread
counting is refused. Each cleanup operation is tried despite an earlier error;
`cleanup_ok:false` stops the protocol. Signals request cleanup; internal alarms
bound the child+idle pair and have a final hard-exit backstop.

The all-CPU private ABI is bounded by256 CPUs ×32 counters, with hw.ncpu equal
to hw.logicalcpu_max and stable shape each read. Sentinel checks reject a missing
or wider payload. Falling counters, changed shape and sum overflow reject counts.
JSON preserves four raw per-CPU snapshots with read begin/end timestamps, summed
and per-CPU event deltas, matched idle deltas, original mask, readback, child
stdout/status and separate first setup/cleanup errors. Reads are sequential
cross-CPU snapshots, not an atomic simultaneous sample.

`scope:system_wide_user_cpu_delta`, `pid_filtered:false`: background processes
and the helper's user-space polling are included. This is not child attribution.
The collector reports duration-normalized idle fractions for every event, raw
N/2N increments, idle-adjusted increments and CPI separately. The5% cycle/
instruction idle flag is a prospective descriptive qualification, not assumed
quietness or acceptance. Negative/oversized idle fractions and negative rare
increments are preserved. Idle subtraction cannot recover PID attribution or
perfectly remove time-varying background/helper polling. No DSB/JCC causal claim.

Root boundary unchanged: validated nonzero SUDO_UID/GID matching account primary
group, root-owned install paths and ACL checks, fixed absolute framework/program
paths, empty parent environment, child TZ=UTC/LC_ALL=C, cwd=/, stdin=/dev/null,
no supplementary groups or regained root, close actual inherited FDs, bounded
stdout/stderr, no output-file or raw-register options, no shell API. The helper
writes only pipes/stdout; the nonroot collector owns artifacts. Timeout1..10000ms
includes setup/child, followed by at most a matched idle window; parent alarm
is2×ceil(timeout seconds)+2, emergency exit2s later. Collector allows26s/call.

```text
qjs-kpc --events cycles,instructions,EVENT,EVENT -- /absolute/program args...
qjs-kpc --describe --events cycles,instructions,EVENT,EVENT
qjs-kpc --list-events
```

Only `--env TZ=UTC`/`--env LC_ALL=C` are accepted. Fixed diagnostic mode is removed.
See the [source record](source-references-v3.json) for Monterey root-read-check
and all-CPU ABI references, and the archive for the v2 failure explanation.

## Verified preparation

Non-timing `20261021T170020Z-t056-kpc-v3-qualified` done0/10s:
clang14/CLT strict warnings/PIE/stack protector, six-module static analysis,
credential/FD/parser tests, real nonroot process gate/env/timeout/output-overflow
cleanup tests, counter API fault injection and JSON serialization. Every injected
setup/read failure releases successfully acquired counters in the mock. Foreign
ownership causes no writes; baseline restoration, repeated sessions, shape,
decreasing/overflow counters and cleanup errors are tested. This does not prove
kernel cleanup under real root; failed cleanup is explicitly invalid.

[qualification-v3.json](qualification-v3.json), [groups-v3.json](groups-v3.json)
(job170125Z done0), [validation-v3.json](validation-v3.json) (354 fake helper calls
plus8 nonroot warmups), [install-validation-v3.json](install-validation-v3.json).
v1/v2 snapshots remain frozen. Current rendering embeds v3 snapshots under the
collector's generic filenames. Binary SHA-256:

```text
4b5776d1126490ddd9d3b6b8394cbe9b43c3406752534fb0704c501cfb81eafe
```

## User reinstall on the iMac

Review and run the prepared installer (requires the user's sudo password):

```sh
cat "$HOME/Code/co-work-space/tmp/bench-queue/20261021T170331Z-t056-kpc-v3-install-ready/out/install-user.sh"
sh "$HOME/Code/co-work-space/tmp/bench-queue/20261021T170331Z-t056-kpc-v3-install-ready/out/install-user.sh"
```

It verifies the reviewed binary, validates the rule, installs root:wheel0755
`/usr/local/libexec/qjs-kpc` and digest-bound NOSETENV/NOPASSWD rule0440, and checks
sudoers. No agent/job performs installation or broadens shell/install privilege.
Uninstall: remove `/private/etc/sudoers.d/qjs-kpc` and `/usr/local/libexec/qjs-kpc`
with sudo, then run `sudo /usr/sbin/visudo -cf /private/etc/sudoers`.

## Short root smoke after reinstall, no timing

Owner may submit this debug-only job after exact digest installation. Two
`id -u` captures (cycles/instructions and +DSB) require successful all-CPU getter,
valid counters, config readback, correct nonroot UID and cleanup. No benchmark.
Any failed capture/cleanup stops; never reset a foreign profiler or kill daemons.

```sh
python3 tasks/diagnostics/t056-pmu/kpc-tool/render.py smoke > /tmp/t056-kpc-smoke.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD --name t056-kpc-v3-smoke \
  -- bash -c "$(cat /tmp/t056-kpc-smoke.sh)"
```

## Lead-only single ≤15min measurement, after successful smoke

Six canonical images+base/dfff A/A clones, global/local,11 groups, one block,
N11,534,336/2N:176 units,352 measured child/idle pairs,2 uid/env checks and8
unprivileged warmups. Group parity reverses N/2N; each role uses the same active
path and independent inode. This is prospective diagnostic evidence only.

```sh
python3 tasks/diagnostics/t056-pmu/kpc-tool/render.py > /tmp/t056-kpc-counters.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD \
  --with f58c696e8379261ed726f0cea1051ed6d34f431a \
  --with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a \
  --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 \
  --name t056-kpc-counters --timing \
  -- bash -c "$(cat /tmp/t056-kpc-counters.sh)"
python3 tasks/diagnostics/t056-pmu/kpc-tool/watch.py JOB_ID
```

Source/images/helper/receipts/frozen plan/correctness/86-case seals remain. CPU
matrix sums, raw monotonic deltas, config/readback and idle/work midpoint-span
ratio0.95..1.05 are checked. Background>5% is retained/flagged, not silently
removed or called small. Malformed rows, permissions, UID/output, counter shape,
cleanup, deadline or load failure stops with all partial evidence, no retries.

Halved work with matched idle preserves prior projected workload180.159s;
354×0.211s setup,2× reserve+120s preparation+5s warmups gives634.706s<720 collect.
Root throughput is unverified; deadlines override projections. Body840/runner890/
watch900 include quieting,03–05 host-local/15min-before03 exclusion and20min
admission remain. Owner never submits timing or consumes an acceptance attempt.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pmu/kpc-tool/validate.py
```
