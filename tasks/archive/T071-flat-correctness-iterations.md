# T071: Flat-repair correctness iterations

- Status: historical-campaign; pre-triage iterations, not performance candidates.
- Implementation: intermediate changes retained in branch history; map-fork experiment removed in `b428ebeb`, typed call-cell refinement present in `a7581aa9`.
- Verified at: individual revisions and checks named below, 2026-10-23; final exact-source checks live in the active task.
- Evidence: reproductions and queue identifiers below; [active T071](../T071-lexical-context-review.md).
- Unresolved: general direct-eval scope model belongs to [T074](../T074-direct-eval-scope-model.md).
- Next action: follow the active T071 record; no timing evidence is established here.

## Correctness iterations before timing

`e9572f4d` passes fast/full (2,407 runtime, 117 focused/5,170 curated Test262)
but NG queue `20261023T185225Z-t071-flat-reference` exits 1 on the existing
combined arguments-context fixture: Rust ends `8:undefined:method`, NG ends
`8:function:changed`. Isolated named-function cases pass; naming is not the
cause. An inherited dynamic arguments entry wins over the promoted current
call cell when initializing eval code; later eval can also fork away that
cell. Repair overlays the actual cell in the already-forked eval view and
hands it to the declaring frame's rare dynamic cell-map fork. The whole
original fixture is now a runtime regression, with expected NG output
`undefined:undefined:undefined:8:function:changed`. All 12 arguments-context
tests pass. No failed-source performance triage was started.

Queue `20261023T190014Z-t071-flat-final-reference` at `6599deee` passes all
261 fixtures and the two-line combined module comparison. A further focused
probe finds late synthetic arguments in an already-emitted try cleanup list
when the key itself does not read arguments: reading arguments after a caught
throw fails. Finalization now removes that exact function binding from cleanup
in the same linear operand walk. The existing catch regression and NG fixture
also read arguments in the declaring activation after catch. No baseline VM
cleanup code changes. Intermediate area submission `20261023T190040Z-t071-flat-areas`
exits 2 before testing (`--report` is unsupported); corrected to `--out-dir`.

The `6599deee`/`d1a56f34` cell-map fork is rejected: replacing the review's
returned arrow with an ordinary function, and using actual-key eval in inner,
freezes a later outer `eval('var b=5')` (`ReferenceError`, expected 5). Final
forwarding uses only the actual cell and metadata alias; no caller map fork.
New regressions cover later var/function additions and direct source assignment
to arguments, with a same-named global property protected. A pre-existing
same-frame `const arguments='m'; eval("arguments='bad'; class C { #p; [(#p in {}, arguments)]() {} }")`
probe returns `bad` on exact base, dd and intermediate candidate examples;
this remains T074's same-frame const gap, not a newly asserted wrong test.
Actual-key unresolved stores still check known immutable/import/TDZ authority.

Final ordinary-path refinement planned before coding: b428's arm64 paired
phase recheck has compile ratios 1.006747/1.001972 and execution ratios
1.001399/1.002218 (immutable/graphlib). It is not the iMac timing target.
Remove the new per-dynamic-view alias lookup: represent the actual-key cell's
retained identity in the existing frame-binding value enum. Direct values and
ordinary cells keep their existing routes; only the actual-key call-context
cell variant survives a value fork. No new CallEnv/Bytecode/frame-plan field.
