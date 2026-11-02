# Complete assertion-enabled IC-off scan: not qualified

Source/order content matches the [cached diagnostic](../exact-diagnostic-cached/README.md),
request commit **1b67d62b**, runtime **2aa117b7**. The four-shard union covers
all **53,572** paths: **42,669 Rust passes, 0 failures, 3 timeouts**, NG
**42,602 pass / 70 fail**. The same three large coerced-values detachment
cases time out. Requests, four summaries, complete result, timeout paths and
case-file hashes are retained. The complete observation is appended to
burndown unchanged; no focused result replaces its rows.

This assertion/overflow-enabled build is not the qualifying production scan.
The [production IC-off focused check](../release-ic-off-probe/README.md) passes
all six coerced-values cases at the same timeout, and the complete production
cached/IC-off scans are running. Runtime, exclusions and timeout policy are
unchanged. Full debug stress/poison/verification already passes separately.
