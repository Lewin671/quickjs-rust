# T056 bidirectional source bisection

[Record](../../archive/T056-source-bisect.md) owns conclusions and mechanism
hypotheses. Production remains dfff. No owner timing or acceptance attempt.

`partition.py` exhaustively splits all41 build-input hunks in f84..dfff into
a(header/flags), b(write-index), c(typed adapter), d(order). Applying all groups
recreates dfff; no groups recreates8f. Full hunk text/hashes are in `hunks.json`.
The8f..f84 differences are documentation only. `build.py` writes archives only,
builds eight combinations with the existing canonical recipe, copies canonical
8f/dfff/f58, verifies outputs and audits repeated numeric paths relative to8f.
Its non-timing queue is `20261021T112221Z-t056-source-bisect-build`, done0/764s.
All11 output checks and six numeric-path byte audits pass; no changed hot-path
normalized instruction/constant bytes. Positions differ and are retained.

`images.json` is the sealed output of that build, including eleven binary hashes,
all changed source-file hashes, toolchain/recipe and raw/normalized byte audits.
`audit.py` adapts the existing checked-relocation audit to arbitrary images;
`extract-paths.py` is byte-identical to the corrected stack extractor (Increment).
Builds have no binary patches, resigning, order regeneration beyond group d,
or padding selection. Byte differences, if any, remain visible in the evidence.

## Lead submission (≤25 minutes; no builds)

```sh
python3 tasks/diagnostics/t056-bisect/render.py > /tmp/t056-source-bisect.sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --repo "$PWD" --ref HEAD \
  --with f58c696e8379261ed726f0cea1051ed6d34f431a \
  --with 8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a \
  --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 \
  --name t056-source-bisect --timing \
  -- bash -c "$(cat /tmp/t056-source-bisect.sh)"
python3 tasks/diagnostics/t056-bisect/watch.py JOB_ID
```

Only lead runs this. The collector rejects `no-timing` status. It verifies frozen
unit/policy and correctness/86-case receipts, host, build manifest, all eleven
image hashes and production-tree identity. Copies have separate inodes; one
active hardlink pathname is used. Clear prior diagnostic env/allocator settings;
no current stack-pad experiment. Input sources/case parameters are identical.

Ten images vs f58, global/local/empty, five shared paired blocks, plus A/A for
canonical8f/dfff:13roles,195units,325measured launches,195preflights. All samples
and raw process metadata retained; no discard/retry, no outcome-based grid.
N=23,068,672 and2N, subtraction for broad; empty is whole process. Retired
instructions and wall only, no untrusted rusage-cycle/CPI fallback.

Preregistered source trigger: **plus_group/base≥1.30 AND minus_group/base≤1.05**
on the five-block global wall median. Anchors8f≤1.05 anddfff≥1.30 must reproduce;
global/local canonical A/A median±3%, CI±10%, instructions±1% qualify the label.
Threshold matches are retained even if unqualified. Bootstrap CIs and all
instruction ratios are reported; no CI tightening/relaxing after results.
Empty A/A is descriptive. Multiple/no triggers or one-way results remain
explicit, not an instruction to test more variants or spend the final attempt.

From stack probe1049.23s/1440units,195units≈142.08s;3x+120s reserve≈546.25s.
Collection1200s/body1380s/runner-start guard1490s/watchdog1500s, including quieting
but not queue wait. Host-local03–05 plus15min pre03 margin excluded,30min
admission headroom. Timeout/disturbance/output/hash failure stops and preserves
partial data without a verdict; watch stops this job's descendants only.
`capture_complete` equivalent here is `complete.json`; the assessment is purely
diagnostic and never consumed by formal acceptance tooling.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-bisect/validate.py
```

Local fetched queue outputs are required to recheck receipts/images. Override
`T056_DIAGNOSTIC_QUEUE` if needed. Validation simulates actual collector processes,
not timing/hardware; covers all five blocks, schema/output/deadline/guard paths,
and trigger/no-trigger/one-way/multiple-trigger/AA/anchor failure cases.
