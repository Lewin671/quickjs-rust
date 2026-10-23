# T070: First ownership-transfer triage

Queue-only diagnostic at candidate `83f47dba069ad3064cda49dd4ebbfbc68cd08735`, base
`b93e4daa5d9d53f1780a66f31b31a96e2bfe1895`, 2026-10-23 UTC. Natural layout, five
alternating pairs, qualified v5 workers-4 collector. This is not acceptance
evidence and consumes no frozen screen attempt. Current standing belongs to
[T070](../T070-value-transport.md).

## Identity and jobs

- `20261023T061259Z-t070-triage`: exit 0 / 310 s, no DISTURBED; verdict **unclear**.
- `20261023T061300Z-t070-compare-qjs`: exit 0, all 256 fixtures agree.
- `20261023T061458Z-t070-triage-export`: exit 0; read-only metadata export.
- Request SHA-256: `68345474581219be7a649aa7865d23b371db3b0c8865af4e20d0d75d5c705167`.
- Result SHA-256: `f56c9b451499da0eb7884f4eb267ac27a45050213e3ec204c0a65bcc8ce6e063`.
- Candidate binary: `9a4a8600c94b13a9d45eaa5dff9803495927f558592f9e813d35cd872b76397e`.
- Base binary: `b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`.
- Comparison log SHA-256: `6c05323ef555f4d20ef010c2c9999c37738241c11ec4cd8333379655520d86c5`.

Raw artifacts remain in the named queue jobs. Result and request hashes bind
refs, inventory, collector, binary builds, raw replay and load monitoring.

## Result

Candidate/base ratios; lower is faster. Intervals are five-pair exploratory
95% bootstrap intervals, unadjusted; they do not resolve layout bias or
unmeasured cases. No obvious heuristic regressions; possible wall regressions
are capturing_closure_call and moment/dates. Instructions are essentially flat
on the former and decrease on the latter; neither establishes a causal wall
cost. Graphlib has a measurable instruction reduction and a promising wall
point estimate; big.js is flat, and immutable wall increases despite lower
instructions. These mixed results justify a complete K=6 screen rather than
a gain or rejection claim. Targets and thresholds remain frozen.

| Case | Role | Wall ratio [95% exploratory] | Instructions [95% exploratory] |
| --- | --- | ---: | ---: |
| development/graphlib/paths | target | 0.946417 [0.935918, 0.984466] | 0.976696 [0.976643, 0.976742] |
| development/big.js/arith | target | 0.997420 [0.962744, 1.009348] | 0.999196 [0.999170, 0.999217] |
| development/immutable/persistent | target | 1.016849 [1.002332, 1.026499] | 0.992150 [0.992094, 0.992189] |
| sentinel/capturing_closure_call | control | 1.068028 [0.926258, 1.151215] | 1.000017 [0.999916, 1.000150] |
| sentinel/heterogeneous_property_read | control | 0.913936 [0.898443, 0.988760] | 0.999970 [0.999821, 0.999991] |
| sentinel/polymorphic_call_site | control | 1.006894 [0.964679, 1.039739] | 1.000022 [0.999890, 1.000173] |
| sentinel/prototype_method_call | control | 0.940074 [0.835946, 1.005606] | 0.999921 [0.999901, 1.000072] |
| sentinel/recursive_call_tree | control | 0.967049 [0.916354, 1.040672] | 0.999990 [0.999701, 1.000151] |
| sentinel/string_key_map_churn | control | 0.960451 [0.917357, 1.035833] | 0.999927 [0.999731, 1.000192] |
| broad/property_dynamic_read | control | 1.002192 [0.952476, 1.011136] | 1.005437 [1.005346, 1.005552] |
| broad/object_allocation | control | 1.003292 [0.968601, 1.024705] | 0.999994 [0.999867, 1.000102] |
| broad/string_slice | control | 0.952810 [0.932419, 0.958807] | 1.000475 [1.000395, 1.000627] |
| broad/top_level_function_call | control | 0.991231 [0.942995, 1.031184] | 0.999980 [0.999948, 1.000061] |
| development/lodash/transform | control | 0.978388 [0.967431, 1.008813] | 0.996168 [0.996052, 0.996571] |
| development/diff/compare | control | 1.003548 [0.997652, 1.010587] | 1.000036 [1.000020, 1.000066] |
| development/moment/dates | control | 1.028090 [1.005399, 1.050480] | 0.997343 [0.997282, 0.997362] |
| external/kraken-1.1/json-stringify-tinderbox | control | 0.983764 [0.980097, 1.024168] | 1.000118 [0.999701, 1.002219] |
| external/kraken-1.1/json-parse-financial | control | 0.979938 [0.976204, 0.999838] | 0.995686 [0.995668, 0.995761] |
| external/jetstream3-js-subset/raytrace-public-class-fields | control | 0.993563 [0.960175, 0.996138] | 0.993249 [0.993167, 0.993499] |
| external/sunspider-1.0/access-binary-trees | control | 0.958905 [0.952797, 0.973074] | 0.990897 [0.990804, 0.990929] |
| external/kraken-1.1/audio-dft | control | 0.989316 [0.931332, 1.013755] | 0.999932 [0.998258, 1.000982] |
| external/jetstream3-js-subset/cdjs | control | 0.990664 [0.973895, 0.996947] | 0.998600 [0.998213, 0.998852] |

## Stage times

Seconds, no queued waiting included in end-to-end. The 1800-second triage
goal passes; collection and build times are not a performance acceptance metric.

| Stage | Seconds |
| --- | ---: |
| analysis | 2.696 |
| build_base | 70.893 |
| build_candidate | 71.010 |
| build_quieting | 0.030 |
| collection | 154.725 |
| command_total | 309.397 |
| end_to_end | 370.082 |
| prepare_workloads | 0.176 |
| queue_quieting | 60.688 |
| seal_and_replay | 8.053 |
| settle_and_launch | 1.474 |
| validate_and_freeze | 0.160 |
