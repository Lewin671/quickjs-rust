# T037: Integration batch 1

- Status: active. Placement accepted; formal batch measured; both promotion
  decisions are inconclusive solely because audio-dft precision is insufficient.
- Implementation: present on `integration/batch-1`; neither unit is retained.
  No code was reverted and no frozen plan/threshold was edited.
- Verified at: `af4d0d3c`, 2026-10-12, locked 30-block comparison, both
  promotion decides, fresh queue and complete exact-candidate CI burndown.
- Evidence: [T018 standing](T018-broad-performance.md#standing-at-af4d0d3c),
  formal results below, and [leaf screens](archive/T018-batch-1-leaf-screens.md);
  raw target/ artifacts are local, not retained.
- Unresolved: audio-dft is the global unit's declared control, not unrelated
  noise; cycles CI is too wide. There is no measured >3% regression elsewhere.
- Next action: [T038](T038-audio-dft-variance.md) owns the audio-dft
  infrastructure diagnosis and re-decision procedure. Lead keeps main at
  e54e33eb; batch implementation/targets/controls pass, promotion inconclusive.
  New units base on the integration tip; frozen plans remain unchanged.


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

## Formal results at af4d0d3c

The 30-block perf-compare completed successfully under the host timing lock;
then both original promotion commands ran under that same lock. Its shell
returned 1 only because --require-retained rejects an inconclusive decision,
not because measurement failed. All target cases and all other case gates passed;
no wall-time divergence exists. Audio-dft alone is imprecise: cycles/base
0.996205, CI [0.906182, 1.040874], relative half-width 9.9344%, 30 valid blocks.
It is declared by the global plan; class promotion includes it as a portfolio
control. Both exact decide outputs have the same decision and reason:

```json
{"decision":"inconclusive","metric":"cycles","reasons":["external/kraken-1.1/audio-dft: insufficient blocks or wide confidence interval"],"wall_time_divergence":[]}
```

| Unit | Target cycles/base | 95% CI | Formal decision |
| --- | ---: | --- | --- |
| global repin: bitops-bitwise-and | 0.116990 | [0.115555, 0.117865] | inconclusive |
| global repin: top_level_function_call | 0.309658 | [0.309318, 0.310071] | same decision |
| class-field-batch-install: raytrace | 0.947281 | [0.943053, 0.948468] | inconclusive |

Original stdout files: target/batch1-global-numeric-binding-sinks-repin-decide-output.txt
(SHA-256 19e6c93ebdf7bf8eb4c88adae99b266c447de9a2552b19b30f8ad726057a7c6a),
and target/batch1-class-field-batch-install-decide-output.txt
(SHA-256 4af24f1df97b3a67c701f6d50d8d7fba76b6da5014507dd4c8f1de2796e695bb).
The full decision JSON files sit next to the comparison directory, named
batch1-formal-af4d0d3c81aad98721661689cf7224d2c17b9781-<unit>-decision.json.
Bundle/queue paths, hashes, all standing and >3% changes live in T018.

Burndown from CI coverage run 38242164403 names exact candidate af4d0d3c:
53572 total, 10900 NG-skipped, 42672 configured Rust passes, zero fails,
timeouts/not-run/parity gaps; NG passes 42602, fails 70. It was recorded once
in docs/conformance/burndown.jsonl after completeness verification. Artifact
file SHA-256 cf78d4178d426c4bcf9362b293e5dbc786702223f81e1e9123cd850e3c152789.

The original queue candidate executable remains linked from the main cache;
formal base SHA-256 is 6b40aa1944af1096657641cab5b2f80319cc35c3765a79da169cb5c698af09c4;
formal candidate SHA-256 is 6878520ff5fc8fc480e9fd8086c2b5caeb4ba609236ea423cd5a6d573a6fc54b,
byte-identical to placement. All samples remain in the sealed bundle, with
no outlier removal or rerun. A five-second monitor observed zero compiler
processes during timing. No full formal run was spent after this outcome.

The new queue binds af4d0d3c (measured engine tip), not the later doc/ledger
recording commit. Profile/freeze the next unit against that exact candidate;
its cached executable and queue identity are preserved. No main update was
made; both implementations remain present but formally unretained.
