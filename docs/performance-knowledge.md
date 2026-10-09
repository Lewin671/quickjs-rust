# Durable performance knowledge

What we know about making this engine fast that stays true across
revisions. Read it before any performance unit.

Admission rule: an entry belongs here only if it would still hold after a
major refactor. It is a rule with its reason. It carries no measured
numbers, dates, offsets or commit hashes. Ratios, rankings, per-operation
costs, profile conclusions and falsified mechanisms are state: they live in
the unit plans and task files, bound to a base revision, and dated
observations are in
[the archive](../tasks/archive/performance-observations.md).

The procedure is in [performance-workflow.md](performance-workflow.md);
tool and artifact meanings are in [benchmarking.md](benchmarking.md).

## Measuring

- **Judge CPU work by cycles; keep wall time beside them.** Background load
  moves wall time far more than cycles or instructions. Cycles include
  cache misses and stalls but exclude time off-CPU, so a wall-time
  regression that cycles do not show is a real cost the counters cannot see.
- **Measure the noise floor, do not assume it.** Compare a binary against a
  byte-identical copy of itself on the same host and harness. A difference
  inside that spread demonstrates nothing. The floor depends on machine and
  corpus, so re-measure it.
- **Judge a unit on the lane geomean.** An aggregate over many cases
  resolves much smaller effects than one case. Attribute a single-case
  change only when it exceeds that case's own same-binary spread.
- **Compare like with like.** Candidate and base run through the same
  harness, with the same build recipe, interleaved on one host. The path of
  a script or a binary can change its timing, so never compare numbers
  taken along different paths, and warm each new binary once first.
- **Keep samples long enough.** Short runs are dominated by startup, timer
  resolution and scheduling. Raise iterations until a sample clears the
  harness minimum; do not add repetitions of a too-short sample.
- **Measure one operation by increment.** An empty loop body often runs on
  a different tier, so subtracting it measures a tier change. Time N and 2N
  operations in the same body shape, after confirming both run on one tier.
- **Split a slow case into micro-pieces.** Time each piece at N and 2N on
  this engine and on the reference. A piece whose cost grows faster than N,
  or whose increment is far above the reference's, is the one to fix; the
  whole-case ratio hides which piece that is.
- **Serialize timing.** Concurrent builds, tests or agents on the host
  corrupt timings and turn slow cases into spurious timeouts. Code may be
  written in parallel; measurements run one at a time.
- **Diagnostic builds never time.** A `perf-counters` build adds work to the
  paths it observes. Use it to count and trace.

## Knowing what a benchmark exercises

- **A neutrality control must execute the path it guards.** A loop that a
  tier folds or specializes whole performs almost none of its nominal
  operations. Confirm with execution counters or the trace that a case runs
  the path in question before using it as a control.
- **Find the blocker before changing code.** Use the trace histograms and a
  profile of the exact binary to name the instruction, shape or call that
  keeps work on the slow path. A suspected cost is not a reason to optimize.
- **Inclusive profile fractions overlap.** Samples attributed to a caller
  include its callees; never add inclusive fractions as independent costs.

## Codegen

- **Unrelated edits change inlining and layout.** Adding code elsewhere in a
  crate can stop the compiler inlining or unrolling a hot function. When a
  case regresses without an explanation, diff symbol sizes between the two
  builds before blaming the change's logic, and pin load-bearing helpers
  with `#[inline(always)]` or `#[inline(never)]`.
- **Hot code placement is an input, so pin it.** Where a function lands
  decides which cache sets and predictor entries its loops share. An edit
  that only moves hot code can change cycles at identical instruction
  counts. Keep the hot functions in the linker order file and regenerate it
  in the commit that adds a hot function; a stale file surfaces as a
  regression several commits later, where no per-commit comparison sees it.
- **An executor's address and its callees' addresses both matter.** An
  order alone does not hold them, because functions placed earlier change
  size with most edits. Give the executor and each hot callee a fixed slot,
  and re-pin after every change to them.
- **Keep an executor in its caller's codegen unit.** A generic dispatch
  loop is instantiated where it is called; moving that caller to another
  module recompiles the loop differently.
- **Know each case's codegen band.** Functions holding a dispatch loop are
  compiled differently after edits anywhere in the crate, which moves
  call-heavy cases with identical execution counts. Measure the band with a
  semantically neutral rebuild of the base, and judge a control regression
  inside it, with every other hot function byte-identical, on the aggregate.
- **Hand a dispatch loop the storage, not its owner.** Reaching a register
  file through its owning struct adds a pointer load the loop cannot keep
  in a register. When instructions fall and cycles rise, suspect an extra
  indirection before alignment.
- **Keep hot dispatch arms tiny.** Growing an arm of a hot interpreter
  `match` perturbs register allocation and placement for the whole loop and
  can slow workloads that never reach the arm. Put new work behind one
  out-of-line call from the arm, and measure the aggregate even when the
  change only adds a case.

## Correctness while optimizing

- **"An older build fails too" needs a bisect.** Reproducing a bug on an
  earlier build does not show it predates your change.
- **Make possible hangs fail.** A test for a loop or control-flow change
  carries an in-script guard counter that throws past a bound, so a hang
  fails fast instead of stalling CI.
- **A faster wrong answer is a regression.** A benchmark's stdout sentinel
  only proves it reached the end; rely on the workload's own validation and
  Test262 for correctness.
