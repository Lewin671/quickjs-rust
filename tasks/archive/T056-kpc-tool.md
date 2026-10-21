# T056 headless PMU helper preparation

- Status: closed preparation; privileged collection awaits user installation.
- Implementation: standalone C helper and lead-only protocol in
  [kpc-tool](../diagnostics/t056-pmu/kpc-tool/README.md); no engine changes.
- Verified at: engine dfff3bb6; diagnostic parent0058895e; iMac nonroot
  qualification20261021T145905Z done0/7s, grouping145945Z done0,
  install assets150720Z done0/visudo parsedOK. No privileged execution.
- Evidence: [qualification](../diagnostics/t056-pmu/kpc-tool/qualification.json),
  [event inventory](../diagnostics/t056-pmu/kpc-tool/event-inventory.json),
  [groups](../diagnostics/t056-pmu/kpc-tool/groups.json),
  [validation](../diagnostics/t056-pmu/kpc-tool/validation.json).
- Unresolved: root API access/readback/cleanup and observed coverage unverified;
  exact complete-child/all-thread lifetime totals are not implemented.
- Next action: user installs reviewed digest-bound helper; lead runs prepared
  ≤15min diagnostic, which begins with real uid/environment/PMU checks.
  No owner timing, automatic installation, SIP change or attempt consumption.

## Interface feasibility and scope

