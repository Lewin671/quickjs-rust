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

The one frozen `t077-s3c2-final` public86 is next. No runtime tuning follows
it. Formal, held-out, RSS acceptance and independent review remain lead-owned;
this receipt does not declare S3c finally accepted.
