# T017: Performance Benchmark System

- Status: active. M6 can start: a dedicated x86_64 measurement host has been
  characterized (see "x86_64 host record"); it is not qualified as fixed
  hardware, and T022's 2026-09-22 note that no such machine exists is stale.
- Implementation: partial. M0-M5 and the hosted preview infrastructure are
  present (`benchmarks/`, `tools/benchmark/`, `scripts/benchmark*.sh`,
  `scripts/performance-*.sh`). Formal comparisons and screens run on a
  platform other than the frozen arm64 profile
  (`tools/benchmark/host_manifest.py`, `tools/benchmark/cycle_trust.py`).
  M6 and M7 are not started.
- Verified at: 2026-10-12 on the x86_64 host: at `07897ce4` a three-block
  `perf-compare.sh` bundle (state success, 76 cases, wall time) and a queue
  built from it; at `195b4341` a `perf-loop.sh` screen. The hosted preview's staged,
  sharded workflow was verified at `d9fb83f3`, 2026-10-09. At `d1313d52`,
  `benchmarks/performance-policy.json` still has `fixed_hardware.configured`
  false, an empty `evidence_entries` list and every gate disabled.
- Evidence: the records below; `benchmarks/performance-policy.json`;
  `docs/benchmarking.md`. Raw bundles are local artifacts, not retained.
- Unresolved: no fixed-hardware fingerprint, no A/A shadow reports, no noise
  envelope, no false-positive budget. Whether formal lanes may be measured
  concurrently on one host is undecided (the probe below is not a
  qualification).
- Next action: a 30-block A/A bundle on the x86_64 host, then follow the
  activation order in `docs/benchmarking.md`, "Gate activation".

Record of the landed milestones, the original assignment and the notes:
`tasks/archive/T017-performance-benchmark-system-log.md`.

## Goal

Build a long-lived, reproducible candidate/base/QuickJS-NG benchmark platform
whose evidence can eventually support conservative regression gates and public
claims without coupling the Rust engine to the reference implementation.

The platform itself is described in `docs/benchmarking.md`. This file tracks
only what is still owed.

## Acceptance

- [ ] M6 establishes fixed-hardware A/A shadow baselines.
- [ ] M7 enables fixed-hardware nightly/release gates and, if justified,
  self-hosted PR sentinels.

## Rules until M6 and M7 land

- Do not add a CI performance gate before M6 demonstrates the noise envelope.
- Hosted previews stay informational and non-gating, with
  `claim_eligible=false`.
- The external-corpus registry stays deny-only. Admitting a corpus needs a
  separately reviewed v2 content-hashed audit bundle; the registry is
  governance metadata, never headline evidence.

The activation prerequisites (fingerprint, the number of A/A reports per
gate, noise envelope, false-positive budget) are stated once, in
`docs/benchmarking.md`, "Gate activation", and enforced by
`tools/benchmark/performance_policy.py` against
`benchmarks/performance-policy.json`.

## Scope

- Allowed paths: `benchmarks/`, `tools/benchmark/`, benchmark scripts and docs.
- Forbidden paths: engine semantics and `third_party/` outside a separately
  approved milestone.
- Owner boundary: serialize manifest/schema changes; external corpora are one
  independently reviewed admission unit each.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/benchmark/tests -v
