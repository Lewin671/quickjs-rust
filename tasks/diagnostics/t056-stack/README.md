# T056 stack-phase evidence and lead-only probe

[Diagnosis](../../archive/T056-stack-phase-diagnosis.md) owns conclusions,
addresses, corrected Increment audit, startup attribution and proposed fixes.
Production remains dfff. No timing job or acceptance attempt is submitted here.

## Evidence producers and identities

- `capture.py`/`render_capture.py`: non-timing terminal image-copy snapshot.
  Original canonical caller/prologue/text remains exact except10B at first
  dispatch and the terminal bridge in an unexecuted handler region. No logger
  call, engine allocation or added native frame. `push %rsp` saves the original
  header RSP before temporary pushes. Bridge writes56B then exits; it is never
  a timing/semantics image. Resigning must preserve all mapped sections.
- `20261021T092005Z-t056-stack-snapshot-v2`:12 valid N/2N snapshots; first version
  `091853Z` failed its expected branch encoding guard before execution.
- `20261021T092318Z-t056-stack-phase-map`:240 snapshots, coarse/fine pilot;
  the coarse256B step alone can skip a narrow alias window.
- `20261021T093012Z-t056-stack-geometry-qualification`: first address-only grid.
- `20261021T093204Z-t056-stack-geometry-final`: finalgeometry29 phases/348 rows,
  done0/4s. `snapshot-evidence.json` retains all registers/launch sizes and
  deduplicates env length maps; its raw_job_snapshot_sha256 matches the original
  indented runner file. No environment secret values are retained.
- `audit.py`/`extract-paths.py`: read-only audit of canonical images. The updated
  finite-number Increment branch is18 instructions/86B, not the old selected
  Decrement17/81. Both new audits match all selected paths modulo relocations.
- `startup.py`: canonical instruction totals plus independent agents/perf-counter
  archive builds; adds event counters only in private archives. Job
  `20261021T092319Z-t056-startup-event-attribution`, done0/215s.
- `bootstrap.py`: separate archive copies the previous diagnostic source, adds
  an end-of-intrinsics event snapshot; `20261021T093246Z-t056-startup-bootstrap-events`,
  done0/70s. No production edit, FFI, Rust unsafe or dependency.

Raw bodies/logs/binaries remain in named queue outputs; committed JSON records
include revisions, hashes, actual addresses and counts. The terminal snapshots
are diagnostic copies with exactly preserved frame layout, not canonical file
hashes or old timing-run register captures. LLDB launch denied noninteractive
debug permission, and externally signalled crash report lacked thread state;
those failed alternatives (`091115Z`,`091341Z`) are not register evidence.
No debug permissions, services, allocator defaults or machine settings changed.

## Preregistered initial-stack experiment (lead only; ≤25min)

Both original canonical images f58/dfff, global_read/local_read/empty, four
paired blocks. At each phase use base,candidate,and separate-inode byte-identical
A/A clones of both (4 roles). All invoke one active pathname, identical source,
N=23,068,672 and2N=46,137,344; N/2N have the same digit width. Code, bytecode,
zone and engine allocation policy do not change. Every role clears old zone/
trace/cookie keys. Only the value length of an unknown fixed-name environment
entry `T056_INITIAL_STACK_PAD` changes; the entry always exists, including the
zero-length control. No CLI padding shim or extra engine allocation is added.
Absolute heap ASLR and incidental allocator histories cannot be frozen by this.

Before reading any canonical costs, terminal snapshots use the same active
path, `/usr/bin/time -l` parent, argv and env as the real samples. Capture both
versions' global seed addresses, then seal **one shared grid**:

- Coarse pads0,256,...,3840 (one full4096-byte cycle).
- For each version's seed, anchor=`(stack_slot−bank) mod4096`; include
  anchor+{-64,-32,-16,0,16,32,64}, wrapped modulo4096.
- Sorted union, at most30 phases; formula fixed before results, no wall-based
  offset choice. Anchor addresses, exact pads and full request are written
  before canonical sampling. This is address-based qualification, not a search
  for a fast phase. Same pads apply to both sides and all three cases.

