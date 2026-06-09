# Harness Convergence Risks

Recorded: 2026-06-09.

This document records known structural problems in the conformance harness
workflow that are expected to slow QuickJS-NG alignment as the project moves
past the quick-win phase. It only records the problems; it does not prescribe
solutions or schedule work. When a problem here is addressed, update or remove
its entry in the same reviewable unit.

## 1. Quick-win supply is running out and broad features have no campaign-level decomposition

The default `find-qjsng-gaps.sh` quickwins strategy is designed to harvest
small, reviewable gaps and to de-prioritize areas whose metadata hints at
broad missing features (async, class, generators, modules, destructuring,
proxy/realm/species, resizable buffers, Annex B global code). That bias was
correct early on, but the remaining gap mass is concentrating in exactly those
de-prioritized features.

The harness can point at a broad feature area, but nothing in the repository
decomposes such a feature into independently verifiable slices. `tasks/` still
only contains the early bootstrap tasks (T001-T005). The expected failure mode
is that the recommendation queue becomes all hard areas, and each agent
iteration re-evaluates the same large features from scratch instead of
advancing a planned slice.

## 2. Structural not-run exclusions are a blind spot with no unlock signal

The baseline classifies modules, async tests, intl402, unsupported harness
includes, fixtures, and known unsupported syntax as structural not-run cases,
and they are excluded from the actionable gap list. This keeps early signal
clean, but it hides a large fraction of Test262 from the entire discovery
loop. No mechanism records what each exclusion is waiting on or signals when a
not-run category should be unlocked, so full NG parity work behind these
exclusions stays invisible by default.

## 3. The harness itself is large untested shell logic

`find-qjsng-gaps.sh` (~860 lines) and `test262-baseline.sh` (~700 lines)
contain real program logic: JSONL processing, greedy ranking, hard-hint
weighting, and concurrent sharded probing, implemented in bash and awk with no
tests of their own. The harness changes frequently and is the leverage point
for all agent throughput, but every strategy adjustment currently carries
untested regression risk that only manual inspection can catch.

## 4. Default probe sampling is deterministic and decays in value

Unfiltered global probes always sample the same fixed shards
(`TEST262_GAP_PROBE_SHARDS`, currently `1/16,5/16,9/16,13/16`) with a fixed
per-shard limit. As the sampled regions are cleared, repeated default probes
re-scan the same cases and surface less new information, while gaps in the
other shards only become visible through explicit `--filter` runs or an
`--exact --all` audit.
