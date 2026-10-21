# Headless Monterey Intel PMU helper

[Evidence and limits](../../../archive/T056-kpc-tool.md). Reviewed source lives
here; no engine files, dependencies, build policy or acceptance attempt changed.
The user approved this narrow helper installation; **the user installs it**.
No root execution, sudoers change or PMU measurement was performed by the agent.

## What it measures

`kpc_get_thread_counters(tid)` cannot read an external child: Monterey XNU rejects
nonzero tid. This implementation uses kperf lightweight PET, a PID-filtered
thread-counter action and an exclusively owned kdebug buffer. It sums each
**observed thread's first-to-last sampled interval**, including every observed
thread or rejecting insufficient/truncated/decreasing data. It cannot prove
that short-lived threads were observed or include their complete lifetimes.
JSON always says `full_lifetime_counts:false`, `all_threads_covered:false`.
This is an explicit limitation relative to the brief's complete-child total;
no self-thread, whole-CPU, rusage or sample-weight substitute is presented.

CLI:

```text
qjs-kpc --events cycles,instructions,EVENT,... -- /absolute/program args...
qjs-kpc --describe --events cycles,instructions,EVENT,...
qjs-kpc --list-events
qjs-kpc --help
```

`--env TZ=UTC` and `--env LC_ALL=C` are the only permitted environment flags;
these two variables are always the entire child's environment. `--timeout-ms`
is1..10000 (default10000); child cwd is `/`, stdin `/dev/null`. Output is one
JSON record containing child stdout/stderr, child status, configured event
names/indices/registers, cycles/instructions, per-thread spans/sample counts,
interval counter sums, coverage limits and cleanup status. Insufficient data
has null counters, not fabricated zeros. stdout/stderr are bounded32KiB each,
FDs/threads/records have fixed bounds; overflow or dropped trace data rejects.
`capture_complete` means child/cleanup completed, not valid counters or coverage.

## Privilege boundary to inspect

- C parent never executes caller input. Only `process.c` calls `execve`, after
  clearing supplementary groups, setting primary gid/uid from nonzero SUDO IDs,
  verifying all real/effective IDs and verifying root cannot be regained.
  SUDO gid must match the account's primary group. Direct root/missing IDs fail.
- Environment is cleared before loading only absolute Apple system frameworks.
  No shell, PATH search, arbitrary library path, event register or output path
  option. All nonstandard FDs are enumerated/closed even when the inherited FD
  number exceeds a lowered RLIMIT_NOFILE. No explicit file writes except stdout;
  core files disabled. The unprivileged command retains its normal file rights.
- Root path ancestors and helper must be root-owned, without group/world write
  or writable allow ACLs. New directory/root:wheel0755, helper0755; not setuid.
  sudoers matches this binary path **and SHA-256**, `NOSETENV`, reset environment.
- Only this CPU (family6/A5/step5), fixed trusted Comet Lake database and21
  crosswalked event names are accepted for measurement. Raw-register CLI absent.
  Fixed0/1 use user-only0x22; programmed events clear OS/AnyThread/PMI bits.
  Named CMask/edge corrections are static in `events-approved.h` and checked
  against Intel's encodings. Readback must match before child release.
- Refuses an existing trace owner, sampling, forced counters or active action/
  timer. Acquires ktrace without first resetting it; only own session is removed.
  Saves/restores prior counter configuration/classes and releases forced counters
  on normal/error exits. Signals request cleanup; a private alarm kills the
  blocked child and has a second-stage hard exit. SIGKILL/hard-exit restoration
  relies on kernel owner teardown and is **not privileged-tested**.
- No child descendants are measured. Arbitrary descendant/thread lifecycle
  completeness is not implemented. This is a diagnostic helper, not a daemon.

## Verified without root

`20261021T145905Z-t056-kpc-tool-audit-final`: done0/7s on iMac20,1,
macOS12.6, i7-10700, Apple clang14.0.0/CommandLineTools. Warnings-as-errors,
stack protector/PIE, all seven C modules' static analyzer, `--help`, database
listing, nonroot/invalid-env refusal,64-bit/interleaved/incomplete trace tests,
FD-above-lowered-limit closure and mock credential-success/every-failure tests.
No root behavior is claimed from mocks. `qualification.json` seals sources,
compiler command/output and exact executable SHA-256:

