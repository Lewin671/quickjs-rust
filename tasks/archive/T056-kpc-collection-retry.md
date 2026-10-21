# T056 installed v3 counter discontinuity and collector recovery

- Status: collector-only recovery prepared; lead-only measurement pending.
- Implementation: [protocol](../diagnostics/t056-pmu/kpc-tool/README.md);
  installed v3 binary/C/H/qualifier/installer/rule unchanged. No more install,
  sudo policy or daemon action; no owner root/timing run or acceptance attempt.
- Verified at: parent fc49e769, installed helper SHA4b5776d1… . Lead job172024Z
  got2 valid measured captures and1 invalid; offline raw replay and fake collector
  validate prospective v3-retry1. Engine remains dfff3bb6.
- Evidence: [raw](../diagnostics/t056-pmu/kpc-tool/run-v3-initial.jsonl),
  [diagnosis](../diagnostics/t056-pmu/kpc-tool/run-v3-diagnosis.json),
  [validation](../diagnostics/t056-pmu/kpc-tool/validation-v3-retry1.json),
  [sealed config](../diagnostics/t056-pmu/kpc-tool/config-v3-retry1.json).
- Unresolved: exact kernel/width/client cause and mid-window configuration
  cannot be determined from this binary. Retry success/failure burden on the
  actual host and diagnostic usefulness remain unverified.
- Next action: lead may run the prepared ≤15min protocol using the existing
  installed digest. Stop PMU if bounds cannot deliver complete valid pairs or
  if causal attribution requires configuration evidence the binary cannot expose.

## Real initial run

The lead reports final v3 installed, root Perl smoke counters_validtrue with
about0.1% idle, and tailspin re-enabled by the user. This does not establish
background for another workload/window. No further user reinstall/sudo action
is permitted. Owner performed offline analysis only.

Lead `20261021T172024Z-t056-kpc-counters` at fc49e769 stopped after6s, after2
uid/env checks,8 warmups and3 measured captures. First2 base/global_read N/2N
returned valid with child windows304.512/209.589ms. Third aligned/global_read/N
11,534,336 had child_status0, cleanup_oktrue and read.delta_integrity/EPROTO100;
child window305.093ms, idle314.186ms, all four raw snapshots present.

Work-window discontinuities (counter indices0 cycles,1 instructions):

| CPU/index | Before | After | Signed delta |
| --- | ---: | ---: | ---: |
| 0/0 | 1,187,246,011 | 281,475,019,421,786 | 281,473,832,175,775 |
| 0/1 | 281,474,690,922,632 | 42,711,131 | −281,474,648,211,501 |
| 2/0 | 1,107,391,450 | 281,476,503,986,695 | 281,475,396,595,245 |
| 2/1 | 281,474,528,774,589 | 1,527,276,040 | −281,473,001,498,549 |

The instruction decreases directly fail count_delta's monotonicity invariant.
No CPU decreased during the idle window; chronology/CPU matrices were sound.
The two huge positive cycle jumps would also be implausible in305ms; the new
collector rejects such jumps even when the binary's monotonicity check passes.

Jumps are near2^48, but this alone does not prove ordinary hardware wrap.
Modulo2^48 produces a still-enormous CPU0 cycle increment, and another cycle
word exceeds2^48. The [Intel SDM](https://cdrdv2-public.intel.com/868137/325462-089-sdm-vol-1-2abcd-3abcd-4.pdf)
places hardware counter width in CPUID.0AH; this capture reports no CPUID width
or counter normalization convention. No modulo repair is justified. XNU's
[generic KPC read](https://github.com/apple-oss-distributions/xnu/blob/xnu-8020.140.41/osfmk/kern/kpc_common.c)
dispatches all-CPU reads; the observed representation discontinuity does not
identify a client or a precise architecture-specific implementation defect.

Requested/pre-counting readback is[41003c,4100c0,410879,410479] in all measured
rows, verified=true. Installed count_begin reads configuration before enabling
counting; count_read checks counting mask/shape but no event configuration.
There is **no after/window configuration readback**. Cannot prove or exclude
another KPC client reprogramming the same class, attribute it to tailspin, or
establish unchanged configuration through a valid-looking window. A collector
can detect reported mismatches and observed counter discontinuities, not supply
missing kernel evidence. The binary stays final; requiring that missing audit
would end this PMU path rather than trigger another reinstall.

The2 valid captures have cycles1,058,051,245/1,056,910,068: N/2N increment
−1,141,177, so CPI increment is uninterpretable. Their duration-normalized idle
cycle fractions are6.459%/7.515%, MITE20.67%/57.99%. Keep them; the Perl smoke's
0.1% background cannot be transferred. These valid unfavorable outcomes never
trigger retry or get selected away.

## Prospective protocol

v3-retry1 restarts exactly the same whole N/2N pair on a demonstrable counter
integrity failure, at most3 total attempts. Same role/group/case/block, image,
active path, inode, argv and N/2N order. Every failed call and any valid mate
remain in raw logs. Accept only the first fully valid pair, independent of
wall time, CPI, background, rare-event sign or N/2N positivity.

Prespecified failure burden is all invalid observations/352 planned captures,
not successes plus an ever-growing retry denominator.5% permits17 invalid,
18th invalid aborts; report the conventional invalid/all-calls fraction too.
No early1/3 provisional-rate rejection. A missing pair after3 attempts aborts
regardless of global burden. Discarded-pair wall budget68s and original720s
collection/840s body/890s runner/900s watch still apply. Prior634.706s conservative
budget+68=702.706s<720. At most34 extra calls (a failed attempt consumes≤2 calls)
before the global bound; no owner timing or actual retry-throughput claim.

Eligibility requires successful child/output, good cleanup, exact setup config,
valid scope/CPU shape/chronology and per-CPU decreases, uint64 overflow or
coarsely impossible rates. Per-CPU cycle/event ceilings16/256 counts/ns+1e6
slack are deliberately loose diagnostic bounds, not performance gates. Known
binary failure must be exit1/read.delta_integrity/EPROTO100, false capture/counts.
Unexplained errors, malformed data, permissions, cleanup/config/shape/clock/idle
span, output, timeout, load and deadline failures remain fatal, not retried.
No metric is corrected or clamped. Valid noisy/negative observations remain.

raw.jsonl records every process result before validation with unit_attempt;
observations.jsonl records issues, discarded mate sizes and accepted-pair
receipts. quality.json checkpoints burden/calls/retry seconds before aborts;
summary/complete include final quality. No new privileged command surface.

## Verification

Six offline unit tests replay actual rows, reproduce all4 CPU/index anomalies,
accept the first2 rows, retain negative/noisy N/2N, replace both pair sizes in
order, reject huge positive jumps without correction, exercise68s exhaustion,
and prohibit retries for config/cleanup/timeout/unexplained failures. Fake full
collector validates normal362 calls, transient363, second-capture364,17-invalid
379,18-invalid abort,3-attempt exhaustion and all prior fatal gates. Tests invoke
no helper, engine, sudo, install or daemon command. Binary source/digest and
all historical qualification/groups/config/validation snapshots stay unchanged.
Full `./scripts/check.sh` passed on this collector tree, including5170 Test262
subset cases. Staged `check-touched.sh --staged --explain`/context passed; only
metadata/context selected because no runtime/parser/lexer semantic source changed.
