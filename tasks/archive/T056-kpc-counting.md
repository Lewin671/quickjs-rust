# T056 PMU counting redesign v3

- Status: prepared; user install/root smoke pending, no owner timing.
- Implementation: [helper/protocol](../diagnostics/t056-pmu/kpc-tool/README.md).
  Runtime/recipe/frozen plans unchanged; last acceptance attempt unspent.
- Verified at: engine dfff3bb6; tooling based on2ef94dbe. v3 C/H/qualifier hashes
  in qualification170020Z done0/10s; groups170125Z and installer170331Z done0.
- Evidence: [qualification](../diagnostics/t056-pmu/kpc-tool/qualification-v3.json),
  [groups](../diagnostics/t056-pmu/kpc-tool/groups-v3.json),
  [collector tests](../diagnostics/t056-pmu/kpc-tool/validation-v3.json),
  [lead v2 observations](../diagnostics/t056-pmu/kpc-tool/failure-v2.json).
- Unresolved: v3 root all-CPU getter, real cleanup and background/throughput
  qualification await exact digest installation. No valid PMU workload evidence.
- Next action: user installs reviewed v3; short non-timing root smoke verifies
  CPU matrices/readback/UID/cleanup. Lead alone may submit ≤15min diagnostic after
  success. Tailspin can be re-enabled now; do not keep stopping system daemons.

## Why replace PET

Lead reports real root v2 SHAa8021d9a… id-u capture completed with uid501 and
verified programmable config,0 samples because it was short. After release,
launchd-managed tailspind took ktrace. User disabled tailspin and stopped its
daemon; a Perl CPU loop then failed drain.record_integrity at1 sample,
child_window56,000ns, cleanup_oktrue. tailspind relaunched within seconds.
These are lead-reported non-timing observations, preserved separately from
[v1 setup history](T056-kpc-setup-failure.md); owner did not repeat daemon changes.

The Monterey [KPC sysctl](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/bsd/kern/kern_kpc.c)
uses ktrace_read_check, not ktrace_configure. The
[permission function](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/bsd/kern/kern_ktrace.c)
grants root independently of the owning trace PID. KPC counting does not need
tailspind to relinquish tracing. A separate forced/programmed PMU consumer may
still make it busy; v3 refuses it without altering KPERF or killing anything.
The [ABI reference](https://gist.github.com/ibireme/173517c208c7dc333ba962c1f0d67d12)
describes all-CPU kpc_get_cpu_counters and its ncpu×counter_count layout.
[Exact source references/hashes](../diagnostics/t056-pmu/kpc-tool/source-references-v3.json)
record what was inspected. Source establishes permission/ABI intent, not actual
root getter success on this host. Installed v2 exposes no CPU getter; its digest
rule cannot authorize a rebuilt tool. No privilege workaround was attempted.

## v2 record_integrity investigation

Old code assumes every DATA payload has requested class2 width4 and filters out
metadata carrying actual counterc. Monterey
[kperf_kpc_thread_sample](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/osfmk/kperf/kperf_kpc.c)
initializes counterc=KPC_MAX_COUNTERS32. If thread lookup fails, it zeroes that
many values; [kpc_get_curthread_counters](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/osfmk/kern/kpc_thread.c)
returns EINVAL without updating inoutcount when the thread buffer is missing.
Buffers are allocated lazily. PET consequently can log eight DATA fragments
(START, six NONE, END), even while requested width is4. The parser accepts the
first as one sample, then marks the first continuation invalid: exactly the
reported sample-count/error signature. A synthetic32-zero-fragment regression
reproduces this through the real legacy parser. Wider7-value fragments also do.

This confirms a structural parser assumption defect and a reachable XNU fallback,
not that this exact kernel path occurred in the lead run: raw rows/counterc
metadata were not retained. Duplicate timestamps, decreasing counters or other
malformed-fragment branches cannot be ruled out. No fabricated raw trace or
claimed exact root cause. v3 has no trace parser in its executable.

## v3 preparation and limits

Parent acquires force/config/counting, reads all CPUs around its gated nonroot
child, then reads around a child-free sleep matched to observed child wall.
Report raw snapshots/read timestamps, per-CPU and summed event deltas for both
windows. Cleanup clears own counting, restores prior config and any unforced
fixed-only baseline, releases force and verifies state; foreign programmable/
thread counting is rejected. Cleanup errors remain distinct and invalidate data.
Security/drop/env/absolute-path/root-install/no-file-write boundary is preserved.
No root executable, installed file, sudo rule or daemon setting was changed.

System-wide USER-only totals include other programs and helper polling. Idle
rates quantify background in an adjacent window; they do not establish stable
background or isolate the child. Raw and idle-adjusted N/2N deltas remain
separate.5% cycle/instruction contamination and10% A/A CPI are descriptive flags.
Negative rare counts/increments and idle fractions above1 are retained. No
JCC/DSB causal attribution or acceptance standing can follow from these alone.

Nonroot170020Z: clang14 strict build, PIE/stack protector, six-module static
analysis, credential/FD tests, mock setup/read fault sweep with release on every
failure, foreign refusal/baseline/readback/bounds/overflow/cleanup-error/repeat
coverage, real unprivileged process gate/env/timeout/overflow cleanup, real JSON
serialization. Binary SHA-256
`4b5776d1126490ddd9d3b6b8394cbe9b43c3406752534fb0704c501cfb81eafe`.
Initial165641Z failed shadow warnings;165834Z descriptor test caught an omitted
events_prepare call. Both fixed before final passing qualification; no root run.
165921Z also passed;170020Z seals additional fallback/readback/bounds tests.

170125Z re-audited21 supported events in11 USER-only2+2 groups.170331Z parsed
new sudoers/install assets without installing. Collector validation exercises354
fake privileged calls+8 nonroot warmups, missing install/permission/UID/output/
counts/idle span/CPU matrix/scope/deadline/load rejection and preserved negative
and noisy background results. All historical v1/v2 snapshots are unchanged.

Prospective lead-only job: six images+two A/A clones, global/local, N11,534,336/
2N, one block,176 units/352 matched child-idle pairs,2 uid/env checks+8 warmups.
Halving work offsets matched sleep: prior180.159s workload+354×0.211s setup,
2× reserve+120s preparation+5s warmups=634.706s<720 collection,840 body,890 runner,
900 watch including quieting. Host-local03–05/15min-before03 exclusion/20min
admission unchanged. Root throughput unverified; hard deadline retains partial
rows, never retries or consumes the last acceptance attempt. On this tooling
tree (engine dfff3bb6), full `./scripts/check.sh` passed including5170 Test262
cases; staged `check-touched.sh --staged --explain`/context passed. The fast gate
selected metadata/context only; no engine/parser/lexer semantic slice matched.
