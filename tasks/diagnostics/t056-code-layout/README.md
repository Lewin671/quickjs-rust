# T056 code-layout diagnostics (lead-only, prospective)

[Architecture review](../../archive/T056-architecture-review.md) owns the code
kernel proposal and historical evidence. This package changes no production
code, acceptance policy or attempt budget. **The owner submits no timing jobs.**
These outputs have artifact type `t056-code-layout-diagnostic`, not a selection,
formal or acceptance panel. The old data-first probe remains historical.

## Fixed question and preregistered routing

A single natural-layout triage cannot establish persistence. Test unchanged
**dfff3bb6 vs f58c696e**, using the frozen plan's six offsets
`0,16,64,240,512,1008` and **five pairs per layout**. Every layout has base,
candidate and a separate-inode byte-identical A/A clone of each; four roles,
all invoked through the same active pathname. No favorable-layout selection,
rerun, pair deletion, data factor or recompilation during collection.

Nine prespecified cases: global_read, local_read, branch_arithmetic, whole
string-unpack-code, heterogeneous_property_read, array_read, captured_read,
whole bitops-bitwise-and, empty. Last four are controls: empty reports launch
cost; the other three cover established array/cell/bitwise operations with
existing policy measurements. “Control” is not an assumption that dfff must
improve them; all results are retained, including instruction changes.

Decision rule (diagnostic routing only, implemented in `analyze.route`):

1. Require the complete6×9×5×4 matrix, correct outputs, qualified images and no
   heavy-load monitor event. Every A/A must have wall median in[1/1.03,1.03],
   its95% paired-block CI in[1/1.10,1.10], instruction median in[1/1.01,1.01].
   Empty's whole-process startup is noisier: median[1/1.10,1.10], CI[1/1.25,1.25].
   Failure makes routing **inconclusive**, not a retry or permission to spend.
2. Global excess “disappears” in a layout when C/B wall median≤1.05, upper95%
   CI≤the unchanged policy band1.1131654120394188, and instruction median≤1.0069.
   The5% diagnostic threshold is set before timing, distinguishing the large
   natural event from the residual≤4.14% seen in T058's other five layouts;
   it does not replace any frozen acceptance threshold.
3. If disappearance holds in **at least5/6** layouts and A/A qualifies, lead
   **may consider** using the last attempt on the **unchanged dfff** under the
   full frozen plan. This mini-panel grants no acceptance, promotion, budget
   reset, case waiver or automatic attempt submission. All other cases remain
   visible for that decision and still face the complete original gates.
4. “Persistent” means global wall median>1.05, lower95% CI>1.0, instruction
   median≤1.0069 in at least5/6 layouts. Do **not** spend the attempt; run the
   code×data2×2 below next. Mixed/uncertain outcomes also do not support
   spending it; investigate without choosing another favorable offset/factor.

These are per-layout paired medians with descriptive95% block-bootstrap CIs,
not a multiple-testing acceptance procedure. A code-position response does not
identify a specific cache/predictor or exclude code/RO-data/stack interactions.

## Follow-up: fixed code×zone2×2

Four cases: global/local/empty/unpack. Both versions use code offsets0 and16B,
and each **same code image** is run with `MallocNanoZone=0` and`=1` (equal-length
values). There is no CLI padding shim and no runtime/source rebuild between
zone cells. Across code levels whole image hashes differ, while the producer
proves body equality modulo relocations and a uniform16B text displacement.
The0-level is the original canonical image. This is a controlled relink, not
an assertion that two different code placements have the same file bytes.

Each of the eight version×code×zone cells has its own A/A clone:16 roles,
15 blocks, no case/factor adaptation. Same-source/code-level factors share an
inode across zones; each A/A copy has a separate inode. Report both code main
effects, zone main effects at both code levels, multiplicative code×zone
interaction, C/B at all four cells, and all eight A/A effects. Same-image zone
contrasts isolate an environment factor from code recompilation; requested
zone settings do not prove specific object addresses or physical cache sets.
Do not attribute an interaction to4K aliasing without additional evidence.
The persistent-mini result is a lead routing decision, not an owner timing
trigger. Cross produces diagnostics, never an acceptance/attempt record.

## Build, identity and correctness preparation (non-timing)

