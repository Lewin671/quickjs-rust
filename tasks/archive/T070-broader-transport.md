# T070 broadened transport triage and closure

Candidate `b74a84a9c58120a7a5206db63b2e02779b1bd083` against exact base
`b93e4daa5d9d53f1780a66f31b31a96e2bfe1895`, 2026-10-23 UTC. The lead
amendment was recorded prospectively in `dd90d06d`; the frozen schema-3 plan
and attempt-1 evidence are unchanged. No origin push or merge.

## Decision

Seven-program wall geomean **0.975245059666**, instructions **0.987032034632**.
The wall result exceeds the lead's <=0.97 condition. Stop and close without
promotion; the final K=6 attempt is unused. Total spent remains one of two.
No new screen, formal, held-out, RSS or resource collection is authorized by
this result. Big.js improves materially, but that does not substitute for
the predeclared cohort. The broadened implementation remains on this branch
for lead review; it is not integrated or promoted.

## Collection and identities

- `20261023T075942Z-t070-broader-triage`: exit 0 / 287 s command;
  347.800 s end-to-end including 60.700 s quieting, budget met. Five natural
  layout pairs, qualified workers-4, all seven T065 round-2 development
  programs plus every fixed triage control: 115/115 valid, no invalid,
  heavy or DISTURBED sample; empty intruder log. Tool verdict: likely loss.
- `20261023T075949Z-t070-broader-compare`: exit 0 / 21 s;
  256/256 QuickJS-NG fixture output comparisons pass (counted from `ok: `
  lines in the hash-verified log).
- `20261023T080100Z-t070-broader-export`: exit 0; exports the complete
  result, request digest, binary identities and comparison log digest.
- iMac clock before submission: 2026-10-23 15:59:36 CST, outside maintenance;
  final remote free disk 55.876 GiB. Local disk was 55 GiB.

| Artifact | SHA-256 |
| --- | --- |
| request | `fcfc971326b37c8624432cbd85eba7733bd82b7c9e2b902518e74fa327d17920` |
| result | `9ed3398a5e40400cc620e8ea4d33421a103c3cecfc667d2bb1aba710698fcf7c` |
| base binary | `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1` |
| candidate binary | `b5a1cb5bd631d2b8254a8c8becc0600d0f6903e0205cc27a22752ac6d18f4051` |
| comparison log | `e678c1fc18a42b2fd6ce1f85fc5651419d278fb1f3b8eedc96514e86199f8c94` |

## Every measured case

Candidate/base, lower is faster. Intervals are exploratory 95% bootstrap
intervals for five natural-layout pairs; they do not establish formal
precision, layout robustness or complete-portfolio standing.