```text
e3d2bc4c7af7dab8c1181b78ca291429c9ef275e44b20a828e1ca01f1a7ec065
```

`20261021T145945Z-t056-kpc-groups-final`: done0,6 groups/21 events, all exact
config words/indices checked without PMU access. `groups.json` and
`event-inventory.json` retain all supported/unavailable names.16 FRONTEND_RETIRED
names need auxiliary MSR/PEBS selectors unsupported here. UOPS_RETIRED.RETIRE_SLOTS
and RESOURCE_STALLS.ANY are absent; no silent alias or guessed encoding.
`--list-events` reports database presence, **not** support for measurement.

## Exact user installation on the iMac

The prepared assets are already on the iMac. Read the14-line installer, then run:

```sh
cat "$HOME/Code/co-work-space/tmp/bench-queue/20261021T150720Z-t056-kpc-install-ready/out/install-user.sh"
sh "$HOME/Code/co-work-space/tmp/bench-queue/20261021T150720Z-t056-kpc-install-ready/out/install-user.sh"
```

It verifies the binary digest, validates the rule with `sudo visudo -cf`, copies
only the reviewed binary to `/usr/local/libexec/qjs-kpc` root:wheel0755 and the
rule to `/private/etc/sudoers.d/qjs-kpc` root:wheel0440, then validates the entire
sudoers configuration. It prompts for the user's existing sudo password; it
never makes `install`, a shell or another executable passwordless. `/usr/local`
was verified root:wheel0755 on this iMac; installer and runtime reject unsafe
ownership/modes rather than changing that ancestor. Runtime also checks ACLs.

The exact rule is [qjs-kpc.sudoers](qjs-kpc.sudoers). Its final form passed
**nonroot** `/usr/sbin/visudo -cf` in job150720Z; no installed rule was changed.
User-side root validation still runs in the installer.

Uninstall only these two installed files:

```sh
sudo /bin/rm -f /private/etc/sudoers.d/qjs-kpc /usr/local/libexec/qjs-kpc
sudo /usr/sbin/visudo -cf /private/etc/sudoers
```

## Lead-only single measurement job, ≤15min

After user installation, the job first verifies the installed hash, frozen unit/
original correctness+86-case receipts and all six canonical images. Under the
quiet-host measurement lock it runs `/usr/bin/id -u` and `/usr/bin/env` through
the helper, checks actual nonroot uid/fixed environment/config readback/cleanup,
then profiles global_read/local_read for base,8f,count,flags,count_flags,dfff
plus separate-inode base/dfff A/A copies. Six event groups, two paired blocks,
N=23,068,672 and2N:192 units/384 measurements+2 privilege checks. No builds.

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

No owner submission. Privileged access remains unverified until the first real
root checks. Missing install, permission, wrong uid/env/output, invalid config,
truncated/dropped data, fewer than10 samples in any observed thread, or observed
span/helper interval below0.70 stops the whole job without retries/selection.
Raw JSON is retained. N/2N startup subtraction is of **sampled intervals**, not
complete lifetimes. Negative rare-event deltas remain negative/uninterpretable;
no clamp. CPI is observed-window user-space cycles/instructions. A/A is a
10% descriptive guard; no formal acceptance, PMU cause or JCC verdict.

Previous measured throughput projects196.537s;2x+120=513.074s<720 collection.
Body840/runner890/watch900s include quieting, exclude queue wait. Host-local
03–05 plus15min-before03 excluded;20min admission. The helper itself bounds each
run with its alarm. Timeout, load disturbance or malformed output preserves
partial evidence and stops. No security settings or SIP changes.

Recheck locally, using fetched named outputs:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pmu/kpc-tool/validate.py
```

It verifies actual compiled-source/binary/encoding receipts and386 fake helper
calls through the real collector, zero-call missing-install rejection, credential/
permission/coverage/output/counter/timeout/load/deadline failures, negative
counter preservation and shell syntax. It never invokes sudo or installs files.
