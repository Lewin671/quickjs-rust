# Headless Monterey Intel PMU helper v2

[Current diagnosis](../../../archive/T056-kpc-setup-failure.md);
[v1 preparation history](../../../archive/T056-kpc-tool.md).
Engine source/recipe and the last acceptance attempt remain unchanged.

**v1 is installed and sudo works.** Lead153553Z and the authorized short root
probe153934Z fail before the child gate/config verification. Old JSON cannot
identify the specific API or readback comparison; no target or PMU evidence was
collected. v2 requires one user reinstall because sudoers pins the binary digest.
No agent installation or v2 root execution has occurred.

## Changes and remaining uncertainty

- Every setup/drain API/policy failure now records `failed_step`, return code,
  captured errno, completed setup steps, initial trace owner/flags and actual
  readback. Cleanup has separate first-error fields; it never erases setup error.
- Cleanup no longer calls KPERF setters after a failure before KPERF setup: those
  setters can create ownership just to clean a KDEBUG-only session. After our
  KPERF write attempts, reset our action/timer/sampling state with `kperf_reset`,
  remove only our KDEBUG session, verify our ownership was released. Foreign
  owner refusal remains strict. `nolog=0` on an uninitialized buffer is not
  treated as active tracing.
- Default cycles/instructions now use KPEP's named architectural programmable
  fallbacks, USER on/OS off. No assumed mapping from a KPC fixed-config word to
  IA32_FIXED_CTR_CTRL. Four programmable slots hold cycles/instructions+two
  extra events. Exact readback remains mandatory. The old path is available
  only with explicit `--fixed-diagnostic` to distinguish the suspected fixed
  ABI issue after reinstall. That suspicion is **not** a verified root cause.
- Coverage uses `child_window_ns`, from gate release to observed completion,
  excluding PMU ownership setup/cleanup. The old210ms failed-setup interval is
  not target execution and must not be a coverage denominator.

Private interface access still needs real v2 root validation. v1 observations
are in `failure-v1.json`; no specific failed API is invented from its null counts.

## Scope and root boundary

PID-filtered lightweight PET sums each observed thread's first-to-last sampled
interval. JSON always says `full_lifetime_counts:false`,
`all_threads_covered:false`. Short-lived/unobserved threads and lifetime ends
cannot be recovered. No self-thread, whole-CPU or rusage substitute is used.

Only the child calls `execve`, after clearing supplementary groups, setting
nonzero validated SUDO gid/uid, checking all IDs and checking root cannot be
regained. Parent clears environment before loading absolute system frameworks;
child has only TZ=UTC/LC_ALL=C, cwd=/, stdin=/dev/null. All nonstandard actual
FDs close, including FDs above a lowered limit. No shell, arbitrary environment,
register or output-path option. Root paths/ACLs and installed digest are guarded.
stdout/stderr32KiB each, threads/trace records bounded; overflow/drop rejects.
Normal/error cleanup restores our counter snapshot, releases forced counters
and removes only our trace. Signals request cleanup; private alarm bounds runs.
Hard-exit/kernel-owner teardown remains untested as root. No SIP/OS changes.

CLI:

```text
qjs-kpc --events cycles,instructions,EVENT,EVENT -- /absolute/program args...
qjs-kpc --describe --events cycles,instructions,EVENT,EVENT
qjs-kpc --fixed-diagnostic --events cycles,instructions -- /absolute/program args...
qjs-kpc --list-events
```

`--env TZ=UTC`/`--env LC_ALL=C` are the only env flags. `--timeout-ms` is1..10000.
`--list-events` means database presence, not measured-event support. All21
approved encodings/CMask/edge corrections are unchanged;16 auxiliary-MSR
FRONTEND_RETIRED variants and2 absent names remain explicitly unavailable.

## Nonroot verification

`20261021T155348Z-t056-kpc-v2-final` done0/11s: clang14/CLT warnings-as-errors,
PIE/stack protector, seven-module static analysis; prior FD/identity/parser tests,
new trace-backend fault injection/repeated-session/foreign-owner tests, and JSON
programmable-index/setup-vs-cleanup error serialization. Both default2+2 and
explicit fixed2+4 descriptors pass. These are not privileged API/runtime proof.
[qualification-v2.json](qualification-v2.json) preserves exact sources/commands.

