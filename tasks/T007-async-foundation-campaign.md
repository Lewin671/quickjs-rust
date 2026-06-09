# T007: Async Foundation Campaign

## Goal

Build the asynchrony foundation — a job queue, async functions, and the
Test262 async harness channel — so the 5,290 QuickJS-NG-passing async-flagged
cases (commit `3e7feb0`, 2026-06-09 full scan) stop being structurally
not-run, plus the 277 async-iteration actionable gaps.

## Evidence

- `comparison.ng_pass_rust_not_run` async bucket: 5,290 cases. The baseline
  skips any test with the `async` flag (`skip_reason` in
  `scripts/test262-baseline.sh`).
- Actionable gaps tagged `async-iteration`: 277.
- Async Test262 cases report completion through `doneprintHandle.js`
  (`$DONE`), which requires print output and a drained job queue.

## Slices

- [ ] S1 Runtime: promise job queue — enqueue promise reactions as jobs,
      drain the queue after script evaluation. Existing Promise surface keeps
      its semantics; ordering tests at the runtime layer.
- [ ] S2 Parser: `async function` declarations/expressions and `await`
      expressions, including arrow forms. Parser-only; spans and focused
      tests.
- [ ] S3 Runtime: evaluate async functions — suspend/resume on `await`,
      return promises, propagate rejections.
- [ ] S4 Harness: support the async test channel — run `doneprintHandle.js`
      includes, treat `$DONE`-reported success/failure as the case result,
      drain jobs before judging. Narrow the async skip in
      `scripts/test262-baseline.sh` to cases the harness still cannot judge,
      and record a fresh burndown entry.
- [ ] S5 Parser + runtime: `for await ... of` and async generators. Depends
      on T010 generator slices; re-cluster remaining async gaps first.

## Scope

- Allowed paths: `crates/qjs-ast/**`, `crates/qjs-lexer/**`,
  `crates/qjs-parser/**`, `crates/qjs-runtime/**`; S4 also
  `scripts/test262-baseline.sh`, `tests/test262/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; S1 must integrate before S3.
- No threads or host async runtime: the job queue is a deterministic
  single-threaded drain loop, consistent with the no-async/no-threads
  engineering standard in `AGENTS.md` (this task explicitly authorizes the
  language-level feature, not host concurrency).

## References

- `docs/architecture.md`
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` (`JS_ExecutePendingJob`,
  async function state machines).
- Test262: `harness/doneprintHandle.js`, `test/built-ins/Promise/**`,
  `test/language/expressions/await/**`,
  `test/language/statements/async-function/**`.

## Acceptance Criteria

- Job-queue ordering matches QuickJS-NG on comparison fixtures under
  `tests/fixtures/compare-qjs/`.
- After S4, async-flagged cases appear as pass/fail signal in
  `./scripts/find-qjsng-gaps.sh` output instead of not-run.
- Campaign exit: the async not-run bucket in the burndown series drops to
  cases blocked on async generators only.

## Verification

```sh
cargo test -p qjs-runtime
./scripts/compare-qjs.sh
./scripts/find-qjsng-gaps.sh --filter test/language/statements/async-function --all
./scripts/check.sh
```

## Notes

S1 is also a prerequisite for unhandled-rejection semantics and dynamic
import (696 module not-run cases stay out of scope here). Keep `$DONE`
handling inside the baseline harness layer; engine crates must not know about
Test262 conventions.
