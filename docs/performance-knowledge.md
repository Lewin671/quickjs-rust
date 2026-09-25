# Durable performance knowledge

Rules for measuring and reasoning about engine performance that stay true
after the engine's architecture changes. Read this before any performance
unit.

Admission rule: an entry belongs here only if it would still hold after a
major refactor. Current ratios, rankings, per-operation costs, profile
conclusions and falsified mechanisms are state, not knowledge. They live in
the evidence queue, the unit plans under `tasks/performance-units/`, and the
task files, each bound to a base revision. Entries carry no measured numbers
and no commit hashes.

The procedure (queue, plan, decision) is in
[performance-workflow.md](performance-workflow.md); tool mechanics are in
[benchmarking.md](benchmarking.md).

## Measuring

- **Judge CPU work by cycles; watch wall time for everything else.** On a
  shared machine, background load moves wall time far more than it moves
  cycles or instructions retired. Cycles still include cache misses and
  pipeline stalls, so they measure what the engine costs on the CPU; they
  exclude time off-CPU (I/O, syscalls blocked in the kernel, waiting,
  descheduling) and frequency changes. Use cycles to judge single-threaded,
  CPU-bound changes and keep wall time beside them: a wall-time regression
  that cycles do not show is a real cost the counters cannot see.
- **Measure the noise floor, do not assume it.** Before trusting a
  comparison, compare a binary against a byte-identical copy of itself on the
  same host and harness. A difference smaller than that A/A spread
  demonstrates nothing. The floor depends on the machine and the corpus, so
  re-measure it rather than reusing an old value.
- **Judge on the aggregate first.** A geometric mean over many cases
  resolves much smaller effects than any single case. Attribute a single-case
  change only when it exceeds that case's own A/A spread.
- **Compare like with like.** Candidate and base run through the same
  harness, from the same directory, with the same build recipe, interleaved
  on one host. The same script has measured differently from different
  filesystem locations; never compare numbers taken along different paths.
- **Keep samples long enough.** Short runs are dominated by startup, timer
  resolution and scheduling. Raise iterations until each sample clears the
  harness minimum instead of adding repetitions of a too-short sample.
- **Measure one operation by increment, not by subtraction from an empty
  body.** An empty or trivial loop body often runs on a different execution
  tier, so subtracting it measures a tier change. Time N and 2N operations
  in the same body shape and take the difference, after confirming both run
  on the same tier.
- **Serialize timing.** Concurrent builds, test runs or other agents on the
  same machine corrupt every timing and can turn slow cases into spurious
  timeouts. Parallel agents may write code in parallel; one coordinator runs
  measurements one at a time.
- **Diagnostic builds never time.** A `perf-counters` build adds work to the
  paths it observes; use it to count and trace, not to measure speed.

## Knowing what a benchmark exercises

- **Verify that a control executes the path it guards.** A loop the
  compiler folds or specializes whole performs almost none of its nominal
  operations. Before using a case as a regression control, confirm with
  execution counters or the typed-loop trace that it really runs the generic
  path in question.
- **Find the blocker before changing code.** Use the trace histograms and a
  profile of the exact binary to name the instruction, shape or call that
  keeps work on the slow path. A cost that is only suspected is not a reason
  to optimize.
- **Inclusive profile fractions overlap.** Stack samples attributed to a
  caller include its callees; never add inclusive fractions together as
  independently removable costs.

## Codegen

- **Unrelated edits change inlining and layout.** Adding code elsewhere in a
  crate can make the compiler stop inlining or unrolling a hot function. When
  a case regresses without an explanation, diff symbol sizes between the two
  builds (`nm -S` / `nm -n`) before blaming the change's own logic, and pin
  load-bearing hot helpers with `#[inline(always)]` or `#[inline(never)]`.
- **Pin the hot layout.** Where a function lands decides which cache sets
  and branch-predictor entries its loops share, so an edit that only moves
  hot code can change a case's cycles with identical instruction counts.
  The linker places the functions listed in `crates/qjs-cli/hot-functions.order`
  first, in that order. A change that adds hot functions, or that makes the
  profile's hottest set differ, regenerates the list in the same commit;
  until then the new functions sit outside the pinned region and their
  measurements carry layout noise.
