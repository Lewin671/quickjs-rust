# Harness Convergence Risks

Recorded: 2026-06-09.

This document records known structural problems in the conformance harness
workflow that are expected to slow QuickJS-NG alignment as the project moves
past the quick-win phase. It only records the problems; it does not prescribe
solutions or schedule work. When a problem here is addressed, update or remove
its entry in the same reviewable unit.

## 1. Structural not-run exclusions are a blind spot with no unlock signal

The baseline classifies modules, async tests, intl402, unsupported harness
includes, fixtures, and known unsupported syntax as structural not-run cases,
and they are excluded from the actionable gap list. This keeps early signal
clean, but it hides a large fraction of Test262 from the entire discovery
loop. The burndown series now tracks the bucket size
(`comparison.ng_pass_rust_not_run`), and the syntax and async exclusions have
unlock slices in the T006 and T007 campaign tasks; the module, intl402, and
unsupported-includes exclusions still have no recorded unlock conditions, so
parity work behind them stays invisible by default.

## 2. The harness itself is large untested shell logic

`find-qjsng-gaps.sh` (~860 lines) and `test262-baseline.sh` (~700 lines)
contain real program logic: JSONL processing, greedy ranking, hard-hint
weighting, and concurrent sharded probing, implemented in bash and awk with no
tests of their own. The harness changes frequently and is the leverage point
for all agent throughput, but every strategy adjustment currently carries
untested regression risk that only manual inspection can catch.