./scripts/performance-policy-audit.sh
./scripts/external-corpus-audit.sh
./scripts/check.sh
```

Running the benchmark lanes and reports: `docs/benchmarking.md`, "Running".

## x86_64 host record

Intel Core i7-10700 (8 cores, 16 threads), 32 GB, macOS 12.6, otherwise idle;
2026-10-12. Candidate `8d24e554` (runtime; harness `07897ce4`), base
`e54e33eb`, QuickJS-NG `f7830186`.

**Cycles are not a clock there.** `proc_pid_rusage` returned about 0.02
cycles per CPU nanosecond, spread 14% (p10 to p90) over 25 identical runs
whose instructions agreed to 0.004% and whose wall time agreed to 0.7%
(QuickJS-NG) and 5.9% (qjs-rust, same loop). `/usr/bin/time -l` prints no
cycle line for a short process. Hence `cycle_trust.py`: that host records
no counters and is judged on wall time.

**A three-block formal run takes 24 minutes** (1453 s; an earlier one at
`8d24e554` took 1577 s): broad 941 s, of which linearity 537 s,
calibration 222 s and the three blocks 134 s; sentinel 129 s; external
about 430 s. Extrapolated to 30 blocks: about 97 minutes, of which the
external lane about 57. Candidate/base half-widths at three blocks: broad
median 1.0% (largest 13.6%), sentinel median 1.1% (largest 1.9%), external
median 2.0% with 33 of 45 cases within 3%; `kraken-1.1/audio-dft` 1.3%,
against 9.9% at 30 blocks on the arm64 host (T038). Ten broad blocks,
candidate against QuickJS-NG: paired log-ratio standard deviation 0.77%
median, 4.1% largest.

**Concurrent measurement probe.** 16 broad cases, base and candidate
alternating, 10 rounds per worker, 1, 4 and 8 workers at once. Per-process
time rose 9% at 4 workers and 32% at 8. Median candidate/base ratios moved
by at most 0.42% (4 workers) and 0.47% (8) from the serial ones, which is
inside the serial estimate's own error. Pair standard deviation: 0.73%,
1.05%, 1.64%. Four workers therefore give four times the samples per
minute at 1.4 times the pair spread. Not covered: the external lane's
memory-heavy cases, and more than one run. Nothing measures concurrently
today.

**Concurrent-lane qualification, criteria written before measuring.** One
30-block sequential bundle and one 30-block `--concurrent-lanes` bundle of
the same three executables on the idle host. Concurrent lanes qualify for
that host when all hold: every lane completes with linearity passing; at
least 73 of the 76 candidate/base case ratios are within 3% of the
sequential ones; each lane's geometric-mean shift of those ratios is within
0.5%, and of the candidate/QuickJS-NG ratios within 1%; the concurrent
bundle has at most two more imprecise cases than the sequential one.

## Hosted preview record

What was measured about the hosted preview itself, so it is not re-derived.

**2026-10-09, lanes in parallel jobs (`35121b6a`).** Run 37933477498 at
`3fbd4e53`, one job: 1385 s, of which broad 815 s, external 361 s, sentinel
121 s, candidate build 40 s. Run 37950725509 at `35121b6a`, staged: 891 s
from creation to conclusion; build job 37 s, then broad 828 s, external
396 s and sentinel 130 s side by side, publish 5 s. The run is now the build
plus the broad lane.

**2026-10-09, broad lane in two shards (`d9fb83f3`).** Run 37963316916:
494 s from creation to conclusion; build job 48 s, then `broad-1` 421 s,
`broad-2` 409 s, external 286 s and sentinel 134 s side by side, publish 7 s.
Checked against criteria written before landing, using run 37950725509
(unsharded) as the reference: all 25 cases present with linearity passing;
per-case candidate/base ratios within 5% for 22 of 25 (required 20, and
equal to the unsharded run-to-run figure) with a mean shift of 0.9970
(required 0.99 to 1.01); total under 600 s. The two executables were
byte-identical in this run, and the report said so.

**Where the broad lane's time goes (run 37933477498, sample durations).** Of
814 s: linearity diagnostics 475 s (600 samples), calibration 183 s, warmup
40 s, the three measurement blocks 116 s. The sentinel lane has the same
shape (69 of 119 s in linearity). Every phase scales with the calibrated
window, and the eight linearity probes are part of measurement protocol v9.

**What identical builds show on hosted runners.** Runs 37933477498 and
37950725509 measured equivalent engine code unsharded (the second one's two
executables were byte-identical). Within a run, every group's
candidate/base ratio was within 0.5% of 1 and the furthest single test was
6.4% (`empty_loop`). Between the two runs, per-test candidate/base ratios
differed by up to 6.9% and 22 of 25 broad cases agreed within 5%; per-test
candidate/QuickJS-NG ratios differed far more (13 of 25 within 5%, mean
shift 3.7%), which is the runner hardware changing between runs. The
summary's wording thresholds (`GROUP_NOISE` 2%, `TEST_NOISE` 7% in
`tools/benchmark/preview_summary.py`) come from these numbers.

**2026-10-09, rejected: a shorter hosted measurement window.** Capping
`min_window_ms` at 250 and raising `startup_max_fraction` to 0.04 in the
derived hosted manifest (14 broad cases ask for 500 ms and 1% startup, which
at about 4 ms of hosted startup forces a window above 400 ms). Qualified
before landing with the same three executables on one macOS arm64 host, broad
lane, three blocks, against criteria written down first: at least 23 of 25
cases within 5% of the template-window candidate/QuickJS-NG ratio and the
geometric mean of the shift within 2%. Template windows: 851 s,
candidate/base 1.0008 [0.9991, 1.0028], candidate/QuickJS-NG 0.9744. Short
windows: 490 s, candidate/base 1.0033 [0.986, 1.069], candidate/QuickJS-NG
1.0114; only 19 of 25 cases within 5%, mean shift +3.8%, and the largest
per-case candidate/base deviation went from 0.9% to 7.0%. It changes the
reading, not only its noise, so it was not landed. The code was never
committed.

A run is now the build plus the slowest of four similar jobs, so a third
broad shard would not shorten it. The remaining lever on per-case cost is
fewer or shorter linearity probes, which is a measurement-protocol change
that also binds the formal lanes and needs its own qualification.
