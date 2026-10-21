# Installed PMU v3: collector recovery protocol

[Current record](../../../archive/T056-kpc-collection-retry.md),
[v3 preparation](../../../archive/T056-kpc-counting.md),
[v2/PET history](../../../archive/T056-kpc-setup-failure.md).

The final helper is **installed**, SHA-256
`4b5776d1126490ddd9d3b6b8394cbe9b43c3406752534fb0704c501cfb81eafe`.
The user re-enabled tailspin. Root smoke passed. This change touches only the
collector/protocol, offline tests and documentation: no new executable,
installation, sudo policy change, daemon action or owner timing job.
Engine, recipe, frozen plans and the last acceptance attempt remain unchanged.
The C/H/qualifier/installer/rule bytes remain exactly those qualified at170020Z.

## Initial real run

Lead `20261021T172024Z-t056-kpc-counters` stopped after6s: uid/env passed,
8 warmups,2 valid global_read captures, then aligned/global_read/N failed
`read.delta_integrity` despite child_status0 and cleanup_oktrue.
[Verbatim raw rows](run-v3-initial.jsonl), [diagnosis](run-v3-diagnosis.json).

Work-window instructions decreased on CPU0 and2; cycles on those CPUs jumped
by approximately2^48. The idle window was monotonic. Setup readback matched all
four requested registers. The helper reports **only setup configuration**:
there is no window/end readback. Width-boundary/shadow behavior and foreign
reprogramming cannot be distinguished; tailspin is not established as the cause.
No modulo correction, wrap repair, clamping or failed-row count recovery.

Even the two valid captures are not a favorable result: N/2N cycles increment
is−1,141,177, and normalized idle cycles are6.46%/7.51%. Preserve them; do not
retry for negative increments, high background, slow wall time or unfavorable CPI.

## Frozen prospective retry policy: v3-retry1

- Unit: the same role, case, group, block and complete N/2N pair. Preserve its
  original N/2N order, active path, source inode and argv. Restart the pair if
  either read has a counter discontinuity; retain any abandoned valid mate.
- At most3 total attempts per unit. Accept the first fully valid pair, without
  looking at its time, CPI, background fraction or N/2N sign.
- At most17 invalid observations over352 planned valid captures: failure burden
  must not exceed5%. The fixed denominator prevents retries diluting failure
  rate. Also report failures/all actual helper calls. Do not stop solely because
  the first few calls have a high provisional fraction; reject at the18th
  failure. A unit still missing after3 attempts invalidates the whole job.
- Discarded pair attempts may consume at most68s, including any valid mate and
  helper setup/idle time. Existing720s collection/840s body/890s runner/900s watch
  remain authoritative. Retry budget/deadline exhaustion retains partial data.
- Retry only well-formed, correctly configured/cleaned captures with successful
  child/output and demonstrable per-CPU counter decrease, impossible positive
  jump or uint64 sum overflow. The known helper exit1 must be exactly
  read.delta_integrity/Monterey EPROTO100 with false capture/counters flags.
  An unexplained integrity error is fatal. Helper-success positive jumps are
  invalid too: conservative per-CPU ceilings are16 cycles/ns,256 other events/ns,
  each plus1,000,000 count slack. These coarse diagnostic ceilings are not
  performance acceptance thresholds and do not reconstruct corrected counts.
- No retry for timeout, truncated/malformed data, bad UID/output, cleanup,
  configuration/shape/clock/idle-span errors, unknown helper failures, load,
  deadline or any unfavorable valid metric. Detect any reported setup readback
  mismatch immediately. Mid-window configuration changes are unobservable with
  this final binary; do not label setup-only verification as end verification.

Every call is written/flushed to raw.jsonl before validation, with unit_attempt.
observations.jsonl retains invalid reasons, per-CPU values/deltas, abandoned
mates and accepted-pair receipts. quality.json checkpoints failure burden,
actual observed failure fraction, discarded seconds and retry counts before any
retry-limit abort. Summary/complete include the final quality record. No whole
job retry, partial-run selection, acceptance claim or attempt consumption.

## Counter scope and unchanged root boundary

System-wide USER-only KPEP class2: cycles/instructions plus two extras in four
programmable slots;21 approved events/11 groups,18 exclusions. No PET/KDEBUG,
thread counting or fixed-control ABI. Four raw per-CPU snapshots and read times
cover child and adjacent matched child-free sleep. Background and helper polling
are included; idle adjustment is descriptive, not PID attribution. Snapshots are
sequential across CPUs, not atomic. Negative rare increments remain uninterpretable.
5% idle and10% A/A CPI are descriptive flags, never conditions for retry.

The installed helper still validates root install ownership/ACLs and nonzero
SUDO_UID/GID, clears environment, drops all child credentials before the gate/
absolute exec, closes inherited FDs, fixes TZ/locale/cwd/stdin and bounds output.
It acquires force/config/counting, restores original config and unforced
fixed-only baseline, releases force and verifies cleanup. Foreign programmed/
thread profilers are refused. Internal timeout/alarm behavior is unchanged.
No helper/root files are written; artifacts belong to the nonroot collector.

## Lead-only ≤15min run

Six canonical images+base/dfff A/A clones, global/local, one block,
N11,534,336/2N:176 units/352 accepted captures,2 uid/env checks+8 nonroot warmups.
At most34 extra measured calls under the failure bound; same existing helper.
Prior conservative634.706s projection+68s retry allowance=702.706s<720 collection.
Root future throughput remains unknown; hard deadlines override the estimate.
Host-local03–05/15min-before03 exclusion and20min admission remain.

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

Owner does not submit this timing job or run sudo. Existing helper/image/recipe/
correctness/86-case/host/file seals remain. Rendering uses
[config-v3-retry1.json](config-v3-retry1.json) and adds recovery.py; old configs,
groups, qualifications and validation receipts remain historical/frozen.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pmu/kpc-tool/validate.py
```

[Validation](validation-v3-retry1.json) replays the real raw capture, preserves
negative/noisy valid outcomes, exercises complete-pair replacement/same argv,
one-off and second-read failures,17-success/18-failure bounds,3-attempt exhaustion,
positive jumps, retry wall cap and every existing fatal gate. No real helper,
benchmark, root command or installation is executed by these tests.
