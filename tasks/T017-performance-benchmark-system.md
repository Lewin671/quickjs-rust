# T017: Performance Benchmark System

- Status: blocked. M6 and M7 need qualified fixed hardware, and T022 records
  (2026-09-22) that there is no separate measurement machine.
- Implementation: partial. M0-M5 and the hosted preview infrastructure are
  present (`benchmarks/`, `tools/benchmark/`, `scripts/benchmark*.sh`,
  `scripts/performance-*.sh`). M6 and M7 are not started.
- Verified at: not recorded per milestone. The hosted preview's staged
  workflow was verified at `35121b6a`, 2026-10-09 (see "Hosted preview
  record"). At `d1313d52`, `benchmarks/performance-policy.json`
  still has `fixed_hardware.configured` false, an empty `evidence_entries`
  list and every gate disabled.
- Evidence: `benchmarks/performance-policy.json`; `docs/benchmarking.md`.
- Unresolved: no fixed-hardware fingerprint, no A/A shadow reports, no noise
  envelope, no false-positive budget.
- Next action: none until fixed hardware exists. Then follow the activation
  order in `docs/benchmarking.md`, "Gate activation".

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

## Hosted preview record

What was measured about the hosted preview itself, so it is not re-derived.

**2026-10-09, lanes in parallel jobs (`35121b6a`).** Run 37933477498 at
`3fbd4e53`, one job: 1385 s, of which broad 815 s, external 361 s, sentinel
121 s, candidate build 40 s. Run 37950725509 at `35121b6a`, staged: 891 s
from creation to conclusion; build job 37 s, then broad 828 s, external
396 s and sentinel 130 s side by side, publish 5 s. The run is now the build
plus the broad lane.

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

The remaining levers on the broad lane both change what it measures and
need their own qualification: fewer or shorter linearity probes (a
measurement-protocol change, also binding the formal lanes), or sharding the
25 cases across runners (the lane's overall ratio would no longer come from
one host).
