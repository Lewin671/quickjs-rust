# T038: Audio DFT cycle variance

- Status: active. Top performance infrastructure item; diagnostic only.
- Implementation: none. Batch 1 remains present on `integration/batch-1`,
  with both promotions retained under the new rule; main stays e54e33eb.
- Verified at: `8e208b8a`, 2026-10-12, isolated-case process measurements of
  the exact e54e33eb queue executable and the integration runtime executable.
- Evidence: distributions and exclusions below. Raw diagnostic artifacts in
  `target/audio-dft-diagnostic/` are local, not retained.
- Unresolved: the hardware cause and residual within-band cycle variance; no engine or
  benchmark-protocol fix is approved or implemented.
- Next action: after retained results are pushed, freeze/validate the approved
  numeric-scratch-placement plan, then sweep a full page before choosing a fix.


## Scope and identities

The runtime is unchanged from measured af4d0d3c at integration 8e208b8a.
The diagnostic worktree is detached at 8e208b8a; its uncommitted address
instrumentation is throwaway. Only documentation is committed to integration.
The main worktree and its dirty typed_loop files are untouched.

Apple M1 Pro, macOS 26.6.2 (25G83). All engine measurements and diagnostic
builds hold `lockf /tmp/qjs-lead/host-timing.lock`; each process waits for
cargo/rustc/linker/engine competitors first. Other sessions' lock waits are
not part of the samples. Whole-process retired instructions/cycles come from
RUSAGE_INFO_V6 before reaping; wall time includes spawn/wait. Live observations
poll cumulative counters every 5 ms. A second series disables this polling.
The early raw keys named `user_ns` and `p_user_ns` contain Mach ticks, not ns;
conversion is 125/3 ns/tick, verified with mach_timebase_info. Their ratios
are unaffected. `wall_ns` is nanoseconds throughout.

SHA-256 of executable copies (all named qjs with equal-length role paths):

- Base: 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4.
- Integration: 6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b.
- NG: cfd8386c3c29b1125a878b8fb82f9627820f2dcc16d2a691c5f8c16ad0b047a0.

These are the formal bundle's binaries, copied without rebuilding. The
workload is the frozen Kraken data+script+completion marker; Rust uses
`--raw`, NG `--script`, with role-specific expected stdout checked. An initial
setup failure (NG mistakenly invoked as a module) is separately preserved in
setup-failed.jsonl, excluded from every experiment; completed experiment
samples are never discarded. Knob variants and base/tip order alternate
within blocks. The baseline rotates base/tip/NG; a second series reverses it.

## Whole-case distributions

Cycles and instructions are billions; wall is seconds. Cycle columns are
minimum / p10 / median / p90 / maximum. Instruction and wall columns are
minimum / median / maximum. These are diagnostic distributions, not CIs or
promotion evidence.

| Series / executable | n | Cycles G | Instructions G | Wall s |
| --- | ---: | --- | --- | --- |
| Baseline / base | 40 | 2.528 / 2.597 / 2.722 / 3.117 / 3.587 | 7.1265 / 7.1292 / 7.1500 | .809 / .877 / 1.189 |
| Baseline / integration | 40 | 2.510 / 2.578 / 2.706 / 2.969 / 3.148 | 7.1236 / 7.1268 / 7.1486 | .796 / .870 / 1.047 |
| Baseline / NG | 40 | 3.271 / 3.302 / 3.369 / 3.635 / 3.676 | 11.0923 / 11.0950 / 11.1093 | 1.033 / 1.086 / 1.235 |
| No live polling / base | 40 | 2.566 / 2.622 / 2.776 / 3.306 / 3.499 | 7.1273 / 7.1295 / 7.1430 | .806 / .881 / 1.130 |
| No live polling / integration | 40 | 2.552 / 2.569 / 2.670 / 3.021 / 3.641 | 7.1231 / 7.1258 / 7.1426 | .799 / .842 / 1.183 |

Baseline cycle bins <2.4 / [2.4,2.7) / [2.7,3.0) / >=3.0 G contain
0/18/15/7 base and 0/20/16/4 integration samples. There is a broad upper tail,
not a clean two-population gap in this baseline. NG also varies (12.0% full
range/median), with overlapping body/tail, not the Rust-sized split. Additional
knobs sometimes expose a much cheaper Rust population at unchanged work;
calling all these distributions one binary fast/slow switch is premature.

Baseline instructions vary only .33% base / .35% integration across the full
range, against 38.9% / 23.6% cycle range/median. Sequence/cycle correlations
are -.054 / -.031 / -.120 for base/integration/NG: no simple monotonic drift.
Cold first runs have major faults, but base 3.587G and integration 3.148G
samples have zero major faults. Fast base 2.528G and slow integration 3.148G
are both 100% P-core user time; the 3.587G base is 99.858%. E-core residency
cannot explain the large difference. Migration between P cores is unobserved.

## One-factor experiments

[The frozen diagnostic data](archive/T038-audio-dft-diagnostic-data.md) records
20 samples per base/tip variant: env 0/4096/16384 bytes, short/long cwd,
executable path and argv0 separately. None eliminates the spread. Env +4096
and +16384 expose a genuinely separated base population (4/20 and 6/20
at 1.87–2.00G, remaining samples >=2.57G). Baseline itself is a broad tail,
not a clean bimodal gap. Selecting a favorable environment is not a remedy.
The reverse NG/tip/base order also preserves the broad distribution (40 each).

## Execution paths and allocation diagnostics

