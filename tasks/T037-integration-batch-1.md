# T037: Integration batch 1

- Status: active. Both batch-1 promotion decisions retained after the
  lead's 2026-10-12 unstable-base amendment; original measurements unchanged.
- Implementation: present in main; both units retained.
  No runtime code was reverted and no frozen plan/threshold was edited.
- Verified at: candidate af4d0d3c; rule merge dbf98c55, 2026-10-12,
  30-block base A/A, both promotion re-decides and the full integration gate.
- Evidence: [T018 standing](T018-broad-performance.md#standing-at-af4d0d3c),
  re-decision below and [leaf screens](archive/T018-batch-1-leaf-screens.md).
- Unresolved: audio-dft placement/variance still needs an engine unit;
  retained control uses instructions, not a claim of precise audio cycles.
- Next action: audio-dft investigation remains with T038. New x86_64 unit
  selection follows [T040](T040-imac-general-performance.md); the original
  af4d0d3c queue and decisions are not relabelled as current-main evidence.


## Scope and integration

Base: `e54e33eb9371c5cb4627bd8e590a648085a7fa26`.
Worktree: `../quickjs-rust-worktrees/wt-quickjs-rust-integration-batch-1`.
The main checkout's typed_loop diff was left unchanged; its SHA-256 before
and after is `a9f3e61d1ebfa859912749623bc04d3d249f5dafd7a3fad727cb42fd93d43783`.

| Source tip | Merge | Scope check | Source CI |
| --- | --- | --- | --- |
| class fields `7c15dde5` | `bd592be6` | 7 files, reported paths, PASS | 38220390353 success |
| global sinks `92ee2968` | `2382d9dc` | 16 files, reported paths, PASS | 38237290482 success |
| mixed-call docs `6a7bf946` | `7095a4aa` | 7 docs/tasks files, PASS | 38233484486 success |

Only the three docs/tasks commits of the mixed-call branch were integrated;
no temporary diagnostic runtime changes exist in its diff. Runtime conflicts
were absent. The order-file conflict provisionally kept the global-sinks pin;
`20c3cd22` independently re-pins the merged tree, stable after two relinks.
Frozen plan JSON files were not edited. The live global plan is the repin plan.

## Review findings

No runtime correctness blocker or benchmark-shape recognizer was found.
Global admission uses operations, binding/property/canonical-cell identities;
unknown observers disable deferral, completed writes publish before continuation,
and call-time callee guards reject captureful replacements. Protected slot/cell
ownership remains intact; no name-keyed per-call snapshot was introduced.
Class batching accepts resolved string keys and primitive constant-return
initializers only, ends before observers/private/symbol elements, and checks
receiver/storage/descriptors before mutation. Duplicate keys keep insertion
positions; indexed-key counts and both revisions update. Proxy, Array,
TypedArray, lazy String-index and failing descriptor paths retain sequential
exceptions and prior definitions. Global mirroring is synchronized before any
observer. The conservative Small-storage/all-configurable boundary can miss
opportunities but does not weaken semantics. The quadratic declined-segment
scan was fixed in `cd32549b`; the 512-field regression proves 512 inspections.
I would not approve the pre-fix scan or interpreting either leaf screen as
promotion. Current runtime code has no identified blocker; placement was accepted by the lead (instructions must not grow). No dependency or public API was added.

## Placement check

| Symbol | Queue base mod 4 KiB | Merged mod 4 KiB |
| --- | ---: | ---: |
| VM executor | 0xc40 | 0xc40 |
| Wide executor | 0x200 | 0x400 |
| call_leaf | 0x580 | 0x580 |

At `20c3cd22`, 15 AB/BA pairs/case, fixed sentinel N of 3844778,
8388608 and 3844778; external cases are whole processes. Equal-length binary
paths, warmup of both executables/case, no quiet-exec wrapper, RUSAGE_INFO_V6
cycles/instructions. The shared lock covered each complete run; no competing
build/engine process was present at the start or before/after any pair.
P-core user-time shares were >=99.999% on the primary run. Medians below:

| Case | Cycles candidate/base | Instructions candidate/base |
| --- | ---: | ---: |
| `external/kraken-1.1/ai-astar` | 1.003438 | 0.999962 |
| `sentinel/prototype_method_call` | 1.016025 | 0.997522 |
| `sentinel/capturing_closure_call` | 1.003885 | 0.999980 |
| `sentinel/heterogeneous_property_read` | 1.006699 | 0.999996 |
| `external/sunspider-1.0/bitops-bitwise-and` | 0.116519 | 0.078169 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.945835 | 0.929610 |

All four cycle medians and their 95% upper bounds are below 1.03. The three
non-prototype instruction intervals include 1.0. Prototype's instruction
CI is [0.997487, 0.997567]; the lead clarified that instructions must not
grow and accepted this cheaper control within the 1.03 cycle ceiling. The target
raytrace cycle CI [0.940251, 0.952860] also does not prove the formal 0.95 ceiling;
this check is diagnostic, not a decision.

Order isolation: retain final pin head/use baseline ranking tail, then use
baseline pin head re-derived on merged code/use final tail. Both hold the
requested executor/call_leaf offsets. Each variant received 15 AB/BA pairs
on all six cases, not selective reruns; neither removes instruction drift.

| Prototype order variant | Cycles/base | Instructions/base |
| --- | ---: | ---: |
| `head-final-tail-base` | 1.012752 | 0.997476 |
| `head-base-tail-final` | 1.020169 | 0.997509 |

The original final order/binary was restored byte-identically after isolation.
The wide executor and call_leaf sizes differ from base (12144->12656 and
2784->2880 bytes); the other measured pinned callees have equal sizes. This
bounds the investigation; it does not attribute the instruction delta to a
specific operation or claim the codegen band is the whole cause.
Follow-up attribution at `af4d0d3c`: all execution counters at N=1000/2000 and final typed-op traces match base and class-only (at N=2000: 1999 closed-form calls, 1 typed entry, 192 interpreter ops, 1167 compact ops), class-only hot executor/call_leaf instructions match base modulo relocation, and release N=1m/2m increments (five samples) are base 1196.350, class-only 1196.201, merged 1193.253 instructions/iteration, locating the ~3-instruction shortening to global-sink shared executor/call_leaf codegen without a tier/path switch; individual ARM64 instruction attribution remains incomplete.

The durable rule lives in [performance knowledge](../docs/performance-knowledge.md#codegen).

Primary binary SHA-256: base
`6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4`, candidate
`6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b`.
Local commands: `lockf /tmp/qjs-lead/host-timing.lock env PYTHONPATH=.
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/qjs-lead/batch1-placement.py merged`;
`lockf /tmp/qjs-lead/host-timing.lock bash /tmp/qjs-lead/batch1-bisect.sh`.
Raw records include argv, counts, pair order, hashes and bootstrap intervals.

## Gates and separate script fix

All three merges passed staged gates, check.sh and 220 QuickJS-NG fixtures.
Runtime tests: class merge 2236; global and docs merges 2246 each. Every full
gate passed 5170 Test262 cases, 23 perf-counter diagnostics, 21 agents tests,
375 benchmark-tool tests and 35 script tests. Focused slices: class 52,
global 65; docs-only merge selects none. Re-pin staged gate passed 13 CLI tests.
An initial fast gate raced local submodule checkout (`error: Test262 allowlist
validation failed`); populated checkouts and a complete rerun resolved it.

Separate branch `agent/check-plan-validation/codex`, one commit `578a8f6b`:
explicit return propagates each plan's error from the conditional stage;
Bash glob enumeration avoids a failed rg process substitution. Three tests
prove first failure exits 17/stops, last failure exits 17, all-valid exits 0;
they run with PATH=/usr/bin:/bin. Before fix the first failure returned 0.
Initial CI 38238700110 exposed `scripts/check.sh: line 63: rg: command not found`
and still printed `check: ok performance unit plans`; its new tests failed
with FileNotFoundError. The corrected one-commit branch passed local full gate
(2218 runtime, 5170 Test262, 375 benchmark-tool, 38 script tests), scope check
and CI 38239774599. It is pushed and intentionally separate from this batch.
All 87 distinct frozen plans at current e54e33eb-descended local branch tips
were checked individually; none fail. Base alone has 82; this batch has 85.
No frozen plans were changed. core.bare remains false; user identity unchanged.
Remaining conditional-stage helper error propagation is a separate audit;
no claim is made that this one fix audits every function in check.sh.

## Re-decision of the af4d0d3c bundle

The original [first decision](archive/T037-first-formal-decision.md) remains
recorded as inconclusive. The lead's 2026-10-12 amendment is owned by
[T022](T022-performance-priority-controller.md#unstable-frozen-base-cases-lead-amendment-2026-10-12).
Rule commit e3df4072, scope 11 approved files, CI 38268268435 success; merge
dbf98c55. Full gate after merge passes: 385 tool tests, 35 script tests,
2246 runtime tests and 5170 Test262 subset cases. No runtime diff was merged.

The 30-block audio-dft base/base A/A uses the exact archived launch protocol,
under the host lock after compiler/engine waits. Cleaned temporary paths were
restored from the unchanged queue executable and hash-verified corpus:
binary 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4,
workload 57fcc07e83044e5552d83e4bf13a1e10af6e71357dddf28ea3c64de8dbd0bbda.
No sample was taken before restoration, and no sealed artifact was modified.

A/A evidence: target/unstable-base/audio-dft-aa-30.json;
SHA-256 5f7cfa9d0b87d096b66d45e7ba80f73f60fd8e986458e088ed3a6f273697a559.
Cycles ratio .999669835, CI [.959332829,1.034034315], half-width 4.20469%.
A/A instruction median .999961472, CI [.999896838,1.000146081]; full paired
instruction floor [.999355445,1.000644971]. Candidate/base instructions
.999663133, CI [.999532817,.999728652], wholly inside that floor.

Both promotion decides replayed the EXISTING bundle (SHA-256
90fe2a3ad29ef7d035195f8f36c3dc3e12d9706f0ae9f5b9a91ef3c174af9bf7),
original e54e33eb queue/profiles and exact af4d0d3c complete CI burndown.
No new formal run or queue was generated. Schema 4 classifies audio-dft as
unstable_on_base, with its instruction gate passing. Precision readiness is
ready_for_unit_gates: 75 timing cases plus that separately reported control;
no declared target is excluded or substituted. All original target gates pass.
Both --require-retained calls exit 0, with exact outcome fields:

```json
{"decision":"retained","metric":"cycles","reasons":[],"wall_time_divergence":[]}
```

| Frozen unit | Promotion outcome | Decision JSON SHA-256 |
| --- | --- | --- |
| global-numeric-binding-sinks-repin | retained | 827f6520ac92e39599d879378c8dfe7ab4c0bcc9535f795bab35ac14f07c4d2a |
| class-field-batch-install | retained | 3aaf17e1e36f2982a3da9e073ab761666307560d41446cb94d714d976caa5161 |

Files are target/unstable-base/<unit>-decision.json and <unit>-stdout.txt;
commands are preserved in <unit>-command.json. Exact stdout hashes, in row
order: 018e1d61608c3b289e891b60c19ea876ab7cef530b447b02e666da281667fc54,
fa5c0391da3d71c93ccf57ec2eeb1201911f54d514a27c32b571c5bb0fbadc5e.
The runtime candidate remains af4d0d3c; dbf98c55 identifies the rule evaluator,
not a re-labelled measurement. Original standing, bundle, queue, archived
outputs and frozen plans stay byte-identical. Retained is a unit-gate result,
not fixed-hardware qualification or a claim that every case beats NG.
Main stays e54e33eb with its typed_loop diff untouched. The next authorized
unit is numeric-scratch-placement, using the af4d0d3c queue after this update
is pushed; T038 owns its mechanism and remaining variance work.
