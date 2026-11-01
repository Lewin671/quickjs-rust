# Final S3b owner requalification after the global-store repair

Frozen runtime **0148b2eb**, frozen sha/order **8abd4c69**, baseline **3cc12c6f**.
Round 4 lands S3b as an enabling stage, subject to the lead's acceptance. The
repair, its cost and the S3c obligations are in the
[progress record](../T077-s3b-progress.md#round-4-milestone-3-global-store-repair-and-final-requalification);
[metadata](metadata.json) pins every arm64 binary, the rebuilt 7b3ed258 reference
runtime, Test262/NG pins and rustc. No timing ran on this host.

## Correctness checks

- [Full gate](full-check.log) `./scripts/check.sh`: exit 0, **2,728 runtime tests**,
  all lint/tool/script/context stages, agents (21) and counter (24) feature
  stages, **5,174 curated Test262 cases**. Full feature suites: **2,739 agents**,
  **2,777 perf-counters** ([checks](checks.json)).
- Stress ([checks](checks.json)): **2,728** pass in each of verify-bindings,
  forced exposure, scope poison, activation poison, IC verification, combined
  IC/binding and combined-all (both poisons); combined IC-off **2,686** with the
  unchanged [42 cache-mechanism controls](skipped-ic-controls.txt) skipped.
- [Fixture matrix](compare-matrix.json): **319 scripts** pass in nine modes (default, IC-off,
  IC-verify, binding-verify, forced exposure, scope poison, activation poison,
  combined IC-off and combined-all) with the unchanged 10-second timeout on an
  unloaded host. All **eight module entries plus two dependency smokes** match NG
  exit/stdout in every mode, TZ=UTC; new: `global-store-inherited-setter.js`,
  `global-store-declarative-first.mjs`, `global-store-object-record.mjs`. (The
  stopped 689efca2 run timed out once in each combined mode while the differential
  loaded the host; that fixture takes 8.8 s there under debug IC-off.)
- [Public correctness](public/summary.json): **86 cases, 258 case-modes,
  351/351 invocations**, zero failed.
- [Seeded differential](differential.json), seeds 0–1999 per profile, 8,000
  programs / 48,000 engine invocations, stable inputs, zero panic/crash/timeout
  and zero disagreement between default, IC-off, IC-verify and stress modes.
  binding / binding-simple / receiver: all **6,000** tag-identical to the
  7b3ed258 qualification; its 742 triaged programs carry identical
  PRE/CHANGED/REGR observations and values; **no new REGR**. global-store (new):
  MATCH 527, FIXED 1,445, CHANGED 39, REGR 35, PRE 0; the 7b3ed258 runtime, rebuilt
  and run by the same runner on the same seeds, has REGR 68, CHANGED 419, PRE 38.
  134 of the candidate's 181 non-NG observations equal that runtime's; the 47
  others (14 programs, three of them REGR) and every inspected shared one follow
  the unresolvable-at-start rule: a reference unresolvable before its RHS stays
  so (PutValue: strict ReferenceError; sloppy sets the global object, even
  beside a lexical the RHS declared), where NG and V8 re-resolve, plus values
  repaired by this round. No candidate-only defect remains.
- Exact Test262 `--exact --all` on the agents arm64 binary: cached and IC-off each exit **0** with
  **42,672/42,672 Rust passes**, zero fail/timeout/not-run/gap; NG **42,602 /
  70 failures**; unchanged **10,900** configuration skips over **53,572** rows.
  Both case maps are byte-identical to each other and to the prior complete map
  (SHA-256 `2263183d…`). Burndown rows: **0148b2eb** cached, **8abd4c69** IC-off
  (same arm64 build inputs) ([cached](exact-cached/metadata.json),
  [IC-off](exact-off/metadata.json)).
- [Depth](depth-parity.json): release maxima (base → candidate) forEach 796→936,
  map 790→927, reduce 829→1,026, call 3,234→3,449, apply 2,066→2,335,
  Reflect.apply 2,188→2,492, yield* 714→820, plain callback ≥4,096→≥4,096.
  **Debug fails three baseline maxima**: call 461→437, apply 372→366,
  Reflect.apply 373→367 (forEach 177→184, map 176→182, reduce 225→236, yield*
  146→152 pass). Bisect: since f609c0ae (milestone 1); the debug frame of
  `compact_fn::wide::activation::run` grew 13,776→15,376 bytes. Not changed: no
  semantic difference, release maxima are higher, the source is frozen; it awaits
  the lead's decision ([candidate traces](candidate-depth.json)).
- [Counter maps](candidate-counts.json): all ten pinned development programs
  reproduce stdout and full maps twice; base maps are identical to the
  [prior base record](../T077-s3b-final-owner/base-counts.json). CallEnv
  constructions/clones, caller-env proofs, binding snapshots, frame forks and
  apply-back calls stay **0** (base 19,031,946 / 72,543 / 26,477,573 / 72,101 /
  100,578 / 5,082,372); new costs unchanged: 457,653 cells, 328,929 promotions,
  5,368 leases, 1,388 lexical owners, 2,137,508 local-vector requests, 673 new
  vectors; typed variable identities 3,027 (minted on first exposure).
- [x86 builds](x86-builds.json): default, agents and perf-counters cross-builds
  with the new order each pass an executable smoke.

## Public86

[Job `20261101T140707Z-t077-s3b-final3`](../T077-s3b-final3-public86/README.md):
430/430 valid, budget met, likely loss; wall/instructions broad 1.017/1.033,
development 1.034/1.019, external 1.026/1.030, sentinel 0.991/1.032, all 86
1.021/1.030.

Lead owns formal, held-out, RSS and the stage decision. No origin push or main
merge.