`20261021T155433Z-t056-kpc-v2-groups` done0:21 events,11 exact allocation-checked
user-only groups. [groups-v2.json](groups-v2.json). v1 qualification/groups/config
remain frozen historical snapshots. Current render uses `config-v2.json` and
embeds v2 snapshots under the collector's generic filenames.

Reviewed v2 SHA-256:

```text
a8021d9adc7a0c583f32920832fafd34450fe462476b4d2f726a206a83f06673
```

## User reinstall on the iMac

Review the installer, then execute it:

```sh
cat "$HOME/Code/co-work-space/tmp/bench-queue/20261021T155544Z-t056-kpc-v2-install-ready/out/install-user.sh"
sh "$HOME/Code/co-work-space/tmp/bench-queue/20261021T155544Z-t056-kpc-v2-install-ready/out/install-user.sh"
```

It verifies this binary digest, validates sudoers, copies the reviewed executable
root:wheel0755 to `/usr/local/libexec/qjs-kpc`, replaces only its digest-bound
NOPASSWD/NOSETENV rule root:wheel0440, then validates all sudoers. Existing sudo
password is required; install/shell access is not made passwordless. Final rule
passed nonroot `visudo -cf` in155544Z; no installed file was changed by that job.
Uninstall is unchanged:

```sh
sudo /bin/rm -f /private/etc/sudoers.d/qjs-kpc /usr/local/libexec/qjs-kpc
sudo /usr/sbin/visudo -cf /private/etc/sudoers
```

## Short root diagnosis after reinstall, no timing

This authorized debug step uses only the exact installed v2 digest. It runs
`/usr/bin/id -u` with default cycles/instructions, default+DSB and explicit fixed
mode. No engine benchmark. Each call≤5s; failed cleanup stops immediately.
Returns full stage/errno/readback JSON even when setup fails; successful calls
must print the invoking nonroot uid. Insufficient samples from `id` are normal.

```sh
python3 tasks/diagnostics/t056-pmu/kpc-tool/render.py smoke > /tmp/t056-kpc-smoke.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD --name t056-kpc-v2-smoke \
  -- bash -c "$(cat /tmp/t056-kpc-smoke.sh)"
```

Owner may submit only this **non-timing** short debug job, after user reinstall.
The smoke refuses the old installed hash and never edits sudoers or permissions.

## Lead-only single ≤15min measurement, after successful smoke

Six canonical images (f58,8f,count,flags,count_flags,dfff), base/dfff A/A clones,
global/local,11 groups, one paired block and N23,068,672/2N:176 units,
352 measurements+2 real uid/env checks; group parity reverses N/2N order. Groups changed prospectively because
user-filtered cycles/instructions occupy two slots. No formal precision claim.
Old failed run is not retried/relabelled as acceptance; the attempt budget stays.

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

No owner timing. Exact installed hash/receipts/images, counter backend/indices/
registers and actual nonroot uid/env are checked before measurement. Any observed
thread<10 samples, observed span/child-window<0.70, malformed output, permission,
readback, cleanup, deadline or load failure stops without retries/deletion.
Negative rare-event deltas stay negative/uninterpretable. CPI describes sampled
user windows, never whole lifetimes or a JCC verdict. A/A is descriptive only.

Prior workload throughput plus0.211s/root setup per352 calls projects254.431s;
2x+120=628.862s<720 collect/840 body/890 runner/900 watchdog, including quieting.
03–05 host local and15min-before03 excluded,20min admission. Hard timeout remains
authoritative; root throughput is not yet qualified. Partial data is not a decision.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pmu/kpc-tool/validate.py
```

Verifies actual v2 compiled receipts/21 encodings,354 fake calls through the real
collector, missing-install/permission/uid/output/counter/coverage/deadline/load
failures and negative deltas. It never runs sudo or installs anything.
