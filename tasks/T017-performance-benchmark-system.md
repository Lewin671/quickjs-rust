# T017: Performance Benchmark System

- Status: blocked. M6 and M7 need qualified fixed hardware, and T022 records
  (2026-09-22) that there is no separate measurement machine.
- Implementation: partial. M0-M5 and the hosted preview infrastructure are
  present (`benchmarks/`, `tools/benchmark/`, `scripts/benchmark*.sh`,
  `scripts/performance-*.sh`). M6 and M7 are not started.
- Verified at: not recorded per milestone. This file was last edited at
  `57c43051`, 2026-07-16. At `d1313d52`, `benchmarks/performance-policy.json`
  still has `fixed_hardware.configured` false, an empty `evidence_entries`
  list and every gate disabled.
- Evidence: `benchmarks/performance-policy.json`; `docs/benchmarking.md`.
- Unresolved: no fixed-hardware fingerprint, no A/A shadow reports, no noise
  envelope, no false-positive budget.
- Next action: none until fixed hardware exists. Then follow the activation
  order in `docs/benchmarking.md`, "CI Layering and Gate Activation".

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
`docs/benchmarking.md`, "CI Layering and Gate Activation", and enforced by
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
