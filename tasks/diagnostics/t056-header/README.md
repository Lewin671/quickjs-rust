# T056 header factors and whole-loop memory accounting

[Record](../../archive/T056-header-subparts.md) owns conclusions, widths/counts,
limitations and the conditional architecture proposal. Production remains dfff.
No owner timing or acceptance attempt. Group a is the lead's unique diagnostic
match; frozen source-bisect assessment remains inconclusive due candidate A/A.

`parts.py` separates count width, flags representation, and declaration/initializer
order. The last axis includes hot/cold-relative moves; ObjectColdData is unchanged.
Six proper nonempty subsets plus original8f and exact full_a cover all8 cells.
`parts.json` seals sources. `bisect_partition.py` and the [frozen parent hunks](../t056-bisect/hunks.json)
preserve the original a-group definition; empty/full sources reconstruct exactly.

`build.py` constructs original-recipe images in archives only, checks global/local
N/2N outputs and normalized numeric paths. `images.json` indexes source/binary
hashes and per-image audits under `audits/`; its raw queue SHA remains sealed. `-Zprint-type-sizes` is a separate diagnostic
build with an isolated fresh target per variant; its binaries are never timed.
Initial job115238Z built/audited every image, then failed layout parsing; the
initial producer is `build-initial.py`. Its flags-side type output also lacked
a fresh runtime emission because the shared target reused a previous variant.
Qualification120731Z uses `qualify.py` to verify/copy unchanged images and freshly
compile all layouts independently. Qualification `20261021T120731Z-t056-header-layout-qualification` is done0/583s;
only its output is eligible for lead. `layouts.json` contains original compiler
blocks; `layouts-summary.json` records offsets. Count/flags/order alone120B,
count+flags and full_a112B; source axes also affect implicit Rust field order.

`walk.py` independently follows the entire real binary from dispatch0 through
one backedge using actual scalar/op snapshots and original jump tables. It is
operand-driven static replay, **not dynamic branch tracing**. Unknown operations,
control values or memory addresses fail closed. All24 real snapshot replays agree:
global152/local197 instructions, no ObjectData/RefCell access in the repeated
loop. `mechanism.json` indexes the per-role/case `loops/` records with every
instruction/branch/memory EA+width and accounting against five lead blocks. Whole-process instruction fractions
do not establish time fractions. `lead-result.json` preserves the old assessment.

## Lead submission (one job, ≤15min)

```sh
python3 tasks/diagnostics/t056-header/render.py > /tmp/t056-header-subparts.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD \
  --with f58c696e8379261ed726f0cea1051ed6d34f431a \
  --with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a \
  --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 \
  --name t056-header-subparts --timing \
  -- bash -c "$(cat /tmp/t056-header-subparts.sh)"
python3 tasks/diagnostics/t056-header/watch.py JOB_ID
```

No builds in that job. Require exact source-tree identity, frozen plan/policy,
correctness/86-case receipts, host, qualified manifest and all ten binary hashes.
9 images vsf58; A/A for8f/full_a/dfff; five shared paired blocks for global/local;
N=23,068,672 and2N; same active pathname, separate inodes, clean diagnostic env.
130units/260 measured+130preflights. Instructions/wall only, no untrusted cycles.

Single-factor ≥1.30 with its complement ≤1.05 is a bidirectional match. Report all
factorial cells/CIs and minimal observed bad sets, including pair/triple patterns.
Anchor8f≤1.05, dfff/full_a≥1.30 and A/A median±3%/CI±10%/instruction±1% qualify
labels; failed AA preserves raw threshold patterns. No deletion/retry/acceptance.
Previous real throughput predicts111.11s;3x+120s≈453.34s reserve. collect720s,
body840s, runner-start890s guard/watchdog900s including quieting, queue wait
excluded. Host local03–05 and15min pre03 margin excluded,20min admission headroom.
Timeout/disturbance/checksum/hash failures stop with partial evidence, no verdict.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-header/validate.py
```

Fetch the named image/qualification/previous-bisect outputs first; override
`T056_DIAGNOSTIC_QUEUE` for the local queue parent. Actual24 operand snapshots
are re-walked; real collector control flow uses **fake processes**, not timings.
Checks cover 390 calls, factors/AA/anchors, rejection paths, DST and15min guards.
