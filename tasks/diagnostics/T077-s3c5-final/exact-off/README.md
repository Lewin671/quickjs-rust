# Complete production IC-off coverage

Request revision **8bff8431**, runtime **2aa117b7**, native orders **476f1bea**.
The canonical production agents binary with QJS_DISABLE_IC=1 passes
**42,672/42,672 configured cases**, with **zero failures, timeouts or omissions**.
Four unfiltered unlimited shards form the exact **53,572**-path pinned corpus.
NG is **42,602 pass / 70 fail**, with **10,900** shared configuration skips.
No gap or lost pass remains relative to the preceding complete standing.

The request, complete result, four original summaries and case-file hashes
are retained. Burndown appends this complete observation independently of the
cached and diagnostic scans. Runtime and exclusions are unchanged. The
release IC-disable policy is effective without debug assertions.