`build.py` uses the existing qualified base matrix from
`20261020T210104Z-t056-m1a-chain-ready`, checks every original hash/host, then
uses `tools.benchmark.layout_variants.build` for untouched dfff. Each shifted
image must have zero body/address drift and the declared uniform displacement.
The base512/1008 variants additionally shift the entire text section by12,288B;
their section-relative shifts remain512/1008, as allowed by the frozen producer.
All actual shifts are retained. Both16B images have an exact16B total shift
for the2×2. Natural hashes must be f58=`b0dfc65d…737f0e`, dfff=`3466c37a…b1e3f` (complete
values in `protocol.py`). All12 images pass NG255 and matching outputs on the
nine cases at smoke N1000. Compilation/qualification is separate from the
≤30-minute timing job: no build, link or calibration occurs inside timing.
`qualify_zones.py` checks NG255 and the four outputs in all eight2×2 cells.
Receipt job IDs, hashes and image identity are sealed in `config.json` and
committed `qualification.json`/`zones.json`; collection rejects stale/missing receipts.
The unchanged dfff full/exact/86 chain is also required, with its original
helper hash. Production source and unit hash must match dfff/frozen plan.

Render-only non-timing producer: `python3 tasks/diagnostics/t056-code-layout/render_build.py`.
It prints the payload and never submits a job. Completed non-timing qualifications:

- `20261021T082252Z-t056-code-layout-qualification`: done0 in382s; all12 images,
  NG255 per image, fixed nine outputs equal; all body/address audits pass.
- `20261021T083025Z-t056-code-zone-qualification`: done0 in18s; all eight
  code×zone cells pass NG255 and four matching outputs.
- `validation.json`: actual collector over simulated processes/counters completes
  both full matrices, checks A/A inode/environment rules, route boundaries,
  night/DST guards and30-minute watchdog; starts no JS engine or timing job.

No timing result is recorded. `render_build.py` reproduces the build payload;
`qualify_zones.py` takes `BENCH_OUT/zone-config.json` with build_job/build_sha256
(as sealed in config.json) and must run without --timing.

## Timing budgets and execution (lead only)

Real throughput reference: data/launch diagnostic
`20261021T040650Z-t056-m1a-data-probe` completed540 case-role-block units in
**401.0238s**, including per-role preflights and the same250ms-scale internal
N/2N scheme. Mini has1080 units: scaled802.05s ×1.4 +180s setup/analysis
reserve =**1302.87s**. Cross has960 units:712.93s ×1.4 +180=**1178.10s**.
Mix differences, A/A copies and statistics make these estimates, not promises.
Fixed N (no timed calibration):23,068,672 global/local;13,182,098 branch;
2,883,584 heterogeneous;1,153,433 array;8,388,608 captured; external/empty whole.
These come from completed lead triage/pre-gate; every internal wall increment
must still meet the standard minimum or the whole diagnostic is incomplete.

For **each** job: collection cap1560s, body cap1680s, whole running-job cap
**1800s including runner quieting**; queue wait excluded. A watchdog and own-tree
body deadline both apply. Retain raw partial samples on timeout and issue no
routing. No invalid-pair repair, retries or deletions. Rotate/reverse layouts,
cases, roles; alternate N/2N order. TZ=UTC/LC_ALL=C, reset all old allocator/
trace/cookie keys each role. Empty is reported separately, never subtracted
from unpack. Host-local03:00–05:00 excluded with15-minute margin; admission
needs35-minute headroom; /etc/localtime is authoritative, not inherited TZ.

After reviewing the committed protocol, **lead may submit one kind**:

```sh
python3 tasks/diagnostics/t056-code-layout/validate.py
kind=mini # cross only after the lead's routing decision
python3 tasks/diagnostics/t056-code-layout/render.py "$kind" > /tmp/t056-code-layout.sh
# LEAD ONLY; the owner does not execute these two lines.
job=$(/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run --repo "$PWD" --timing --ref HEAD --with f58c696e8379261ed726f0cea1051ed6d34f431a --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 --name "t056-code-layout-$kind" -- bash -c "$(cat /tmp/t056-code-layout.sh)")
python3 tasks/diagnostics/t056-code-layout/watch.py "$job"
```

Always fetch, including on timeout. `complete.json`, every raw row, image
receipts, monitor and all layout/case files are required together. All results
remain diagnostic with`attempts_consumed=0`. Do not feed them to the frozen
plan's acceptance/decision tools or append them as a completed full K=6 screen.