Recheck all phases, both cases and N/2N: RSP/slot must move by−pad modulo4096;
N/2N phase must agree; group must include predicted overlaps and clean points.
Track bank/program/ops low-bit histograms and all exceptions. Candidate bank
must stay fixed. Do not remove anomalous processes, substitute offsets or retry.
The completed qualification observed rare heap-offset changes on base/local
paths; candidate global was stable. If candidate-global program/ops move during
live calibration, a flat result cannot reject the hypothesis. Empty has no
claimed bank/typed path; it is a whole-process setup/launch control, never
subtracted from internal increments or unpack.

Address prediction uses actual original stack load `[rbp−0x30]` width8 and
actual bank stores (tag bytes, boolean byte, payload8), checking byte ranges and
4B/8B granules modulo4096. The selected loop has no stack stores. Predicted
collision means a candidate partial-address match, not proof of a Comet Lake
hardware penalty; published AVX examples do not specify every scalar comparator.

All data and full per-phase C/B/A/A ratios are retained. `assessment.json`
predefines collision-vs-clean geometric-mean contrasts **per block**, using
address labels only; report95% block-bootstrap CIs and max/min phase medians.
Ambiguous labels remain in raw/per-phase outputs and prevent a primary rejection;
they are not deleted to improve an estimate. Descriptive CIs are not acceptance.

Primary=dfff global. Require hit/clean coverage, stable bank/program/ops low
bits, no ambiguous labels, instruction max/min≤1.01, and both global A/A median
in[1/1.03,1.03],95% wall CI within[1/1.10,1.10], instruction median within1%.
Then:

- If wall max/min≤1.05 **and** hit/clean95% CI entirely within[1/1.05,1.05],
  reject the stack-bank explanation **under sampled conditions**. Old unrecorded
  timing-phase addresses remain unverified.
- If wall span≥1.10 and hit/clean median≥1.10 with lowerCI>1.03, report support
  for stack-phase sensitivity. It is not by itself a hardware-cause proof, nor
  attribution of the old43% event when that process's phase is unknown.
- Otherwise inconclusive. No favorable phase, run selection, acceptance or
  attempt expenditure follows automatically. General data/code alternatives
  remain open; program+8 versus bank+8 is a separate common alias candidate.

## Budget, guards and execution

Real reference: cross960 units in600.242272s. Maximum30×3×4×4=1440 units projects
900.36s ×1.2 +120s setup/statistics =1200.44s. Collection cap1260s, body1380s,
whole running job including quieting1500s; queue wait excluded. Mix differences
make completion an estimate. Timeout retains partial evidence and gives no
verdict; no retries or pair deletion. Both wrapper and own-job watchdog required.
Host-local03:00–05:00 excluded with15-minute margin and30-minute admission
headroom; /etc/localtime overrides inherited TZ. Same measurement lock and
load monitor as prior diagnostics; any heavy event invalidates the whole probe.

Sealed `config.json` binds original canonical/correctness86 receipts, snapshot
image hashes, collector/protocol/helpers. `validate.py` parses rendered code,
checks geometry for every16B-aligned phase, uses real sealed receipts/binaries
with **fake** subprocess/counters/phase reads through the actual collector,
and verifies AA, env reset, groups, flat/sensitive/inconclusive branches and
clock/DST/deadline guards. It starts no engine and submits no timing job.

After reviewing the committed protocol, **lead may execute**:

```sh
python3 tasks/diagnostics/t056-stack/validate.py
python3 tasks/diagnostics/t056-stack/render.py > /tmp/t056-stack-phase.sh
# LEAD ONLY; the owner does not execute the following two lines.
job=$(/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run --repo "$PWD" --timing --ref HEAD --with f58c696e8379261ed726f0cea1051ed6d34f431a --with dfff3bb6bf3db01fb52a7612ed1144900b5f08c5 --name t056-stack-phase-probe -- bash -c "$(cat /tmp/t056-stack-phase.sh)")
python3 tasks/diagnostics/t056-stack/watch.py "$job"
```

Fetch even on timeout. Only complete.json plus sealed calibration/request,
all phase/case files, raw samples, monitor and both A/A sets is a complete
probe. Outputs are `t056-stack-phase-diagnostic`, decision_evidence=false,
attempts_consumed=0, and cannot satisfy or amend the frozen acceptance plan.
