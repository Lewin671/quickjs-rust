# T010: Generators and Iteration Campaign

## Goal

Implement generator functions and round out the iteration protocol. At commit
`3e7feb0` (2026-06-09 full scan) `generators` tags 1,234 actionable gaps,
`Symbol.iterator` 482, and `iterator-helpers` 244, with
`test/built-ins/Iterator` contributing 358 gap cases.

## Evidence

- Generators are pure engine gaps (no structural not-run), so every slice is
  measurable with focused gap runs.
- Async generators are out of scope here; they belong to T007 S5, which
  depends on this campaign's resume machinery.

## Slices

- [x] S1 Parser: `function*` declarations/expressions, `yield` and `yield*`
      expressions with correct operator-precedence and strict-mode rules.
- [x] S2 Runtime: generator objects — suspend/resume state machine,
      `next`/`return`/`throw`, completion semantics. Reuse the suspension
      design that T007 S3 will need; coordinate if both are active.
      A generator object owns its body's resumable VM state
      (`bytecode::vm_generator::GeneratorState` on `ObjectRef`); `Op::Yield`
      exits the bytecode loop and resume re-enters it, delivering the resume
      value or an injected return/throw completion through the existing
      try/finally unwinder. Scope cuts deferred to later slices: `yield*`
      delegation (S3, still a structured early error) and the
      `%GeneratorFunction%` / `%GeneratorFunction.prototype%` intrinsic identity
      chain (the runtime cannot yet use a function as a `[[Prototype]]`, so
      `Object.getPrototypeOf(g).constructor` identity is a follow-up).
- [x] S3 Runtime: `yield*` delegation, including return/throw forwarding and
      iterator-close interaction. `yield*` compiles to `Op::YieldDelegate`,
      which runs the ES2023 14.4.14 loop in the VM: it resolves the inner
      iterator once, forwards `next`/`return`/`throw` resumes to it (a
      `delegating` marker on the suspension routes the resume through
      `Vm::resume_mode`), suspends the outer generator yielding each non-done
      inner result object unwrapped, closes a throw-less inner iterator before
      raising a TypeError, and turns a return-less inner `return` into an outer
      return completion (running outer `finally` blocks).
- [ ] S4 Iteration protocol cleanup: remaining `Symbol.iterator` gaps on
      built-in iterables and iterator-close paths beyond the for-of cases
      already landed.
- [ ] S5 Iterator helpers (`Iterator.prototype.map/filter/take/...`), batched
      by method family after S1-S3 land.

## Scope

- Allowed paths: `crates/qjs-ast/**`, `crates/qjs-lexer/**`,
  `crates/qjs-parser/**`, `crates/qjs-runtime/**`.
- Forbidden paths: `third_party/**`.
- Owner boundary: one slice per owner; S2 owns the suspension machinery and
  serializes with T007 S3.

## References

- `docs/architecture.md`
- QuickJS-NG: `third_party/quickjs-ng/quickjs.c` (generator opcodes and
  `js_generator_function_call`).
- Test262: `test/language/statements/generators/**`,
  `test/language/expressions/yield/**`, `test/built-ins/GeneratorPrototype/**`,
  `test/built-ins/Iterator/**`.

## Acceptance Criteria

- Generator state-machine behavior is covered by runtime unit tests
  (suspend/resume, abrupt completions, reentrancy errors), not only Test262.
- Each slice reduces the focused gap count for its subtree.
- Campaign exit: `generators` leaves the top feature clusters and T007 S5 is
  unblocked.

## Verification

```sh
cargo test -p qjs-parser
cargo test -p qjs-runtime
./scripts/find-qjsng-gaps.sh --filter test/built-ins/GeneratorPrototype --all
./scripts/find-qjsng-gaps.sh --filter test/built-ins/Iterator --all
./scripts/check.sh
```

## Notes

The suspension machinery is the architectural decision with the longest
shadow (it also serves async functions). If S2's design forces changes to
shared AST or runtime call structures, serialize that slice on one branch per
the parallel-workflow rules.
