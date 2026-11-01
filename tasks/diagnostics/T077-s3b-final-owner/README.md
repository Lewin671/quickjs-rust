# Final S3b owner qualification

Frozen runtime **e1405670**, candidate/order **8413f925**, baseline **3cc12c6f**.
The runtime is present, but S3b **misses its payoff requirement**. Healthy
[public86](../T077-s3b-final-public86/README.md) reports a loss in every lane;
there is no stage acceptance. Lead owns formal/held-out/RSS, re-review and the
stage's disposition. No tuning or measurement retry follows this result.

## Source and repair

Realm removal invalidates every retained ObjectStorage route before dropping its
mapping; invalidated captures select the current global record. Deletion,
recreation, accessor redefinition and async resumption match NG/main in all three
modes. [Repair evidence](../T077-s3b-round3-fuzz/README.md),
[final seeded differential](../T077-s3b-final-differential/README.md).

No runtime change follows e1405670. The corrected receiver generator and fixture
comment repair change no engine build inputs. The only fuzz-to-final build-input
change is the regenerated x86 order, ignored by ARM's target-specific build script.
[Metadata](metadata.json) pins the same release/verifier binaries as differential
qualification. [Order job](../T077-s3b-final-order/README.md) profiles the default
76 public inputs into 527 symbols. [x86 builds](x86-builds.json) cover default,
agents and perf-counters, each with a passing executable smoke; ARM builds also
cover all three, plus the assertion/overflow-enabled optimized verifier.

## Main changed files in this finish

- `crates/qjs-runtime/src/bytecode/vm_scopes.rs`: invalidated captures defer to current global resolution.
- `crates/qjs-runtime/src/context/realm.rs`: invalidate retained object routes at the common removal boundary.
- `crates/qjs-runtime/src/tests/direct_eval/self_binding.rs`: deletion, recreation, accessor and async regression tests.
- `tools/binding_fuzz/generate_this.py`: distinct nested loop counters and matching index expressions.
- `tests/fixtures/compare-qjs/invalidated-global-self-capture.js` and `tests/fixtures/compare-modules/invalidated-global-capture-jobs.mjs`: permanent reference comparisons.
- `tests/fixtures/compare-qjs/s3b-property-equality-receiver-resume.js`: remove line comments that break newline flattening.
- `crates/qjs-cli/hot-functions-x86_64.order`: regenerated target order.
- T077 resume/index/progress, differential runbook, complete burndown and linked qualification receipts: current standing and evidence.

## Correctness checks

- [Full gate](full-check.log): exit 0, including **2,695 default runtime tests**,
  all lint/tool/script/context checks and **5,174 curated Test262 cases**.
  Additional full suites pass **2,706 agents tests** and **2,739 perf-counter tests**;
  check.sh's own feature stages also run 21 Atomics and 24 counter-specific tests.
- [Stress receipts](checks.json): **2,695** tests pass in each binding verification,
  forced exposure, poison, IC verification and combined IC/binding mode.
  [Combined IC-off](combined-off.json): **2,653** pass; its unchanged explicit
  42 cache-enabled mechanism controls run in the complete IC-on suites. No
  repository test or Test262 exclusion is removed.
- [Fixture matrix](compare-matrix.json): **318 scripts** pass in seven modes
  (default/off/IC-verify/binding-verify/force/poison/combined-off). All **six module
  entries plus two dependency-module smokes** match NG exit/stdout in every mode,
  including jobs, TZ=UTC. The [initial default attempt](fixture-transport-error.json)
  found line comments swallowed closing braces when the script flattened newlines; deleting those stale
  comments repairs the fixture transport. Runtime/binaries remain unchanged.
- [Public correctness](public/summary.json): **86 cases, 258 case-modes,
  351/351 invocations**, zero failed; complete stdout/checksums and deadlines
  unchanged. No held-out input is resolved or run.
