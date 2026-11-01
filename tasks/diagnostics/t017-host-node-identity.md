# T017 host-node identity tooling change

- Status: implementation present; tooling checks pass; full gate lacks Test262 input.
- Revision: parent `3cc12c6f8d93ffd6eae91d1bd44efe74134ae195` plus the
  host-node change on `agent/bench-host-node/codex`, 2026-10-31.
- Scope: tooling only; no iMac timing jobs, pushes or merges.

The comparison contract is owned by
[layout policy artifacts](../../docs/layout-policy-artifacts.md).
`layout_identity.node_identity` strips the first DNS suffix and case-folds;
`same_node` refuses missing/empty names. `same_host` applies that predicate
only to `node`, retaining exact dictionary equality for every other field.
Base A/A launch-context replay uses the same rule for its embedded host.
Raw `platform.node()` recording remains unchanged.

`benchmarks/analysis.json` and `benchmarks/resource-analysis.json` add the
identity module to their analysis source inventories and refresh their source
hashes because raw validators now import it. No analysis parameters change.
Measurement manifests, calibrated host policies, frozen plans and archived
evidence are unchanged.

## Converted comparisons

Line numbers refer to this change. All files below are in `tools/benchmark/`;
multiple predicate calls on one line are listed once.

| File | Lines |
| --- | --- |
| `base_aa.py` | 33 |
| `base_aa_evidence.py` | 24, 43, 61, 84 |
| `external_report.py` | 251 |
| `layout_development_a2.py` | 71, 122 |
| `layout_development_qualification.py` | 126, 127 |
| `layout_evidence.py` | 21, 36 |
| `layout_formal_continuation.py` | 65, 114 |
| `layout_formal_raw.py` | 24, 40 |
| `layout_held_out_policy.py` | 78 |
| `layout_panel.py` | 47, 48 |
| `layout_policy.py` | 104, 147 |
| `layout_qualification.py` | 211, 237, 247 |
| `layout_resources.py` | 57 |
| `layout_schedule_bands.py` | 68 |
| `layout_schedule_qualification.py` | 91 |
| `performance_decision.py` | 188, 567 |
| `performance_evidence.py` | 88, 101 |
| `raw_contract.py` | 275 |
| `resource_validation.py` | 327 |

33 predicate calls (including the launch-context host comparison and its
three consumers); `same_host` delegates node comparisons to `same_node`.

`quick_triage.py` records a raw node but has no direct node comparison; its
live admission uses the converted `layout_policy.load_policy` path.
Whole-artifact equality used to authenticate immutable snapshots/replayed
reports remains exact; that compares recorded provenance, not machine identity.

## Coverage and verification

- New helper tests cover suffix variants, multi-label domains, case,
  distinct short names, missing/empty nodes, unchanged raw values and exact
  non-node metadata (including missing/extra fields).
- Live policy and panel/matrix replay tests accept `.lan` versus `.local`,
  reject `mini`, preserve artifact hashes and reject other host-field drift.
- Base A/A replay tests exercise embedded context and sample hosts across
  suffix changes and reject machine/OS changes. Analysis inventory tests
  bind the new helper and refreshed hashes.
Eight test methods were added:

| Test file under `tools/benchmark/tests/` | Added coverage |
| --- | --- |
| `test_layout_identity.py` | Five methods: suffix/case variants, distinct short names, invalid nodes, exact remaining host fields and identity without a node |
| `test_layout_rule.py` | `test_live_policy_accepts_dns_suffix_change_but_requires_same_machine_and_recipe` |
| `test_layout_evidence.py` | `test_panel_and_matrices_accept_suffix_change_but_refuse_other_short_names` |
| `test_base_aa_evidence.py` | `test_context_and_sample_nodes_ignore_dns_suffix_only` |

Existing analysis/resource inventory assertions also bind the added helper.

- `python3 -m unittest discover -s tools/benchmark/tests`: all 761 tests pass
  after the approved digest refresh (106.957 seconds).
- `./scripts/check-touched.sh --staged --explain`: pass (761 benchmark
  tests, context/file-size checks and selected shell syntax checks).
- `./scripts/check-context.py`: pass; `git diff --cached --check`: pass.
- `./scripts/check.sh`: run once after approval, exit 1. Formatting, all
  three clippy configurations, agents/perf-counter tests, workspace tests,
  all 761 benchmark tests, frozen performance-unit validation, context,
  all 35 script tests, hook worktree isolation, shell syntax and file-size
  guard pass. Test262 subset stops at allowlist validation because the
  input submodule is uninitialized. Representative exact output:

  ```text
  error: derived Test262 source does not exist: test/built-ins/Symbol/iterator/prop-desc.js
  error: Test262 allowlist validation failed
  error: failed Test262 subset after 12s
  ```

  `git submodule status -- third_party/test262` reports
  `-4249661388e5d3f92a85186213da140a6481490f third_party/test262` (leading
  minus means uninitialized); the committed gitlink is that same revision.
  No submodule pointer or third-party content changed. No runtime/parser/
  lexer semantics changed, so the fast gate selected no semantic slices.

## Source fingerprint refresh

The lead approved updating only the mechanically invalidated source digests
in `benchmarks/performance-policy.json`, following the pattern of `41914706`.
Exactly nine digest fields change: the two analysis `protocol_sha256` values,
their six matching gate `required_protocol_sha256` entries and the hosted
implementation `aggregate_sha256`. Protocol IDs, policy configuration,
thresholds, plans and historical evidence stay unchanged. The strict policy
repository cross-check passes; no integrity check is bypassed.

| Binding | SHA-256 |
| --- | --- |
| Throughput analysis | `8c0a06d2df7348d13129d8eb941976adda15cba4ed87f35f1a0ec209b62e1e54` |
| Resource analysis | `3e266bed5136ec4bc29374df55d1b5a08e02996e69b7073138e822a3ac66eed3` |
| Hosted implementation | `66d058e63336a4ce32464378377d24a823b826935ff15c3394c0a3b7e5e673d5` |

Remaining limit: short hostnames are not globally unique. The other exact
host/toolchain/recipe fields remain required; no unique hardware identifier
was introduced. Next useful action: the lead can review this branch and
initialize the pinned Test262 input and rerun the full gate, then resume a
separately authorized iMac job; no timing qualification was run here.
