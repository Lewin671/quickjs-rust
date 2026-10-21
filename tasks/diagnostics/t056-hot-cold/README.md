# T056 hot/cold candidate: static prerequisite failed

Runtime `5dd3a810cac6a88f6a1f86c0ef46dc68dfac47ee`, canonical image
`26ed840a9cbb5bc17c2993ad7fb61380bbeab03b3a5bf39acbe220512da783fc`.
[Owning record](../../archive/T056-hot-cold-preparation.md) contains results and limits.
The112B header and instruction/allocation repairs are present and locally verified.
**The required fast relative-PC family is not preserved. Do not submit timing or
spend the last frozen attempt on this image.** `fast-family.json` records failure;
03 and mini reject it explicitly. Do not flip that guard or reuse older receipts.
No owner timing was submitted. The PMU path remains closed; leave its installed helper untouched.

For provenance/reproduction, `render-owner.py qualification|layouts` prints the exact
producer sources in a non-timing queue body. Submit only with `--ref` equal to the
runtime commit above (plus base/parent via `--with`), without `--timing`. The layout
producer retains its original exact86 job binding; it does not select from reruns.

## Architecture and evidence

[Header plan](implementation-plan.json), [summary refinement](summary-plan.json),
[instruction plan](instruction-plan.json) and [shared-key plan](shared-key-plan.json)
precede their edits. Native-width index count is cold; an independent inline
boolean guards hot indexless checks. No new packing, narrowing, header padding,
unsafe, dependency, parser/binding/tier or compiler-recipe change.

`probe.py <exact-HEAD>` builds canonical base/120B parent/candidate, counts three
warmed whole-process instruction samples (empty/CLI/unpack/class-fields), and
runs the existing static numeric decoder. `qualify.py` adds all86 original
instruction gates; wall fields are discarded. `native.py` captures native
MallocStackLoggingNoCompact histories for empty/unpack and queries every request's
actual host malloc_good_size. `events.py` instruments an isolated archive for
startup/layout/key/class-segment events; it never supplies canonical timing or
instructions. `displacements.py` compares actual emitted operands/lengths;
`fast-family.py` separately checks the exact parent relative-PC family.

`header-*` is8dc59f64, `repair52380-*` is52380c2e; neither qualifies5dd3a810.
Final receipts have unsuffixed names; [readiness](readiness.json) is explicitly false for timing. Raw allocation histories, full stack groups
and assembly remain in their named queue jobs; conclusions/numbers/revisions and
hashes are recorded here and in the owning record. Source scope is in
[source-scope.json](source-scope.json). Frozen unit/policy/old evidence are untouched.

## Prepared lead chain (timing blocked)

1. `lead/render.py 01`: fresh full local gate, NG255/subset5170 and eight exact
   both-engine Test262 shards;53572 pinned paths/42672 configured cases zero-gap.
2. `lead/render.py 02`: require01, bind identical candidate/image, original86
   instruction admission max(1.03, policy band). Owner counts cannot replace it.
3. `lead/render.py 03`: require01/02 and static fast-family pass, then original
   T061 controls+14 focuses (28 unique cases; class-fields already a control).
4. `panel/render.py mini`: require01/02/03 and static pass, then global/local/
   unpack/class-fields/empty, six prescribed offsets×five pairs, both A/A sides.

Renderers print self-contained bodies only; prerequisite job IDs are arguments
to those bodies. Future qualifying source needs a newly bound/sealed package,
not edited hashes or a flipped boolean. Full exact correctness of5dd3a810 is
**pending lead**;8f898dd9's exact pass cannot cover it. No acceptance command is supplied. [Concrete chain](lead/commands.sh) checks the failed
static prerequisite before submitting any job; individual01/02 renderers remain available for verification.

The five-case mini is600 case-role blocks,840 measured+600 preflight calls.
The original401.0238s/540-unit reference with1.4× reserve+180s projects803.815s,
within1560s collection/1680s body/1800s running watchdog, including quieting and
excluding queued wait.35min admission, host-local03–05 exclusion and15min-before03
guard remain. N/2N is23068672/46137344 for both internals, whole for the others.
No builds occur in timing. All prescribed offsets0/16/64/240/512/1008 are retained;
no favorable selection, pair deletion or retry. An incomplete/disturbed job stays invalid.

All A/A wall medians3%/CI10%, instructions1%; empty wall10%/CI25%.
Global disappearance requires median≤1.05, upper95% CI≤1.1131654120394188,
instructions≤1.0069 in≥5/6 layouts; persistence median>1.05, lowerCI>1 with
same instruction condition in≥5/6. A/A failure/mixed/incomplete is inconclusive.
These diagnostic rules and the frozen full acceptance/attempt budget are unchanged.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-hot-cold/lead/validate-local.py
PYTHONDONTWRITEBYTECODE=1 python3 tasks/diagnostics/t056-hot-cold/panel/validate.py
python3 tasks/diagnostics/t056-hot-cold/lead/render.py 01 > /tmp/t056-hot-cold-01.sh
python3 tasks/diagnostics/t056-hot-cold/lead/render.py 02 > /tmp/t056-hot-cold-02.sh
python3 tasks/diagnostics/t056-hot-cold/lead/render.py 03 > /tmp/t056-hot-cold-03.sh
python3 tasks/diagnostics/t056-hot-cold/panel/render.py mini > /tmp/t056-hot-cold-mini.sh
```

Validators parse scripts, reject identity/status/disturbance/static failures,
check DST/deadlines, retain original routing boundaries and simulate all600 units.
The simulation uses fake processes/counters, qualified image hashes and **mocked
future correctness/pre-gate/triage/static receipts**; it cannot waive the real
failed static prerequisite. Reruns write under target/. The sealed validation
snapshots describe simulation only, not real exact correctness or timing.