The brief authorized a small auditable root helper without Xcode. Monterey's
[XNU kern_kpc.c](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/bsd/kern/kern_kpc.c)
rejects nonzero tid in `sysctl_kpc_get_thread_counters`. A root parent reading
its own thread is not evidence about an exec'd child. The external-process
[ibireme demo](https://gist.github.com/ibireme/173517c208c7dc333ba962c1f0d67d12)
uses PID-filtered kperf/PET samples and sums first/last observed thread counts.
The tool follows that mechanism, not the self-thread demo. Local bounds/parser/
privilege handling are implemented independently; reference is public domain.

It includes every observed thread or rejects truncated/incomplete/decreasing
samples, preserving64-bit thread IDs and records split across reads. It cannot
prove every short-lived thread was sampled, or include unsampled beginnings and
ends. JSON explicitly marks `all_threads_covered:false` and
`full_lifetime_counts:false` on every run. This is a documented deviation from
the requested complete-child total; no claim that this unavailable interface
was implemented. An exact whole-lifetime requirement would need a different
kernel/backend mechanism. No engine instrumentation or injected library added.

## Root boundary and installation authorization

The user approved preparation for a user-installed root-owned fixed path with
a narrow NOPASSWD rule. The agent has not run the tool as root, changed a rule,
installed a file, obtained a password or changed security settings.

The only caller-controlled executable is passed to `execve` in the forked child
after supplementary-group clearance, gid/uid drop and an irreversible-drop
check. Nonzero SUDO IDs/primary gid are validated; environment is cleared before
loading absolute system frameworks. Child has only TZ=UTC/LC_ALL=C, cwd=/,
stdin=/dev/null and captured bounded stdout/stderr. Actual open FDs are listed
and closed, including descriptors above a lowered soft limit. No shell calls,
output path, arbitrary environment or raw counter register input is accepted.

The root parent accepts only this CPU and21 named audited event encodings;
it acquires an idle ktrace session without resetting another owner. It snapshots
and restores the counter classes/configuration, checks programmed readback,
releases forced counters and removes only its own trace. Active or preconfigured
profilers are refused. An internal timeout and alarm bound the privileged child
wrapper. Hard-exit/SIGKILL restoration still depends on kernel owner teardown
and has not been tested with root privileges; no complete security/runtime claim.

Current iMac `/`, `/usr`, `/usr/local` are root:wheel0755; libexec was absent.
Runtime checks root ownership, nonwritable modes and write ACLs. Installer creates
root:wheel0755 libexec/helper; sudoers is root:wheel0440, fixed binary path,
SHA-256 match, `NOSETENV`, environment reset with only TERM preserved before the
helper clears it. It does not grant passwordless install/shell access.

## Real nonroot qualification

At20261021T145905Z, clang14.0.0/CLT on iMac20,1/macOS12.6 compiled7 C modules
with warnings-as-errors, stack protector and PIE. All module static analyzer
runs reported no warnings. `--help`, database listing, nonroot and invalid-env
rejection pass. Unit tests cover64-bit/interleaved/incomplete/decreasing counter
records, UID parsing, inherited high-FD closure after lowering RLIMIT_NOFILE and
mocked credential-success/every-failure cases. Mock tests never confer root proof.

Exact reviewed executable SHA-256:
`e3d2bc4c7af7dab8c1181b78ca291429c9ef275e44b20a828e1ca01f1a7ec065`.
Source/header/compiler commands and outputs are sealed in qualification.json.
Build outputs remain in queue job and are excluded from Git. No Rust crate,
dependency, public engine API or canonical recipe changed. C uses libSystem/
libproc and the two system private frameworks; there is no new engine FFI.

Earlier nonroot builds found missing Monterey `closefrom` and const-macro
warnings (v1); an overly strict database-name check rejected marketing-name
`Intel Comet Lake` (v2). These were corrected. Final syscall mocks and actual
FD-enumeration unit tests cover the replacements; no early binary is installed.

## Event semantics and six groups

Local KPEP maps named fixed cycles/instructions to programmable fallbacks,
even with fixed events available. The tool explicitly reserves Intel fixed0
instructions and fixed1 cycles, configuration0x22 for user-only counting,
then lets KPEP allocate the four programmable slots. All seven counter positions
are retained;0x22 readback must succeed before child release.

The CPU's [Intel perfmon map](https://github.com/intel/perfmon/blob/main/mapfile.csv)
routes A5 to SKL; the [V59 event definitions](https://github.com/intel/perfmon/blob/main/SKL/events/skylake_core.json)
are crosswalked with the actual Comet Lake plist. Metadata hashes and selected
encoding facts are recorded, not a guessed name equivalence. Most local events
lack modifier fields: CYCLE_ACTIVITY.STALLS_TOTAL needs CMask4;
MACHINE_CLEARS.COUNT needs CMask1+edge; ITLB_MISSES.WALK_ACTIVE needs CMask1.
These named modifiers are compiled into the approved table after verifying
the database EventCode/UMask. OS/AnyThread/PMI remain off.

21 supported events form6 allocation-checked groups, all with fixed cycles/
instructions. They cover DSB/MITE/MS uops, DSB-to-MITE penalties, branch misses/
BACLEARS, three machine-clear events, store-forward/address aliases, issued uops,
I-cache misses, total execution stalls, LSD and all six local ITLB_MISSES events.
16 FRONTEND_RETIRED names are present but require auxiliary MSR/PEBS selector
state unavailable through this tool. UOPS_RETIRED.RETIRE_SLOTS and
RESOURCE_STALLS.ANY are absent. All18 exclusions and exact group encodings are
recorded; no zero-fill, silent alias or multiplexing. Presence is not support.

## Prepared installation and lead job

The exact user commands and uninstall are in the linked README. Queue job
`20261021T150720Z-t056-kpc-install-ready` contains the installer/rule; nonroot
`visudo -cf` parsed the final rule successfully. The earlier150422Z rule used
an unsupported empty env_keep string; it failed parsing and was never installed.
The corrected rule retains only TERM, which the helper then clears.

The single lead-only job verifies hash/provenance/unit/old correctness+86-case
receipts, then under the measurement lock runs actual `/usr/bin/id -u` and
`/usr/bin/env` through sudo/helper before any engine measurement. It checks
nonroot identity, exact environment, config readback and cleanup. Missing install
or unavailable root PMU access stops; no agent installation or permission change.

Base/f58,8f,count,flags,count_flags,dfff plus base/dfff A/A clones, global/local,
six groups, two paired blocks, N23,068,672/2N:192 units,384 measurements and2
privilege checks. One active executable path, separate inodes and rotated/reversed
order. Sample windows require10+ samples per observed thread and observed span/
helper interval≥0.70. Bad output/config/permission/sample/coverage, deadline or
load disturbance stops without retries or deleting rows. Rare negative N/2N
event deltas remain uninterpretable, never clamped. CPI is sampled user-window
cycles/instructions, not untrusted rusage cycles or a complete-process CPI claim.

Prior job135119Z throughput174.017s/170 units projects196.537s;2x+120=513.074s
under720 collection/840 body/890 runner/900 watchdog, including quieting and
excluding queue wait. Host-local03–05 and15min-before03 excluded,20min admission.
A/A is descriptive, no performance acceptance, event-cause/JCC inference or
attempt consumption. Full-child totals remain a real limitation regardless of
positive PMU access after installation.

## Repository verification

Diagnostic validation passes386 simulated helper calls through the actual
collector, missing-install/permission/UID/output/counter/coverage/timeout/load/
deadline failures, exact21 encodings and real source/binary/nonroot receipts.
No simulated result is hardware evidence. `./scripts/check.sh` passed including
5170 curated Test262 cases on the working tree with engine stilldfff3bb6.
Task/diagnostic-only fast gate selects context checks; `./scripts/check-touched.sh --staged --explain` passed. Frozen plan/policy and the final remaining attempt stay
unchanged. Privileged correctness and tracing access await user installation.
