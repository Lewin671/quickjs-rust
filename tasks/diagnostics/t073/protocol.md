# T073 proposal for T017: process tails and invalid increments

Part 1, "Preserve evidence without stopping unrelated cases", is adopted
prospectively in [T017](../../T017-performance-benchmark-system.md). Part 2 is not
adopted; neither part reinterprets T072. The owning task is [T073](../../T073-local-read-bimodal.md).
The standing contract remains [load-only recovery](../../../docs/layout-policy-artifacts.md#load-only-pair-recovery).

## Preserve evidence without stopping unrelated cases

Before adoption, the collector retained all five raw pairs, then replay rejected
any load-valid increment below 50 ms, including a negative one. That exception
terminated its case shard. T072 therefore loses complete portfolio evidence
because one N process is as expensive as its separately launched 2N process.
A stable instruction increment does not make that CPU-time increment valid.

Adopted collection changes, independently of the acceptance statistic:

1. Seal the raw packet, load eligibility, checksums and executable identities
   exactly as today. Never classify a process as load-invalid from its timing.
2. Turn a valid short/nonpositive increment into a structured **case health
   failure**, retaining the signed difference and its four process records.
   The scheduler should continue the remaining cases and layouts in its
   already requested inventory. A crash, bad checksum or timeout also stays
   a failure; continuing collection cannot erase it.
3. Write a complete inventory/report with per-case collection and analysis
   states. Overall readiness remains **incomplete** whenever any required
   case has a health failure. Do not feed nonpositive costs into log ratios,
   geomeans or the decision verifier. Distinguish inventory completion from
   acceptance-evidence completeness.
4. Give the failed case raw N/2N wall, user CPU and instructions, all attempt
   indexes, load flags and a process-distribution diagnostic. A single slow
   process stays in that evidence. Under existing policy it is never
   replaced, relabeled, pooled with another run or retried until favorable.
   T072's two charged attempts and closed standing remain unchanged.

This changes failure containment and reporting, not thresholds, attempt
accounting, recovery eligibility, or the meaning of a passing screen.

Implementation boundary: `pair_recording.py` owns retained case health;
`case_schedule.py` / `worker_pool.py` own continuing unrelated work;
`screen_replay.py`, `layout_replay.py` and the decision verifier must agree on
fail-closed replay. Do not catch arbitrary infrastructure exceptions and
silently call a packet usable.

Focused verification must inject one slow, checksum-correct N with unchanged
instructions: preserve every raw sample, record its short/negative increment,
finish all other cases, produce no valid aggregate or promotion standing,
and replay the same health state. Also cover a timeout/correctness failure,
load-only replacement and interrupted/incomplete inventory. A captured T073
packet should remain a failure under old and proposed analysis. A/A tests
must establish that healthy reports are unchanged before adoption.

## A new statistic needs separate qualification

Independent fresh processes admit different execution rates: a model of
`time = startup + iterations * constant_cost` is not automatically true for
each N/2N pair. Raising both iteration counts alone cannot fix a multiplicative
rate change. The failure above is honest evidence of a violated assumption.

If a successor protocol is desired, freeze its estimand first. One candidate
is the difference between independently estimated median process times at N
and 2N, with the same randomized block ordering and a predeclared repeat
budget. Retain every process, resample whole blocks, and report the process
tail distribution separately. A minority CPU tail may be hidden by any
median; an independently specified stability gate must therefore accompany
it. A favorable median cannot certify the pathology absent.

A same-process phased measurement is another research option, but changes
warmup, allocator/cache state and the benchmark interface. It is not a drop-in
replacement for the current black-box process contract. Execution-segment
data should first test whether a process keeps one rate for its lifetime.

Qualify any new statistic on frozen base A/A, the original candidate/base,
all selection lanes and all six layouts, with prospective precision and
false-positive criteria. Record all samples and failed trials. No change to
the retired T072 plan, accepted policy JSON, or attempt budget is implied.
Do not disable production ASLR or select an environment/code offset merely
to obtain a passing measurement.