| Case | Wall [95% interval] | Instructions [95% interval] |
| --- | ---: | ---: |
| `development/big.js/arith` | 0.913263 [0.904939, 0.942580] | 0.955023 [0.955007, 0.955028] |
| `development/immutable/persistent` | 0.998894 [0.966460, 1.016491] | 0.989913 [0.989864, 0.989947] |
| `development/js-yaml/document` | 0.975763 [0.957022, 1.063961] | 0.995752 [0.995667, 0.995831] |
| `development/graphlib/paths` | 0.955574 [0.940626, 0.974693] | 0.977769 [0.977587, 0.978017] |
| `development/lodash/transform` | 0.983315 [0.949420, 0.990083] | 0.996711 [0.996307, 0.996891] |
| `development/diff/compare` | 1.011687 [0.997634, 1.055686] | 1.000055 [1.000030, 1.000123] |
| `development/moment/dates` | 0.991593 [0.979998, 1.007815] | 0.994781 [0.994757, 0.994851] |
| `sentinel/capturing_closure_call` | 1.039747 [0.988062, 1.451868] | 1.013545 [1.013446, 1.013900] |
| `sentinel/heterogeneous_property_read` | 0.942578 [0.861453, 1.007414] | 1.017690 [1.017626, 1.017777] |
| `sentinel/polymorphic_call_site` | 1.089715 [1.002099, 1.120486] | 1.014478 [1.014366, 1.014619] |
| `sentinel/prototype_method_call` | 1.013044 [0.803437, 1.172855] | 1.015245 [1.014971, 1.015481] |
| `sentinel/recursive_call_tree` | 0.988807 [0.959645, 0.996323] | 0.999726 [0.999639, 0.999969] |
| `sentinel/string_key_map_churn` | 1.075660 [0.984893, 1.131220] | 1.030547 [1.030403, 1.030608] |
| `broad/property_dynamic_read` | 0.993697 [0.979578, 1.143068] | 0.996201 [0.996161, 0.996301] |
| `broad/object_allocation` | 0.997380 [0.983185, 1.018432] | 0.997261 [0.997199, 0.997366] |
| `broad/string_slice` | 0.930130 [0.793556, 0.992957] | 1.006856 [1.006676, 1.006967] |
| `broad/top_level_function_call` | 0.962921 [0.940831, 0.973591] | 0.993773 [0.993744, 0.993806] |
| `external/kraken-1.1/json-stringify-tinderbox` | 1.002933 [0.997249, 1.019970] | 0.999561 [0.999520, 0.999905] |
| `external/kraken-1.1/json-parse-financial` | 1.009789 [0.967610, 1.018777] | 0.995010 [0.994941, 0.995092] |
| `external/jetstream3-js-subset/raytrace-public-class-fields` | 0.977645 [0.957446, 0.993148] | 0.989789 [0.989703, 0.989829] |
| `external/sunspider-1.0/access-binary-trees` | 0.957181 [0.939830, 0.968618] | 0.985224 [0.985149, 0.985287] |
| `external/kraken-1.1/audio-dft` | 0.975715 [0.960043, 1.021391] | 0.997692 [0.997638, 0.997769] |
| `external/jetstream3-js-subset/cdjs` | 0.978081 [0.956438, 0.987128] | 0.993688 [0.993463, 0.994314] |

The heuristic's obvious regression is `sentinel/string_key_map_churn`:
instructions 1.0305 exceeds 1.03 with its exploratory lower bound also above
the limit. Possible regressions: js-yaml, capturing_closure_call,
polymorphic_call_site, prototype_method_call, string_key_map_churn and
property_dynamic_read. Every point regression and interval is retained above;
no control is dropped or remeasured to change the decision.

## Stage seconds

| Stage | Seconds |
| --- | ---: |
| analysis | 2.657103 |
| build_base | 0.860536 |
| build_candidate | 71.380628 |
| build_quieting | 0.029512 |
| collection | 202.358797 |
| command_total | 287.106203 |
| end_to_end | 347.800416 |
| prepare_workloads | 0.205463 |
| queue_quieting | 60.699895 |
| seal_and_replay | 8.060113 |
| settle_and_launch | 1.214126 |
| validate_and_freeze | 0.161425 |

## Correctness and scope

`b74a84a9` passes the pre-commit fast gate and full `scripts/check.sh` with
the pinned shared Test262 corpus: 2362 default runtime tests, agents and
perf-counters checks, workspace tests, 732 benchmark-tool tests, 35 script
tests, policy/plan/context/size/hooks checks and 5170 curated Test262 cases.
Fast gate selects eval-code, block and function-statement Test262 slices.

Nine new runtime tests cover recovery-aware last-use proofs, branch and
backedge uses, duplicated exit operands plus frame publication, exact owner
release, dense holes/descriptors, VM argument order and setters, bound/proxy
calls and exceptions. No dependencies, unsafe, cross-crate public APIs,
Value-layout, binding-model or call-preparation changes.

The prior complete exact scan remains evidence for `83f47dba` only:
42672/42672 configured cases and zero actionable gaps. The broadened candidate
has not received a new full exact scan; it is closed before promotion.
No formal, held-out, RSS or resource claim is made for it.

Full local gate log SHA-256: `ca6bb907c2a641e63af077a0dd52904e0f0a529b38ae4660a04137909781c75e`.
