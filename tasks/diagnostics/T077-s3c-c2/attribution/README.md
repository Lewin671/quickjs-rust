# Independent attribution of the S3c C2 regression (lead receipt, 2026-11-01)

Base: main `8322fc98`. Candidate: C2 `63b2347f`, on the retained branch
`agent/t077-s3c-activations/codex`. The evidence is deterministic local arm64 data:
- instructions retired from `/usr/bin/time -l`, using the N/2N slope method;
- coverage line counts from a build with LTO off and 16 codegen units;
- more than 30 measurement-only ablation builds.

Reproducers and per-call data are in this directory.

## Findings

1. **access-fannkuch is 5.29× slower because of a sticky typed-loop decline.**
   `vm_frame_init.rs:151-158` reuses a record without clearing
   `declined_typed_loop_programs`. S3b built a fresh Vm on every resume, so it
   retried and admitted the loop on attempt 30. Clearing the bit per entry returns
   fannkuch to 1.003×. Repro: `sticky-decline.js` (2.76×, 1.01× after the fix).
2. **C2 adds about 950 instructions per call.** upw0 goes from 1003 to 1941. The
   new record lifecycle costs about 1050 gross, minus the 112 instructions of S3b
   binding protocol that C2 deleted. About three quarters of it is implementation
   waste:

   | Cause | Instructions per call |
   | --- | --- |
   | Three segment entries per call, each re-deriving the callee Rc, strict and receiver; the native back-edge exit re-runs the segment prologue | ~250 |
   | Directory reserve/park/resume/retire, live and generation checks | ~174 |
   | Call prefix fully bound and cleared on every call | ~107 |
   | Whole-block clears instead of written-range clears | ~66 |
   | Release-build asserts and `ActiveFrame` `expect` calls | ~74 |
   | RefCell plus `Rc<Store>` traffic | ~34 |
   | Hot paths not inlined | ~52 |
   | Residual, from using the whole general FrameState as the inline record instead of a 64-byte header | ~300 |

   date-format-tofte also deep-copies the bytecode on every eval entry
   (`frame_program.rs:51-53`, `Rc::new(body.clone())`); repro `eval-root-clone.js`.
   With every semantics-preserving fix applied together, upw0 drops to 1316
   instructions per call, still well above S3b without its protocol (790).
3. **The architectural judgement** is based on the minimal unexposed inline call
   sequence, where a is the argument count and w the number of written registers:

   | Design | Instructions per call |
   | --- | --- |
   | Contiguous register stack: `Vec<Value>` plus `Vec<64-byte header>`, slice bound once per non-reentrant segment | ~45+3a+3w |
   | Boxed record, directory and pool | ~65–85+3a+3w |

   S3b's compact/wide drivers already used a contiguous window. Moving every call
   onto a boxed FrameState was a structural step backwards for the hot path.
   Recommendation:
   - put ordinary activations on a contiguous register stack;
   - give statically known suspendable functions (generators, async, TLA) boxed
     records from entry, and resume the same box;
   - keep only suspended or exposed records in the directory;
   - allocate FrameIds lazily.

   This converges directly with S3e's `values: Vec<Value>`.
