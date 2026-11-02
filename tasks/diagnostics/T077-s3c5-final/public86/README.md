# Frozen final public86: unhealthy collection, stop

Candidate **e37d6871**, runtime **2aa117b7**, native orders **476f1bea**,
base **8322fc98**. The actual measurement job is
**20261102T092436Z-t077-s3c2-final**, submitted outside the blackout after the
owner qualification matrix and both production exact scans completed.

The job exits **1**, raw verdict **unclear**, **DISTURBED**: all four workers
exceed the **20% invalid-pair cap**. No complete healthy case/lane aggregate
is produced. No final ratio, external <=0.983 target or performance acceptance
is claimed. The [summary](summary.json), original request/triage, four worker
errors, command, log, status and intruder log preserve the result unchanged.
Command time **391.1 s**, end-to-end **451.9 s**; budget met does not waive health.

The runner reports other processes using two or more cores in two samples,
including searchpartyuseragent and XprotectService. This observation is
retained, not used to override the failed health gate. Nothing else was
cancelled, and no further measurement or runtime tuning follows this job.

The candidate binary SHA **4779454e0943db30519e52e50ca30cc178b5136d2b7ec9f3cee5cae4ee7df90c**
equals the binary from the qualified x86 order job. Runtime source remains
byte-identical to the final seal. Correctness/structure remains qualified;
healthy performance, held-out, RSS and independent review remain lead-owned.

An earlier [setup-only attempt](setup-only-failure/cleanup.json), job
20261102T092127Z-t077-s3c2-final, exits98 before any benchmark collection:
the order-generation job had left its generated file in the persistent runner
checkout. Under the runner lock, only that known file was restored after its
status/hash matched the order artifact. The same frozen candidate was then
submitted for the one actual collection above. No failed measurement row is
discarded or replaced. Future order generation must restore the runner's
original order after copying artifacts; its working checkout is persistent.

Owner action: **STOP for lead review** under decision3. The final source is
committed on the owner branch only; there is no origin push or main merge.
