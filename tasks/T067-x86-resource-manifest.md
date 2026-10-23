# T067: A resource manifest for the measuring host

- Status: active.
- Implementation: present. `tools/benchmark/host_manifest.py` derives
  `benchmarks/.host-<platform>-resources.json` from the frozen
  `benchmarks/resources.json` with only `profile` and `build_recipes`
  rewritten for the live host; `tools/benchmark/local_receipt.py` accepts
  either manifest schema; `layout_resources.resource_manifest_for` selects the
  manifest a resource report names.
- Verified at: implementation `f801ae43`, 2026-10-23; full gate
  (`TEST262_DIR=<shared corpus> ./scripts/check.sh`, exit 0, 5170/5170) on the
  same code; iMac qualification jobs
  `20261023T035402Z-t067-resource-qualification` and
  `20261023T035953Z-t067-resource-verify`.
- Evidence: this file; fetched job directories under workspace
  `outputs/imac-bench/` (local artifacts, not retained in Git); frozen
  `benchmarks/resources.json` unchanged.
- Unresolved: `layout_resources.verify` derives the host manifest from the
  live recipe, so an instruction-blind decision must run on the measurement
  host (it says so when derivation fails). Evidence that names the frozen
  arm64 manifest still verifies byte for byte.
- Next action: none. T066's recorded resource gate can now be produced for its
  formal engines with the derived manifest; that is a decision on its frozen
  plan, not part of this task.

## Goal

Make the resource lanes usable on the only measurement host (the Intel iMac,
x86_64, fat LTO / 16 CGUs / panic=abort) without changing the arm64 manifest
that existing evidence names.

`benchmarks/resources.json` was frozen with `macos-arm64-release-v1` and
LTO-off `aarch64-apple-darwin` recipes. `ResourceRun` refuses a host whose
system or machine differs from the profile, and
`layout_resources.verify` replayed evidence against that same frozen manifest,
so an instruction-blind unit measured on the iMac could not produce resource
evidence (T066 attempt `20261022T225532Z-t066-resource-attempt`, exit 2,
`host machine 'x86_64' does not match profile 'arm64'`).

## Selection rule

The resource manifest is selected from host plus recipe, through the same
`tools/benchmark/host_manifest.py` derivation the broad and sentinel lanes
already use:

- The frozen `benchmarks/resources.json` is the default of
  `resource-benchmark.sh` and `resource-benchmark-report.sh`, and an existing
  user keeps measuring arm64/LTO-off exactly as before.
- `python3 -m tools.benchmark.host_manifest --resources` writes
  `benchmarks/.host-<platform>-resources.json`: the frozen bytes with only
  `profile` (id, `platform` = `darwin`/`linux`, `machine`, `rss_raw_unit`) and
  `build_recipes` rewritten from the live `rustc`/reference compiler and the
  canonical build flags. Everything else, including the protocol file list,
  is copied unchanged, so the protocol hash still verifies.
- A resource run and its report take that path as `--manifest`, and
  `local_receipt.py` takes it too, so the receipts name the derived profile
  and recipe.
- `layout_resources.verify` accepts a report naming either the frozen
  manifest or the manifest the policy host derives, and refuses any other
  manifest or host: a run from one host or toolchain is never interchangeable
  with another's.

## Qualification on the iMac

Job `20261023T035402Z-t067-resource-qualification` (timing) built current main
`95b6f03d` with the canonical recipe (binary
`b3132b40cc52b9b39012089db873c25dc7d2a1802e249322c1891577edee58c1`), wrote
receipts against the derived manifest, and ran all three lanes base-vs-base
(candidate = base = that binary) plus the pinned QuickJS-NG.

Derived manifest `.host-macos-x86_64-resources.json` SHA-256
`9f65728c4dc4b9868f43b3279138001b6527225d30eb0fcc4c985262abb5fecb`;
profile `macos-x86_64-fat16-abort-v1`, `darwin`/`x86_64`, RSS raw unit `bytes`.

| Lane | Health | Complete | Base/candidate ratio [95%] | QuickJS-NG/candidate |
| --- | --- | --- | --- | --- |
| size | healthy | yes | 1.0 (6,121,052 B both roles) | 4.2276× |
| fresh | healthy | yes | 0.990126 [0.973377, 1.000308], half width 1.72% | 1.1546 [1.1399, 1.1693] |
| rss | healthy | yes | 1.0 [1.0, 1.0] (4,644,864 B both roles) | 2.0035 [2.0035, 2.0035] |

The job command ended with a summary print that assumed a confidence interval
on the static size lane and raised `KeyError` after every run and report had
been written; its exit status is 1 for that reason only. Job
`20261023T035953Z-t067-resource-verify` (no-timing) re-derived the manifest
(same SHA-256) and replayed all three reports through
`layout_resources.verify`: exit 0, `missing: []`, healthy, profile
`macos-x86_64-fat16-abort-v1`. Report/raw SHA-256:

| Lane | Report | Raw |
| --- | --- | --- |
| size | `d9d8250a00693dd2f71957f932985a2c8e92c609106d1ef1710773e79fc3e37d` | `948f8506f8468787333e2f66d3dd623b4de685ee204a1697f945595f80353251` |
| fresh | `ad293038a9b28e18c97d1c6b9686912fcf781f3321dd9676ed5ff085c6330c10` | `45a6816bfab08b0d6862ef8023941f2bf235894f25a12c4cd5dedd62782099de` |
| rss | `fed49089924ce81abea05c91a7f5d33e8768f048f254a8b989719c94a3c529c2` | `89b8b90aeeb10a13e9509eb9a2a5977c5b49faa3a60cf5911179d4ad48385fde` |

The qjs-rust recipe the derived manifest carries is target
`x86_64-apple-darwin`, features `agents`, `lto` `fat-forced-by-cargo-config`,
and the iMac toolchain `rustc 1.95.0 …; cargo 1.95.0 …; LLVM 22.1.2`; the
reference is `Apple clang version 14.0.0` targeting
`x86_64-apple-darwin21.6.0`.

## Scope

- Allowed paths: `tools/benchmark/host_manifest.py`,
  `tools/benchmark/local_receipt.py`, `tools/benchmark/layout_resources.py`,
  their tests, `benchmarks/performance-policy.json` (hosted implementation
  hash), `docs/performance-workflow.md`, `docs/layout-policy-artifacts.md`,
  `tasks/T067-x86-resource-manifest.md`, `tasks/README.md`.
- Forbidden paths: `third_party/**`, runtime/parser crates, frozen layout
  policies and their sidecars, `benchmarks/resources.json`,
  `benchmarks/resource-analysis.json`, `tasks/performance-units/*.json`,
  burndown data, archived task records.
- Owner boundary: single owner; base `9f3ab85c`, branch
  `agent/t067-x86-resource-manifest/cmd`, worktree
  `worktrees/quickjs-rust/t067-x86-resources`.

## Acceptance

- [x] A resource profile for the x86_64 fat16-abort host exists and loads.
- [x] Frozen manifests stay byte-identical; their selection still verifies.
- [x] A base-vs-base run of all three lanes on the iMac is healthy and
  complete under the derived manifest.
- [x] `layout_resources.verify` accepts the derived-manifest evidence and
  refuses another host's manifest.
- [x] Focused tests and the full gate pass.

## Verification

```sh
python3 -m unittest discover -s tools/benchmark/tests
./scripts/performance-policy-audit.sh
./scripts/check.sh
python3 -m tools.benchmark.host_manifest --resources
```
