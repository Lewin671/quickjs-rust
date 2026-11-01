# Cost of the global-store repair (deterministic, local)

Instructions retired on this arm64 host (`/usr/bin/time -l`), never wall time.
Before: release CLI of 689efca2 (runtime f7057a51). After: release CLI of the
repair 0148b2eb (same toolchain and profile). `workloads-*.tsv`: one run per
public external/development workload (the 55 programs of the public86 work
directory). `repro-*.txt`: median of three runs per program in
`tasks/diagnostics/T077-s3b-r4/repro/`, including the three global-store
programs added with the repair. Ratios are recorded in the
[progress record](../../T077-s3b-progress.md#round-4-milestone-3-global-store-repair-and-final-requalification).
