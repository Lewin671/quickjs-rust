# Production IC-off focused check

Canonical production agents binary from the ARM order receipt, runtime content
2aa117b7, with QJS_DISABLE_IC=1, QJS_AGENTS=1, one worker, 20-second timeout
and one retry. All six coerced-values copyWithin cases pass in both engines.
This filtered check is not a complete scan and is never a burndown record.

The disabled-IC policy is read unconditionally in property/atom.rs, independently
of debug assertions. Verification/poison flags separately require assertions.
Complete production cached and IC-off scans retain the same timeout and input
configuration, and will establish the qualifying parity standing. No runtime
source or exclusion changes follow this check.
