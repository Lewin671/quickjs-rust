# Pragmatic stage-A verification bundle

[Final qualification](../../archive/T056-pragmatic-qualification.md);
[PMU is closed](../../archive/T056-pmu-closure.md). Never invoke the installed
helper, change sudo/daemon state or submit owner timing.

Runtime candidate is8f898dd9 (005b8dd4 plus bounded weak-bucket cleanup).
ObjectData120B keeps packed flags, native-width index count and the existing
field order; other object headers and scalar alignment are unchanged. Repeated
large-shape misses share the4-probe index budget. Complete native descriptor
batches publish one final layout. Final canonical image is10211f83…; owner182852Z86/86 and183100Z12-image K=6
qualification passed. The supplied receipts are sealed; fresh full exact
correctness and bound86 remain prerequisites of the lead timing steps.

## Owner preparation, non-timing only

Local full gate, NG255/subset5170, batch semantics/work-count tests and the
same-hash dead-Weak retention regression pass before handoff. `qualify.py`
builds exact canonical images and snapshots all86 original policy cases once,
then reports empty/unpack/CLI instruction totals; wall fields are not timing
evidence. Canonical receipts cannot be transferred from an older image.
`events.py` is an isolated perf-counters build: event counts only, never wall
or canonical instruction evidence. Its005b8dd4 record precedes weak cleanup.

The initial005b8dd4 qualified source/field work and all12 K=6 images are retained
as initial evidence, not a qualification of the final weak-cache-fixed binary.
Final source/header order/types also match the previously measured120B
flags_order variant; the local exact-size assertion and focused tests remain.

## Lead sequence, after final receipts are sealed

All three standard jobs use the exact candidate revision and image:

1. `lead/render.py 01`: full gate, NG255/subset5170,8 disjoint exact Test262
   shards covering53,572 pinned paths and42,672 configured cases zero-gap.
2. `lead/render.py 02`: same-image86-case instruction
   admission, unchanged max(1.03, original instruction band). Owner snapshot
   is useful diagnosis but cannot substitute for01/02 receipts.
3. `lead/render.py 03`: lead-only T061 diagnostic
   triage, original19 controls+13 focus/28 unique cases, five pairs/workers4.
4. `panel/render.py mini`: second
   lead-only diagnostic, global/local/unpack/empty,6 fixed offsets×5 pairs,
   base/candidate plus independent-inode A/A clones at the same active path.

The renderers take only the step/kind; pass prerequisite job IDs to the rendered
shell's arguments as shown in the final commands. Failed, cancelled, disturbed,
stale-candidate, stale-image or tampered receipts block progression. Preserve
all output, raw pairs, load and incomplete evidence. No retries or favorable
case/layout/pair selection. A likely win or successful diagnostic routing never
promotes stage A or spends/restarts an attempt. One of2 remains across A+B.

The mini retains the original numerical routing: all A/A wall medians within
3%,95% CI within10%, instructions within1%; empty wall median10%/CI25%.
Global disappearance requires median≤1.05, upper95% CI≤original1.1131654120394188,
instructions≤1.0069 in≥5/6 layouts. Persistence requires median>1.05, lowerCI>1,
instructions≤1.0069 in≥5/6. A/A failure, mixed or incomplete data is inconclusive.
These are diagnostic rules, not acceptance or a hardware explanation. Other
three cases remain visible; full frozen gates still govern every future attempt.

The480 case-role-block units use the original N values23,068,672/2N for both
internal cases and whole unpack/empty. Original401.0238s/540-unit reference,
1.4×+180s gives679.052s estimate. Existing1560s collection/1680s body/1800s
running watchdog include quieting, exclude queued wait.35min admission,
host-local03–05 exclusion and15min-before03 guard remain; no timezone/service
changes. No build or link takes place inside timing jobs. Code variants are
prebuilt/audited identical bodies modulo relocations with uniform declared
shifts0,16,64,240,512,1008; all12 pass NG255 and matching four-case smoke.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pragmatic/lead/validate-local.py
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-pragmatic/panel/validate.py
```

Validators parse scripts, reject bad prerequisites, preserve frozen unit/
controls and original routing boundaries, check night/DST guards, and simulate
the full mini. The simulation uses fake processes/counters and mocked future
01/02/03 receipts; it does not claim those jobs ran. It never submits timing or
executes an engine/helper. Committed validator snapshots are [lead/validation.json](lead/validation.json)
and [panel/validation.json](panel/validation.json); reruns write under target/.

## Concrete lead commands

Runtime8f898dd96239d392c168bdd20049d073b3b47ff4, canonical SHA-256
`10211f836606e0c14ce5d9a427f818adda1ff2c5b1c8c7cc615860d8b2a4b57e`.
Run these only from the stage-A checkout after inspecting this bundle. The owner
has not executed them. Stop on any failure; always fetch its raw evidence.

```bash
set -euo pipefail
BUNDLE=tasks/diagnostics/t056-pragmatic
RUNNER=/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh
CAND=8f898dd96239d392c168bdd20049d073b3b47ff4
BASE=f58c696e8379261ed726f0cea1051ed6d34f431a
finish() {
  local result=0
  "$RUNNER" wait "$1" || result=$?
  "$RUNNER" fetch "$1"
  return "$result"
}
PYTHONDONTWRITEBYTECODE=1 python3 "$BUNDLE/lead/validate-local.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$BUNDLE/panel/validate.py"
CORRECT=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" \
  --name t056-m1a-pragmatic-correctness \
  -- bash -c "$(python3 "$BUNDLE/lead/render.py" 01)")
finish "$CORRECT"
PREGATE=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" \
  --name t056-m1a-pragmatic-pregate86 \
  -- bash -c "$(python3 "$BUNDLE/lead/render.py" 02)" -- "$CORRECT")
finish "$PREGATE"
TRIAGE=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" --timing \
  --name t056-m1a-pragmatic-triage \
  -- bash -c "$(python3 "$BUNDLE/lead/render.py" 03)" -- "$CORRECT" "$PREGATE")
finish "$TRIAGE"
MINI=$("$RUNNER" run --repo "$PWD" --ref "$CAND" --with "$BASE" --timing \
  --name t056-pragmatic-mini \
  -- bash -c "$(python3 "$BUNDLE/panel/render.py" mini)" -- "$CORRECT" "$PREGATE" "$TRIAGE")
watch_status=0
python3 "$BUNDLE/panel/watch.py" "$MINI" || watch_status=$?
"$RUNNER" fetch "$MINI"
test "$watch_status" -eq 0
```

After01's complete exact scan, record its numbers/revision and append the
verified burndown entry according to the repository procedure; never record a
partial scan. No last-attempt selection/formal/acceptance command is included.