- [Depth parity](depth-parity.json): every baseline-success maximum and prior
  successful landmark passes in both debug and release, on ordinary CLI stacks.
  [Candidate](candidate-depth.json) / [base](base-depth.json) preserve full traces.
  Release maxima (base → candidate): forEach **796→920**, map **790→912**, reduce
  **829→963**, call **3234→3801**, apply **2066→2367**, Reflect.apply **2188→2528**,
  yield* **714→795**, plain callback recursion **>=4096→>=4096** (probe cap).

## Counter/deletion audit

[Candidate maps](candidate-counts.json) and [base maps](base-counts.json) reproduce
all ten pinned development sources/stdout/full maps twice. The
[base source proof](base-instrumentation-proof.json) verifies only the existing
six-file diagnostic instrumentation delta against 3cc12c6f. Counts are events,
not malloc savings; diagnostic binaries are never timed.

| Event | Base | Candidate |
| --- | ---: | ---: |
| CallEnv constructions | 19,031,946 | 0 |
| CallEnv clones | 72,543 | 0 |
| Caller-env equality proofs | 26,477,573 | 0 |
| Binding snapshots | 72,101 | 0 |
| Frame forks | 100,578 | 0 |
| Apply-back calls | 5,082,372 | 0 |

[Totals](counter-totals.json) keep new costs separate: **457,653** cell allocations,
**328,929** promotions, **5,368** ScopeLease allocations, **1,388** lexical-owner
allocations, **2,137,508** local-vector pool requests and **673** new local vectors.
Typed variable identities **29,298,771** equal **29,298,761** ordinary entries
plus ten host roots; **199,691** forwardings remain distinct.
S3a argument-vector pop/native family probe/decline events remain zero on both
sides. These fresh full maps include the round-3 semantic repairs; earlier maps
remain revision-bound history.
[Deletion audit](deletion-audit.json) finds no retired execution API, preserves
all 592 native entry arms/source, dependencies, pins, layout and property-IC
modules against main. No new dependency, unsafe or name-keyed execution snapshot.

## Public86 payoff: failed

Job **20261101T043615Z-t077-s3b-final-public86**, five pairs, workers-4, natural
layout, **430/430 valid attempts**, zero invalid/heavy samples, budget met.

The [public86 record](../T077-s3b-final-public86/README.md) owns every lane's wall
and instruction ratio and the frozen case-level result.

Every lane regresses in both metrics. Original verdict: **likely loss**. Full
[request/result/binary/policy receipts](../T077-s3b-final-public86/README.md) are
retained. Zero old environment events do not establish a speedup. Formal,
held-out and RSS acceptance are not run by this owner.

## Complete exact cached and IC-off

Both complete configured `--exact --all` scans exit **0** on the same frozen
agents ARM binary: **42,672/42,672 Rust passes**, zero fail/timeout/not-run/gap
in each mode. NG has **42,602 passes / 70 failures**; unchanged **10,900**
configuration skips across **53,572** rows. No new exclusion or lost pass.

The [complete case maps](exact-parity.json) are byte-identical to each other and
to the prior complete accepted-S2 map, SHA-256
`2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.
[Cached summary](exact-cached/summary.json) / [metadata](exact-cached/metadata.json)
and [IC-off summary](exact-off/summary.json) / [metadata](exact-off/metadata.json)
retain the actual commands, pins, binary and raw result hashes. Complete
burndown records are appended at **e1405670** (cached) and **7ce2ab30** (off);
these revisions share all runtime/harness/configuration inputs. The final x86
order changes no ARM build input, and the hash-pinned ARM binaries are unchanged.
No partial scan enters the ledger; earlier rejected/historical rows are preserved.

## Remaining limitations and next action

Owner qualification is finished; **S3b is not accepted** because the public payoff
requirement fails in every lane. No tuning or measurement retry follows in this
owner finish. Lead owns formal/held-out/RSS, re-review and disposition before
any dependent stage advances. The raw differential runner's conservative exit 1
and all reviewed reference differences are retained in its linked receipt.
Inherited CLI error-prefix spelling remains at binding-simple/170 and strict-delete
PRE cases; no unexplained candidate-only semantic difference remains. Existing
S3e/S4, class-construction and native-stack-policy ownership is unchanged.

The owner stops after committing this evidence. No origin push or merge into main.
