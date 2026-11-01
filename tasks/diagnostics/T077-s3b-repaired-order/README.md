# S3b repaired-source x86 order

Job `20261101T002031Z-t077-s3b-repaired-order` exits 0 after 127 seconds.
Source `1e91be92c2af7b876efdc68f7d8698820b44dfe1` includes main `3cc12c6f`
and the exact-checkpoint repairs. The [actual case list](cases.json) matches the
previous default selection exactly: 76 external/sentinel/broad cases, no
development or held-out. Default sampling/coverage/pin offset remain unchanged.

Canonical x86 release profiling input SHA-256:
`93ae6023c6e2db795363bba6fc7a1e1a3d6abf134ec67ebe15635281c911da30`.
The new 496-symbol order SHA-256:
`79bafb11370667e4098ab295586b2b01f0d97e21db1855978b5c520f69eff70a`.
The final x86 candidate must be rebuilt after installing this order; the
profiling input is not a final timing binary.

[Request](request.json), [result](result.json), [command](command.sh),
[log](log) and [status](status) retain the exact runner evidence. The command
checks CST and stops its own profiling group before the blackout. This job
establishes no public86/formal/held-out/RSS performance result. The hostname
qualification decision for public86 is still pending.
