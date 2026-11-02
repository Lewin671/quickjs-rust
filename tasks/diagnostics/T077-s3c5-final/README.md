# S3c final qualification

Runtime is frozen at **2aa117b7**, native orders **476f1bea**. The
[source/binary seal](freeze.json) matches every runtime source byte and order
file qualified by the [semantic matrix](semantic/README.md). The
[retirement audit](retirement-audit.md) records deletions and approved deferrals.

Owner correctness/structure is complete:

- full gate/5,174 curated, runtime2,753/counters2,805, both target layouts;
- seven stress modes plus combined-off, NG322+10 modules in all nine modes;
- public351 and 8,000 fuzz seeds with no new regression or mode failure;
- debug/release depth >=S3b, deterministic call/method instruction targets;
- [native orders](../T077-s3c5-order/README.md): fresh x86 public profile and
  strict audit; ARM prior-ranking symbol refresh after local sampling failed;
- complete production [cached](exact-cached/README.md) and [IC-off](exact-off/README.md):
  each **42,672 configured passes**, zero failures/timeouts/omissions, all
  **53,572** unique pinned paths, no lost passes, separate burndown records.

The assertion/overflow-enabled [cached](exact-diagnostic-cached/README.md) and
[IC-off](exact-diagnostic-off/README.md) scans remain in burndown with their
three timeouts on large typed-array cases. Production focused and complete
coverage passes at the same timeout; no failed row is substituted or erased.
Debug stress/poison/verification is qualified separately. These are explicit
build distinctions, not a new exclusion or correctness waiver.

The [frozen final public86](public86/README.md), actual collection job
**20261102T092436Z-t077-s3c2-final**, exits **DISTURBED**: all four workers
exceed the 20% invalid-pair cap. Raw verdict is **unclear**, with no healthy
final case/lane aggregates. The setup-only exit98 attempt is recorded separately.
No final performance ratio or acceptance is claimed, and no collection/tuning
retry follows the actual measurement. Runtime remains byte-identical to the seal.

**Owner STOP for lead review**, as decision3 requires after the final job.
Healthy performance, formal/held-out/RSS acceptance and independent review
remain with the lead. No origin push or main merge. Owner correctness is
qualified; S3c is not declared finally accepted.
