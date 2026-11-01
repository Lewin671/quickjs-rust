# S3b public86 host qualification checkpoint

Engine candidate `73c93d8f`, baseline main `3cc12c6f`. This is a failed
qualification attempt, not performance evidence or stage acceptance.

- Runner job `20261031T233745Z-t077-s3b-merged-public86` exited 2 before
  collecting measurements: `layout policy host/recipe mismatch: recalibration required`.
  Its original command, log and exit status are retained here.
- Read-only audit job `20261031T234927Z-t077-s3b-host-identity` exited 0.
  [Identity audit](identity.json) records expected `iMac.local`, actual `iMac.lan`,
  and no differences in the entire qualified hardware, OS, toolchain, recipe,
  Cargo configuration and manifest identity dictionary.
- Neither the hostname guard nor the frozen v5 policy was changed. The policy's
  SHA-256 is `f3e8ef40b8b45601b5b86edaf4b47c2d9d904d0045eb024e837f1c447a52bec4`.
  The existing v6 policy expects the same old hostname.

An explicit user decision is pending on an alias-only guard change. The proposed
change would recognize this exact hostname pair while retaining the complete
identity comparison, unchanged thresholds and actual hostname in requests.
Approval has not been inferred from posting the question. Until qualification is
resolved, no valid merged public86 result exists. Complete exact Test262 scans
continue independently.
