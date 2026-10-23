# T069: First candidate and pre-implementation evidence

- Status: historical-campaign; first engineering candidate diagnosed, no screen attempt started.
- Implementation: candidate `74e8f8aa`; superseding preparation revision is tracked in [T069](../T069-call-activation.md).
- Verified at: `74e8f8aa`, 2026-10-23 UTC; full gates, iMac comparisons/subset, complete local conformance and five-pair triage.
- Evidence: this immutable record and [frozen plan](../performance-units/positional-call-activation.json).
- Unresolved: no layout-robust performance or formal/held-out claim for this candidate.
- Next action: follow the active task; this diagnostic does not spend or reset a plan attempt.

## Collection and correctness

- `20261023T044841Z-t069-baseline-queue`: cancelled while queued (130), no
  measurements. The already-running T070 baseline job has exactly the same
  base/candidate, 30 blocks and workers-4 command; reuse its sealed bundle
  `20261023T044840Z-t070-base-queue` rather than duplicate host work.
- `20261023T045032Z-t069-exact-profiles`: exit 0; three complete-process
  1ms sample runs for each of the five selected development programs at b93e4daa.
  All 15 receipts and copied assets verify against exact image
  `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
- Shared baseline `20261023T044840Z-t070-base-queue`: exit 0 in 2,085 s,
  but unhealthy: base `branch_arithmetic` linearity 1.1696704 exceeds 1.15.
  All 30 blocks and samples are retained; no queue may use it. The Rust roles
  are byte-identical (`b3132b40...`), so this is not a candidate effect.
- `20261023T052725Z-t069-baseline-replacement`: exit 0 in 2,016 s; healthy
  whole-bundle replacement with `ready_for_unit_gates`, no measurement issues,
  same refs, 30 blocks and workers-4. No samples or lanes are pooled with the
  invalid bundle. This is a pre-implementation baseline, not a unit attempt.
- `20261023T052759Z-t069-bind-replacement`: exit 0; queue generated only
  from that replacement's complete healthy sealed bundle.
- `20261023T063345Z-t069-candidate-correctness`: exit 0 in 75 s, QuickJS-NG fixture
  comparisons and the full curated subset at exact candidate `74e8f8aa`.
- `20261023T063414Z-t069-triage`: exit 0 (268 s command, 333 s including quieting); all five frozen targets plus the
  fixed triage controls, exact base/candidate, no attempt spent.
- `20261023T065346Z-t069-screen-if-likely-win`: exit 0; deferred without starting a screen (triage was `likely loss`). Prospective gate:
  starts attempt 1 only if the preceding iMac correctness job exits 0 and
  the hash-bound triage reports `likely win`, within budget, without an error.
  Otherwise records deferral and spends no attempt. This avoids a second queue
  wait when triage is unambiguously promising; no acceptance threshold changes.

Candidate full-gate log SHA-256:
`f2a088d8dc4b35fc1bd628c36e3675e27dfd8813f19788cc1401dd108be34252`.
The commit hook passed the mapped 117-case Test262 slice in addition to the
full gate's 5,170 cases. The 24 new tests cover raw parameter preparation,
fallback selection and observable context/cell behavior, including synthetic
derived rest forwarding and `super` method receivers.
Local read-only reference comparison also passes **256/256 fixtures** on the
same candidate (unchanged comparison script in the scratch runner). Output
SHA-256 `95f3261da2f4c09d6564cacc2c99a6fb8af89144ba4db97968e25923fb792537`;
candidate local release/agents image
`2ce6dd53fc6673fba1733dda9e14d005c2977c8116340bc7bd9198c0303c71be`.
Complete local candidate scan: **42,672/42,672 configured cases pass**, zero
failures/timeouts/not-run and zero actionable gaps at exact `74e8f8aa`.
The independently scanned NG baseline and unchanged copied harness match the
pre-implementation scan; the complete candidate burndown entry is appended.

Fresh setup-frame inclusive unions (all three repetitions, recursion counted
once) bind the preselected programs; they do not retarget the unit:

| Development program | Setup union / main-thread samples |
| --- | ---: |
| lodash/transform | 368 / 4,439 |
| immutable/persistent | 1,278 / 9,249 |
| diff/compare | 1,015 / 14,787 |
| graphlib/paths | 386 / 3,248 |
| moment/dates | 997 / 26,068 |

The union recognizes `function_env`, FrameBindings, initial slot/cell/mask
routes, function-frame/VM constructors, frame_call_env/callee_env, seeding
eligibility and captured-cell indexing. It excludes body-level slow binding
access that positional authority can also remove, but may include coercion
called during setup. It is reach, not a savings forecast. Receipt hashes and
per-repetition fractions will be bound in the frozen plan.

Base full-gate log SHA-256:
`ed6fc08f35cbf3afec8ccfe841e6c00e0651f4f24821af2c27342ccbc5b04015`.
The log under `target/t069/base-check.log` is local and will be cleaned;
the counts and revision above are the durable result.

Before runtime changes, a complete local release/agents scan of b93e4daa also
passes all **42,672 configured cases**, with zero failures/timeouts/not-run
and zero actionable gaps (NG 42,602 pass / 70 fail). The complete entry is
appended to `docs/conformance/burndown.jsonl`. Exact unchanged harness scripts
and copies of the pinned test/harness directories ran under
`target/t069/conformance`; the reference executables were read-only links to
the main checkout. No checkout's `third_party` was modified. Eight workers,
30-second case timeout, `QJS_AGENTS=1`; unsupported NG-excluded cases are not
included in the configured-case claim. Raw evidence is local, not retained.

- Rust binary SHA-256: `557c0dd0a571d2ddb1b152c162bbf5f573f0e37fcfb7e25043427beb9e9c514d`.
- Rust case JSONL: `18f6a9a9db9a442f52f9b05ca9ffa618a877488360972eb6ef7a72f9ccc5c549`.
- NG case JSONL: `aae13b897e579f5f35ba7226d9e0c4a78d7105bb18cc33c7275986aa02e775cb`.

## Diagnostic triage

Result: **likely loss**; 105/105 valid pairs, no load-invalid pairs; budget met.
Natural layout, five pairs, qualified v5 workers-4; intervals are exploratory 95%, not acceptance.

| Case | Wall C/B [95% exploratory] | Instructions C/B |
| --- | ---: | ---: |
| `development/lodash/transform` | 0.9844 [0.9709, 1.0820] | 0.9890 |
| `development/immutable/persistent` | 0.9049 [0.8897, 0.9286] | 0.9204 |
| `development/diff/compare` | 1.0323 [0.9906, 1.0940] | 0.9983 |
| `development/graphlib/paths` | 0.9842 [0.9274, 1.0277] | 0.9929 |
| `development/moment/dates` | 1.0544 [1.0524, 1.0657] | 1.0040 |
| `sentinel/capturing_closure_call` | 1.0197 [0.9597, 1.1990] | 1.0001 |
| `sentinel/heterogeneous_property_read` | 0.9265 [0.8314, 0.9474] | 0.9999 |
| `sentinel/polymorphic_call_site` | 0.9740 [0.7103, 1.0085] | 0.9999 |
| `sentinel/prototype_method_call` | 0.9797 [0.9680, 1.3448] | 0.9992 |
| `sentinel/recursive_call_tree` | 1.1746 [1.1604, 1.1978] | 1.0002 |
| `sentinel/string_key_map_churn` | 1.0069 [0.9521, 1.0733] | 1.0309 |
| `broad/property_dynamic_read` | 1.0195 [1.0082, 1.1333] | 1.0022 |
| `broad/object_allocation` | 0.9931 [0.8379, 1.0187] | 1.0000 |
| `broad/string_slice` | 0.9449 [0.9246, 1.0277] | 0.9982 |
| `broad/top_level_function_call` | 0.9812 [0.9259, 0.9982] | 1.0000 |
| `external/kraken-1.1/json-stringify-tinderbox` | 0.9979 [0.9947, 1.0277] | 1.0034 |
| `external/kraken-1.1/json-parse-financial` | 0.9863 [0.9595, 1.0114] | 1.0004 |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.9736 [0.9696, 0.9794] | 0.9987 |
| `external/sunspider-1.0/access-binary-trees` | 0.9942 [0.9792, 1.0419] | 0.9888 |
| `external/kraken-1.1/audio-dft` | 1.0069 [0.9854, 1.0310] | 1.0002 |
| `external/jetstream3-js-subset/cdjs` | 0.9776 [0.9508, 0.9879] | 0.9791 |

Request SHA-256: `7e41b3bc2a53693ea740b383d072d0696ab8ecd9292d81aca7eb5e993c2ea1f5`.
Result SHA-256: `dc304d29115d59dd043eb15fd87fac68364ff046c423c63bfbf71727129bdc01`.
Stage seconds: `{"analysis": 2.5403427080018446, "build_base": 0.8942090249620378, "build_candidate": 70.70023403898813, "build_quieting": 0.029677932034246624, "collection": 183.56647677300498, "command_total": 267.5492752119899, "end_to_end": 333.2468180656433, "prepare_workloads": 0.1522222280036658, "queue_quieting": 65.70135307312012, "seal_and_replay": 8.058511283947155, "settle_and_launch": 1.2695015500066802, "validate_and_freeze": 0.15823742991779}`.

Only immutable shows a convincing target wall gain. The first candidate does not establish the full call-family payoff. Moment and recursive-call wall costs have nearly flat instructions; this run does not establish their cause. String-key churn instructions are about 3.1% higher.

The next engineering revision uses the already frozen transitive dynamic-scope metadata at outward native/getter/coercion boundaries, visits only cached global-fallback slots on refresh, and completes ordinary calls in place. All targets, thresholds, collection and attempt budgets remain frozen. These are hypotheses grounded in T068, not performance claims.

Fresh baseline queue development C/NG geomean: **2.7566091** at b93e4daa. This is a whole-process wall baseline, separate from T068 retired-instruction ratios.
