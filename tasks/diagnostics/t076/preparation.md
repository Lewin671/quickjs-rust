# T076 preparation evidence

Base `19125d48c626869d776df9a501c44f3a00566e32`; no runtime edits.
Selection and staged architecture are owned by [T076](../../T076-executor-bundle.md).

## Baseline instructions and profiles

Non-timing job `20261023T192551Z-t076-baseline-instructions-profiles` runs
collector `f2608217`, exit 0 / 244 s, after T071's queue job finishes. The
canonical image is SHA-256
`cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`, identical
to T075's engine image. All 86 selection cases complete with 351 successful
checksum-checked processes, three repeats, internal N=65,536 and 2N. All 21
seven-program sample receipts complete and conserve first-thread counts.
No held-out input is resolved. Complete instruction-summary SHA-256:
`e0d6e0c14da79acab0780fccad99de1b5a5e4ce6a731943886bdba37e25b911d`.

| Development program | Whole-process retired instructions (three repeats) |
| --- | --- |
| big.js | 24039485555, 24039601158, 24039346966 |
| diff | 44698027012, 44697177801, 44696617964 |
| graphlib | 9028427239, 9028846509, 9029254265 |
| immutable | 22127777462, 22129902684, 22128055940 |
| js-yaml | 26514476276, 26514345593, 26515182909 |
| lodash | 13331522891, 13336789999, 13332343130 |
| matter-js | 30663413195, 30663377636, 30662899095 |
| meriyah | 16998802002, 17001634397, 17000798065 |
| moment | 57659834290, 57657437347, 57657506527 |
| papaparse | 20028766428, 20018861998, 20021765878 |

Before any candidate or frozen plan, this pilot reveals that the short
internal increments are unsuitable for the 0.3% control ceiling: empty-loop
increments 7,000,921 / 7,063,614 / 7,016,839 span 0.8955%; global-read
10,012,779 / 9,939,189 / 9,943,068 span 0.7404%; dynamic-array-read
32,922,716 / 33,145,583 / 32,924,008 span 0.6769%. Unit collections therefore
prospectively use fixed N=2^20 / 2N and five alternating pairs, without any
wall-time calibration. All revisions in a unit's gate use that same request.
Whole external audio-beat-detection also varies (17,323,115,304 /
17,523,764,156 / 17,447,123,790 instructions, 1.1583% range); five-pair medians
reduce sampling sensitivity but do not waive or prove the control ceiling.

## U2 scope diagnosis

A local arm64 `--release --features perf-counters` build at the unchanged
base runs both original T075 target workloads with `QJS_TL_TRACE=4`; no
workload is amplified or edited. These are supplemental IR/route diagnostics,
never timing or canonical instruction evidence. Both scripts exit 0.

Big.js: 29 final regions, 557 final operations, 23 ComputedRead, 40 Box,
44 Unbox and 33 DenseRead. A namespace-aware Box/MoveBoxed scan
clearing facts at branches, jump targets and source/destination writes finds
six ComputedRead keys with an available scalar source; [site inventory](index-eligibility.json).
This supersedes an initial five-site scanner that unnecessarily conflated
scalar/boxed register numbers. It is an eligibility observation, not a proof
of dynamic reach after guards or savings. Matter-js: 39 final regions, 1,344 final operations, zero ComputedRead,
45 ElementRead and 14 DenseRead. Its boxed result accesses already use scalar
indices. Before plan freeze or runtime code, refine U2's selection to require an
eligible baseline final-IR scalar-key path as well as above-mean T075 indexed
reach. Only big.js qualifies; matter-js becomes a <=1.003 instruction control,
while the bundle seven-program union stays unchanged. No candidate performance
result selects this refinement; T045's broader identity-join candidate stays excluded.

Raw traces and process assets are local artifacts, not retained. The counts,
revision and interpretation above are the durable diagnostic record.

## T045 reconciliation

