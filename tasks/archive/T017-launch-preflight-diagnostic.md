# T017 diagnostic: executable first-launch delay

- Status: recorded; this is a measurement-method diagnosis, not a runtime win.
- Implementation: proposed symmetric counter-screen launch preflight.
- Verified at: source `1bbbd875`, 2026-10-12, queued iMac diagnostics below.
- Evidence: exact executable hashes, commands and absolute per-process records;
  raw JSON is workspace-local, not retained.
- Unresolved: precise host service causing the launch delay; the general
  fixed-hardware noise envelope remains unqualified.
- Next action: verify the shared screen method, preserve all measured pairs
  and existing failed attempts, then use only remaining declared attempt budget.

## Byte-identical controls

All copies have candidate SHA
`e079067920853be584e0c022c854ccee46e6992fbdd0ddfe05752817e9e5bdf3`.
Job `20261013T060857Z-startup-byte-identical-aa` executes eight fresh files,
alternating role order against the cached original, then executes each fresh
file again. The unchanged date-format-tofte workload runs in a new process:

| Role | Wall ms range | Child user ms range |
| --- | ---: | ---: |
| Cached original | 116.875–120.504 | 108.472–111.971 |
| New file first launch | 315.983–997.543 | 108.422–112.663 |
| Same new file second launch | 116.802–120.404 | 108.575–111.702 |

Every fresh first launch is slower; ratios range `2.6494–8.4110`, median
`3.0973`. Child CPU is unchanged and major faults are zero. This demonstrates
host new-file launch overhead, not its exact service or an engine-code cause.
It does not erase T042 screen 1's failure or count as a formal noise baseline.

## Fixed tiny preflight

Job `20261013T061314Z-startup-preflight-byte-identical-aa` repeats eight new
candidate files with one prior `--raw -e 0` execution. Preflight wall costs
`206.834–775.277 ms`, child user `1.713–1.797 ms`; those costs are recorded,
not subtracted from any benchmark sample. Fresh first workload executions are
`117.216–119.941 ms` versus cached `118.133–120.771 ms`; ratios range
`0.9750–1.0094`, median `0.9903`.

Job `20261013T061629Z-startup-preflight-base-aa` independently tests eight
fresh base files, all SHA
`19ddbb62222a9c3b4b8578d65d891bd0893f354efdff28ddc092016bd9b648f7`.
Tiny preflights cost `203.336–836.310 ms`, child user `1.695–1.756 ms`.
Fresh first workload executions are `137.647–140.688 ms`, cached
`138.130–144.890 ms`; ratios range `0.9674–1.0121`, median `0.9911`.
Both independent sets remove the identified large new-file startup penalty;
these small diagnostic sets do not establish the complete platform noise budget.

## Shared method boundary

Use the same tiny JS program once per distinct base/candidate executable
before screen pairs, with command, hash, first-launch timestamp/cost in JSON.
Every measured sample still starts a fresh process; process initialization,
parsing, workload and shutdown remain measured. The preflight contains no
benchmark source or identity. Formal external runs already execute a separately
recorded capability sample before measuring each engine/case.

Do not edit frozen plans, alter thresholds, drop measured samples, reset an
attempt budget or reclassify the old failure. Applying this diagnosed shared
method change uses the remaining T042 attempt, not unlimited identical reruns.
