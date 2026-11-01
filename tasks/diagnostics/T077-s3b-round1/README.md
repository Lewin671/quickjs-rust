# S3b rejected first freeze

Runtime `0b659b89`, accepted S2 base `1e6e2a2b`. This is historical evidence,
not S3b acceptance. Full `check.sh` passes (5,171 curated Test262 cases); source
also passes 2,601 default/2,655 feature runtime tests and 297 NG scripts.

[Base counts](base-counts.json) and [candidate counts](candidate-counts.json)
reproduce all ten development source hashes/stdout/full maps twice. The fresh
base has **19,031,946 constructions + 72,543 clones = 19,104,489 CallEnv events**;
candidate has zero. The brief's 24,987,337 belongs to the older `dd0905b2` receipt.
These are event counts, not avoided mallocs. Base instrumentation is retained
as a [patch](base-instrumentation.patch); both counter binaries use release plus
perf-counters, with inherited QJS_* variables removed. Input generation uses
`development.fetch_sources` and `external_preview._bundle_source`, with the
pinned manifest and source hashes recorded in each full-map file. Execute each
bundle twice as `<binary> --raw <bundle>` and parse the complete
QJS_PERF_COUNTERS stderr map; assert exact stdout and equal maps.

The candidate's ordinary-call count still misses successful wide numeric roots;
its logical token total (29,298,771, including ten host roots) exposed that
attribution error. The later entry repair adds those roots without changing
production execution. Candidate also loses static tier admission: lodash VM
entries grow 41,661 → 4,557,190 and papaparse 97 → 1,075,619. Fixed-name lease
propagation is over-conservative; these results establish no payoff.

[Base depth](base-depth.json) and [candidate depth](candidate-depth.json) use
`T077-s3a-depth-probe.py --root <build-root> --label <source> --output <json>`
on ordinary CLI main-thread stacks, default features, debug/release, cap 4096.
The cap is a probe bound, not an engine limit. Candidate release call/apply/
Reflect.apply maxima 1814/1399/1454 lose to base 3099/2010/2125. Callback depths
also regress. [ARM prologues](stack-prologues.json) show retained entry/move
storage; general repairs are required. No new depth guard or stack override
was added.

Runner job `20261031T215437Z-t077-s3b-public86` was canceled while queued after
this depth failure; no measurement was collected. Its prospective public-only
[command](public86-command.sh) is retained. No other job was canceled, no held-out
input was read, and no origin push/merge occurred. [Metadata](metadata.json)
records hashes and the source endpoints. Final evidence must use a later freeze.