Current base does not contain `dae07218`; the only containing branch is
`agent/typed-boxed-demand/owner`. Queue job
`20261013T135302Z-t045-screen-attempt1` actually finished exit 3 / 268 s:
legacy screen FAIL on `sentinel/polymorphic_call_site` median wall 1.0327 >
1.03. Its reported selection geomeans are instructions 0.9525 and wall 0.9342;
this is historical evidence on the old candidate, not T076 standing or a new
acceptance decision. `tasks/T045-typed-loop-boxed-demand-provenance.md` still
says the screen is pending and is stale outside this task boundary. T076
does not reopen that attempt or import its identity-provenance union. U2
only considers an already-proven scalar key for an existing boxed read.

## Baseline verification

`TEST262_DIR=<shared pinned corpus> ./scripts/check.sh` exits 0 on the
preparation tree over unchanged base runtime: format, clippy/agents/counters,
workspace tests, benchmark-tool tests, plans, context/scripts/hooks/size and
5,170 curated Test262 cases. Log SHA-256:
`fc003346bd51b87c7c7d42313abca5e668ffebff9a7152ce43ef8f5120583fee`.
The task/index/collector commits pass staged context gates; no Rust change
selects a focused semantic slice. No complete exact scan or candidate standing
is claimed. A new frozen-plan validation and runtime gates remain required.

## Instruction A/A qualification

Non-timing job `20261023T193828Z-t076-instruction-aa`, collector `1c00d994`,
exit 0 / 804 s. Both equal-length role copies are exact base `19125d48`,
image `cbb471c7...735e`. All 86 cases finish with five alternating pairs and
internal fixed N=2^20 / 2N (1,170 checksum-checked processes). Raw rusage
hashes/counters, source/image identities, N/2N arithmetic, per-pair ratios
and declared medians replay exactly. [Retained counts](instruction-aa.json)
contain every case, five base/candidate increments, ratios and input hashes.
This is measurement qualification, not a unit target-gain claim.

No control exceeds 1.003. Maximum ratio is imaging-desaturate 1.000676525;
cdjs 1.000442602 and audio-beat-detection 1.000233851 follow. Initial-population
A/A geomeans: U1 0.999986151, U2 (both indexed envelopes) 0.999996325,
U3 0.999987910. The later pre-freeze U2 eligibility refinement changes only
its target membership, not these immutable A/A processes or ratios. Five-pair
collection now resolves the frozen instruction ceiling on this finite A/A;
it does not guarantee future variance or license dropping any process.
Replay also rejects mocked positive wrong counters, negative counters and
changed raw/summary hashes (three fail-closed integrity checks). No metric,
threshold or runtime code is changed after seeing a candidate result.

Disk preflight `20261023T202815Z-t076-disk-preflight`: non-timing, exit 0,
52,550,344,704 bytes free on the iMac at local 04:28:15; >=8 GiB holds.

## Fresh queue and plan freeze

`20261023T210005Z-t076-base-queue`: 30-block base-versus-base, workers 4,
exit 0 / 2,074 s. Starts local 05:00:06, after the prohibited window. Complete
86-case sealed bundle is `ready_for_unit_gates`, with no measurement issue,
imprecise case or intruder sample. Both Rust roles are the canonical image
`cbb471c7...735e` at exact `19125d48`; NG remains `919f7235...6f40`.
This establishes queue eligibility, not a candidate performance result.

Local arm64 queue generation rejects the fetched broad report with
`error: report.json: report differs from raw replay`; no queue or plan is
created from that failed local replay. Cause is not diagnosed. The qualified
measurement-host replay is authoritative: non-timing job
`20261023T213557Z-t076-bind-base-queue`, after the unrelated T071 timing job,
exits 0 / 40 s, replaying all sealed reports and creating queue SHA-256
`fa992f84c3974e9a92dafc89be7dc4c5246c0eb0bdab2f7e6108c08d5b1cfd25`.
No report, raw sample, host identity or frozen evidence is edited.

Local `validate-unit` passes against that queue and all 21 content-verified
receipts; [validation](plan-validation.json) is retained. Frozen plan SHA-256
`de7c229d450e75c7fdd71b91a36180dbabe76ade8650fb42859dcae54c64c73a`.
No runtime file has changed before this freeze. Seven program targets and
all per-unit thresholds, controls, overall/local classification, RSS/resource
chain, incomplete/bimodality rules and two-screen budget are prospective.
