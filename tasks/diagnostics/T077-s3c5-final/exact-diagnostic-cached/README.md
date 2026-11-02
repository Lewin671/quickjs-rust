# Complete assertion-enabled cached scan: not qualified

Source/order revision **476f1bea**, runtime content **2aa117b7**. The exact
six-shard union covers all **53,572** pinned paths: **42,669 Rust passes,
0 failures, 3 timeouts**. NG is **42,602 pass / 70 fail**. This complete result
is retained in burndown unchanged; it is not a successful parity claim.

The 20-second budget (one timeout retry) expired on copyWithin's coerced end,
end-with-detached-prototype and start detachment cases. A serial recheck of
the six coerced-values cases on the same assertion/overflow-enabled binary
still has three timeouts. The canonical production agents binary passes all
six under the same timeout. Both focused results are retained separately;
neither replaces rows of this complete scan.

Qualifying complete cached and IC-off scans will use the canonical production
build. `property/atom.rs::DomainState` captures `QJS_DISABLE_IC` regardless of
debug assertions; release IC-off is effective. Full debug stress, poison, IC
verification, public and fuzz matrices are already recorded separately.
Runtime source is unchanged. This is a build/verification distinction, not a
correctness waiver or a new exclusion. Requests, six summaries, complete result
and original case-file hashes preserve the failed diagnostic observation.