- **A stale order file hides as a regression several commits later.** New
  hot helpers (an out-of-line read path, a memo check) land after the
  pinned region, and any later edit anywhere in the crate shifts them: on
  2026-09-24 a 32-byte size change in an unrelated string function moved
  them and cost the call sentinels 15-22% and ai-astar 18% with identical
  instruction counts, invisible to per-commit A/B runs whose base had the
  same stale file. Canaries: `capturing_closure_call` and ai-astar cycles
  against the last formal base. Regenerating the file restored both
  exactly; do it in the commit that adds a hot function, and before any
  formal run.
- **The typed-loop executor's own address is the big layout lever, so it
  is pinned by address.** Measured 2026-09-24 with byte-identical code:
  `capturing_closure_call` and ai-astar run 18-25% more cycles unless
  `try_run_typed_loop<WideLoopFrame>` starts in a narrow window modulo
  4 KiB (0xf80..0xfe0; 0xd90..0xdc0 after one added operation), with its
  callees in a fixed order right after it. Stack placement (env
  size), heap placement (JS-level allocations, malloc settings), jump-table
  offsets and loop alignment all measured flat; shifting only the executor
  flipped the state every time. An order file alone could not hold it: the
  functions listed before the executor (VM, wide driver) change size with
  most edits, and codegen-unit changes resize even untouched ones.
  `python3 -m tools.benchmark.layout_pin --binary <qjs>` puts standard-library
  functions of fixed size first, sized so the executor lands at the pinned
  offset, then the executor and its callees; `order_file` applies it after
  every regeneration. The executor is the dispatch loop, `run<WideLoopFrame>`,
  which the compiler has kept out of line since 2026-09-25 -- the tool had
  been pinning its 132-instruction caller while the loop itself floated in
  the unordered tail; it now looks the loop up in the binary first. After editing the executor itself, re-scan the offset
  (`--offset`, one relink and the two canaries per probe, ~15 s each) and
  keep the centre of the fast window. The executor is generic and is
  instantiated in its caller's codegen unit: moving `run_typed_loop_here`
  into another module recompiled it (48 more instructions) and lost the fast
  state at the same address, so keep that caller in `wide/activation.rs`.
  The interpreter's instantiation, `run<Vm>` (a script's top-level loops:
  74% of access-fannkuch, 48% of math-partial-sums), is pinned too, after
  its own filler (`--vm-offset`, 0xc40): unpinned it moved 0xc40 -> 0x870
  with an unrelated edit and partial-sums ran 4.7% more cycles on fewer
  instructions. Its page scan at 0x100 steps was flat within 2%. A name
  defined once per codegen unit (`Value::clone`, `drop_in_place<Value>`)
  is placed once per copy -- count every copy when sizing a gap.
- **Know each case's codegen noise band before blaming a change.** The
  functions that hold a dispatch loop are re-compiled differently by edits
  anywhere in the crate (an inlined thread-local access, a helper's inline
  decision), which moves call-heavy cases by several percent with identical
  execution counts. A base that happens to be a good roll makes every
  later change look like a regression on those cases. Measure the band with
  a semantically neutral rebuild of the base (a reachable-but-never-run edit,
  an out-of-lined helper) and treat a control regression inside it, with
  every other hot function byte-identical, as codegen noise to be judged on
  the aggregate.
- **Hand a dispatch loop the storage, not its owner.** Reaching a register
  file through `&mut Owner { file: Box<[T; N]> }` instead of `&mut [T; N]`
  added a pointer load the loop could not keep in a register: the typed
  loop's fixed register file ran 3-5% fewer instructions and 6-18% more
  cycles (ai-astar, the sentinels) until `execute` borrowed the array
  itself. When instructions fall and cycles rise, suspect an extra
  indirection before alignment.
- **Keep hot dispatch arms tiny.** Growing an arm of a hot interpreter
  `match` can degrade register allocation and layout for the whole loop, and
  slow workloads that never reach the new arm. Put new work behind a single
  out-of-line call from the arm, and measure the aggregate even when the
  change only adds a case.

## Correctness while optimizing

- **"An older build fails too" needs a bisect.** Reproducing a bug on some
  earlier build does not show it predates your change; bisect to the
  introducing commit.
- **Make possible hangs fail.** A test for a loop or control-flow change
  carries an in-script guard counter that throws past a bound, so a hang
  fails fast instead of stalling CI.
- **A faster wrong answer is a regression.** A benchmark's stdout sentinel
  only proves it reached the end; rely on the workload's own validation and
  Test262 for correctness.