Audio DFT uses ordinary Array elements (`Value` stride 16), not Float64Array.
The sin/cos tables each have 524288 elements; the 1024-element mixed signal is
read by the inner reduction. Inputs/table coefficients are deterministic.
The perf-counters trace has 10281 loop-plan entries and 2 declined edges;
closed-form leaf evaluations are zero. Numerical reduction admission follows
operations/data sources, not a benchmark-name test.

The first address probe logged only the typed channel/input path, missing the
local-only dense reduction. Its enabled/disabled medians are 2.684/1.998G
at 7.1315/7.1267G instructions (40 each), so it cannot price production code.
The second probe logs at the legacy read-only dense entry after construction,
with safe as_ptr values for backing stores, f64 register bank and locals,
plus stack/code markers and finite/subnormal/NaN scans. No pointers are
unsafely dereferenced, and no production allocation policy is changed.


A private per-process XNU ASLR-off flag was empirically verified (details
in the diagnostic record); code/stack slides stop, malloc bases remain random.
Stock on/off base medians are 2.735/2.658G, integration 2.805/2.697G,
20 each; off still spans 2.607–2.956G base and 2.545–3.111G integration.
No system/SIP setting changed. ASLR-off alone is not a remedy.

The second hot probe reproduced the cheap population with/without ASLR:

| ASLR / register bank mod 4096 | n | Cycle min / median / max G |
| --- | ---: | --- |
| on / 0xda0 | 36 | 2.543 / 2.711 / 3.732 |
| on / 0xdb0 | 4 | 1.940 / 2.000 / 2.307 |
| off / 0xda0 | 28 | 2.549 / 2.676 / 3.483 |
| off / 0xdb0 | 12 | 1.891 / 1.947 / 2.247 |

Every process logs exactly three hot receivers, one stable register-bank
position, all arrays mod 4096 zero; sin/cos distance is exactly 0x800000.
Locals/stack markers shift by the same 16 bytes as the bank. No overlap exists
between the two observed cost bands, although absolute malloc addresses
vary. This is a placement association, not proof of a particular 4K-alias or
cache-line hazard. All 80 scans report zero subnormal/NaN/infinite inputs;
minimum nonzero coefficient is 6.1898e-19, mixed signal 6.1628e-12.
Intermediate floating-point exception status was not sampled.

The slow baseline base's execution-decile CPI is .229/.997/.821/.691/.568/
.530/.392/.413/.412 (one startup decile lacked enough live observations):
the slowdown can recede inside a process. At 100 transforms on the same
objects (20 processes each), cycles min/median/max are base
11.920/12.258/13.121G, integration 11.521/12.041/13.459G, NG
15.299/15.608/16.335G. Instruction medians are 29.142/29.134/50.145G;
wall min/median/max are 3.740/3.856/4.147, 3.620/3.791/4.287,
4.814/4.922/5.244s. Merely increasing iterations does not eliminate drift.
P-core migration/cache residency, physical page colouring and the hardware
stall mechanism remain unobserved. pmset records no thermal/performance
warning; that is not continuous thermal telemetry or proof of no throttling.

The entry-stack probe confirms bank = main + 0xd70 modulo 4096, with
16-byte main-frame changes at fixed user env and ASLR-off. A controlled
follow-up keeps the same hot-probe binary/argv/ASLR and varies only one env
pad (4016/4032/4048 bytes, 20 each). Pads were selected from the pilot address
and the prior hypothesis, never its cycles; all samples including the pilot
are preserved. Actual bank offsets and cycle min/median/max:

| Bank mod 4096 | n | Cycles G |
| --- | ---: | --- |
| 0xda0 | 17 | 2.614 / 2.920 / 3.328 |
| 0xdb0 | 17 | 1.907 / 2.020 / 2.515 |
| 0xdc0 | 21 | 2.642 / 2.949 / 3.879 |
| 0xdd0 | 5 | 2.594 / 2.823 / 3.056 |

Instruction medians are 7.2264/7.2249/7.2262/7.2257G. The intervention
supports a real stack-relative scratch-placement effect, with residual
within-band variance. [Startup details and XNU hypothesis](archive/T038-audio-dft-diagnostic-data.md#startup-stack-checks)
record what could/could not be established about the startup trigger.

Recommendation (engine-unit proposal, not implementation): freeze a
`dense-number-scratch-placement` unit on the integration tip. Make f64 scratch
placement deliberate and independent of incidental entry-stack placement;
scan offsets/alignment policies across dense numeric workloads and generic
controls before choosing a policy. Never pin one audio source/array shape or
assume that 0xdb0 is universally fast. The hardware stall and within-band
upper tail are unresolved, so acceptance must include stable distributions
and precise >=30-pair case evidence, not just a better median. No backing-store
or scratch-bank placement policy was changed in these diagnostics.

Diagnostic paired intervals are recorded in the frozen data; every sample
is retained. No selective extension, outlier removal or pooling was performed.

## Re-decision follow-up

The lead changed the rule on 2026-10-12; the earlier split/re-measure estimate
is superseded. [T022](T022-performance-priority-controller.md#unstable-frozen-base-cases-lead-amendment-2026-10-12)
owns that policy; [T037](T037-integration-batch-1.md#re-decision-of-the-af4d0d3c-bundle)
owns the qualifying 30-block base A/A and both retained promotion decisions.
No new formal bundle was measured. The engine-placement recommendation now
has lead approval as numeric-scratch-placement, two attempts, af4d0d3c queue.
After the retained-result update is pushed, freeze/validate its plan before
implementation; first use a full-page diagnostic offset sweep to distinguish
relative backing-store hazards from bank alignment/page splits. Do not ship
an observed page offset or re-pin without screen evidence.
