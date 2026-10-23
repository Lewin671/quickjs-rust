# T073: Local-read process bimodality

- Status: active; diagnostic slice complete; remedy proposals await lead review. No T072 attempt reopened.
- Implementation: diagnostic collectors and records present; no engine or accepted measurement-policy change.
- Verified at: exact T072 base `6e899d24` / candidate `6c122407`, 2026-10-23 UTC (24 CST), serialized iMac x86_64; local full gate on unchanged runtime `f75bec10`.
- Evidence: [job ledger and measurements](diagnostics/t073/measurements.md), [T017 proposal](diagnostics/t073/protocol.md), [original T072 failure](diagnostics/t072/attempt2.md).
- Unresolved: precise hardware stall and separate code/stack-ASLR contributions; no production runtime remedy or acceptance standing.
- Next action: lead reviews the diagnosis, qualifies T017 case-health reporting, then freezes a separate engine repair if selected.

## Scope

Base `f75bec10ad55a7c06a80bd672e1f6067b3470bcb`; branch
`agent/t073-local-read-bimodal/codex`, this worktree only. Private queue refs
transport experiments; no origin push, merge, dependency or third-party edit.
All iMac builds and process probes use the serialized queue; no other owner's job is cancelled.
All timing is outside local 02:45–05:00. No held-out input selects work.
The audited original T072 images are hash-verified before every collection.
The owning measurement record retains every run, including failed pilots.

## Diagnosis and classification

The frozen candidate has a **real address-dependent CPU-time pathology**:
roughly the same retired work can take about twice the user CPU time. It
reproduces in serial; the canonical base is tightly grouped. A uniform text relocation of the
same candidate removes the observed tail in a predeclared larger run, and
ASLR-off controls are also stable. This identifies executable/process address
geometry as a causal variable, with finite-sample limits. Classify that
observed extreme tail as **(a), a candidate-image pathology a successor T072
must avoid**. The property-proof algorithm is not shown to do extra work:
the sampled shared typed executor has identical normalized code in both
images, and does not execute repeated named-property operations here.

General engine placement sensitivity **(b)** is also demonstrated by ordinary
base latency changing under placement-only variants. That does not establish
an allocator-specific cause or reproduce the same extreme tail in the base.
The exact cache/TLB/predictor/store-load mechanism remains unobserved; equal
instructions plus extra CPU time alone cannot name it. The code-placement
interventions and detailed limits are in the measurement record.

The independent **(c)** problem is failure containment: one slow N process
makes its independently launched N→2N increment short or negative, and the
collector aborts the shard, leaving the portfolio incomplete. That health
failure is valid evidence; replacing the slow process would hide a real cost.
Live trajectories also show within-process rate changes under observation,
so a constant startup-selected fast/slow binary mode is too strong a model.

## Proposed fixes and boundaries

Adopt the [T017 case-health proposal](diagnostics/t073/protocol.md) prospectively:
retain the signed invalid increment and every raw process, finish the already
requested unrelated inventory, and keep overall readiness incomplete. Its
owning document specifies packet/replay boundaries and qualification tests.
Changing the timing statistic requires separate A/A/noise qualification and
a stability criterion that cannot hide a minority CPU tail behind a median.

The relocation and ASLR-off controls are causal diagnostics. No measured
winning offset, disabled ASLR, changed compiler recipe, or probe logger is
adopted as a production fix. A runtime successor must demonstrate a stable
ASLR-on fresh-process distribution across its whole frozen layout/control
portfolio, in addition to correctness and ordinary performance gates.
T072 stays closed without promotion; its two charged attempts are unchanged.

## Verification and handoff

`TEST262_DIR=<shared pinned corpus> ./scripts/check.sh` passes after all
collector source changes: 2,382 runtime tests, 732 benchmark-tool tests,
35 script tests, context/size/hooks and 5,170 curated Test262 cases. Final log
SHA-256 `50c772f65bee5fec5b6f08950b9c7b429e93f87f5927b72cbc1dfc4b7c8ff8db`.
Diagnostic Python syntax, warning-clean C syntax/build, exact-image code
comparison and executed collector/checksum checks also pass. No complete
Test262 scan or new conformance standing is claimed for this documentation
and diagnostic-tool change.

All fourteen diagnostic jobs are finished. The retained debugger pilot is
OS-denied; the PIE-header pilot cannot separate code/stack randomization and
therefore correctly skips its conditional distribution. Every failed or
negative result remains in the measurement record. The engine source remains
exactly the branch base. Cleanup runs `cargo clean` (6,276 files / 4.0 GiB),
then `git checkout -- target` restores the tracked fixture. Local free space
is 53 GiB. Measured queue assets are retained outside this worktree; no build
output is committed. The staged fast gate passes (context selected; no Rust
semantics changed, so no focused semantic slice is selected). The full gate
already ran all 5,170 curated Test262 cases. No origin push or merge occurs.
