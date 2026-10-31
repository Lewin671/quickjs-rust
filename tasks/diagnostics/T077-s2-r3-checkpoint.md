# S2 round-3 redirect checkpoint

Lead intentionally stopped the preceding run. Runtime is coherent at a8e7709f;
HEAD 6e8f7d99 adds only the proposed typed-write follow-up. That follow-up was
not implemented and is deferred under the redirect. No partial runtime edit
remains. The Test262 submodule has only transient untracked harness files; its
tracked revision is unchanged and the corpus is left alone.

Public86 before order regeneration: job
`20261031T010923Z-t077-s2-r2-lazy-array-public86`, source a8e7709f versus
accepted main dcee3c56, five natural-layout pairs, automatic **likely loss**.
[Raw triage](T077-s2-r3-before-order.json): development .965461 wall/.945662
instructions; external 1.027942/1.009637; broad .995508/.983117; sentinel
.928547/.944279; all86 1.003860/.989626. Diagnostic only; no formal/held-out claim.
Earlier intermediate cohorts are preserved in [later public checkpoints](T077-s2-r2-later-public-checkpoints.md).

Verification bound to earlier sources: e0c16e96 full gate and 2,530 verified
runtime tests pass; its debug original/warmed fuzz cohorts have 300/50 seeds,
all three modes identical. a8e7709f has 376 verified array tests, strict feature
clippy and 268 NG fixtures with a debug IC-verified pin (four workers, 120-second
fixture limit). The default 10-second debug run timed out on the 200k-node
iterative-release fixture; the complete 120-second rerun passes. The a8e7709f
full gate was stopped during curated execution and cleanup; it is incomplete,
not a passing full gate. Correct-agent e0c16e96 cached/off exact scans were also
stopped before completion and have no burndown rows. Final scans will be fresh.
Qualified complete/invalid CLI-agent checkpoints are recorded in
[agent pin verification](T077-s2-r2-agent-pin-verification.md).

Next, in order: regenerate only the x86 hot-function order using the canonical
candidate and generator defaults (external/sentinel/broad, no development or
held-out), commit it, and repeat public86 versus dcee3c56. Only if external still
loses, classify exact-binary base/candidate profiles for the five worst public
external/broad cases before any structural fix. Then freeze, complete final
correctness/resource/public gates, report and stop for lead acceptance. No
origin push or merge into main; temporary iMac bench refs remain authorized.
