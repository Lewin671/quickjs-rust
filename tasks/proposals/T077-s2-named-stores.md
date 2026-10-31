# T077 S2 implementation boundary

Source base: `b2daec46`, branch `agent/t077-s2-named-stores/codex`.
Owner authorization is the self-contained S2 brief dated 2026-10-30. Work and
commits stay in this worktree; no push, merge or main-checkout edits.

## Frozen scope and sequence

1. Replace function text/symbol maps and their independent ordering index with
   the S1 NamedStore. Keep standard lazy descriptors as explicit hooks; once
   materialized, ordering and descriptors belong solely to that store. Port
   every installer/consumer to the function semantic API.
2. Replace array text/symbol maps with NamedStore, passing the realm domain
   through construction. Keep dense indices, sparse descriptor hooks and length
   semantics explicit. No per-array interner or second named authority.
3. Replace exact-value/receiver and per-storage cache variants with weak
   shape/version + live-slot entries. Mono, allocated four-way poly, then
   generic megamorphic; inherited hits guard the first edge and registered
   structural dependencies. Port every executor and enumeration consumer.
4. Verify IC-off and debug cached-versus-generic semantics, all stage gates,
   receiver-family counters and committed-candidate iMac triage.

No new dependencies, crates, unsafe, public semantic error ABI, alternative
storage path, benchmark recognizer, or environment-model changes. Any necessary
sound contract correction is recorded in T077 before relying on it.

## Acceptance and inventories

The case inventory is unchanged: all runtime tests, all NG differential fixtures,
curated Test262, and the complete pinned configured exact corpus (42,672 cases
at base; no new exclusions). Add focused structural/lazy/exotic discriminators.
`check.sh`, combined agents/perf-counters tests, both production-target size
checks, `compare-qjs.sh`, complete exact scan with timeout 20 and its burndown
are mandatory. IC-off complete correctness and debug data-probe verification
must not replay accessors or coercions.

Measure only on the iMac queue, respecting its quiet window and other jobs.
Development quick-triage compares against `dd0905b2`; counters cover the ten
development programs only. The lead owns formal acceptance: development,
external, equal-case selection and held-out aggregates must all improve against
the rebuild baseline and accepted predecessor under T077's existing precision
and health rule. Held-out inputs never guide implementation.

Deletion receipt will name retired paths/line counts; final evidence records
revision, sizes, counters, job names/numbers, exact commands and unresolved
formal standing. Intermediate commits are verified increments, not S2 acceptance.
