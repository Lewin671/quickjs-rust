# T077 S1 final verification

Implementation and requested correctness exit gates complete; formal stage
performance acceptance remains lead-owned. Verified candidate `06044f964f5bd82fe192e4fb998434b7c6c45afe`, tree
`2e303e7ced1e269fcddb1f2c45a8aa66da0d1f06`, owner branch
`agent/t077-s1-property-authority/codex`, base `dd0905b2`.

## Source and correctness

The candidate contains the [cutover](T077-s1-cutover.md), the
[Array.prototype hook correction](T077-s1-exotic-hooks.md) and the lead's
[decided key lifetime](../proposals/T077-s1-fallibility.md).
No engine changes follow the scanned candidate; final receipt/ledger changes
are documentation only. No exclusions or expected failures were changed.

Commands run on this source, 2026-10-30 UTC:

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: pass; 2,414 default runtime tests, 753 benchmark-tool tests, 35 script tests, default/agents/perf-counters clippy and selected feature tests, context/file-size/hook checks, 5,170/5,170 curated Test262 cases.
- `cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet`: 2,454/2,454 pass.
- `./scripts/compare-qjs.sh`: 258/258 fixtures pass.
- `cargo check -p qjs-runtime --target x86_64-apple-darwin --features agents,perf-counters`: pass; representation assertions also compile on the native aarch64 target. This is no x86 runtime or timing claim.
- The commit hook runs `check-touched.sh --staged --explain`: pass, including 2,414 runtime tests and 65 selected Test262 cases.

Assertions on both 64-bit targets: KeyId 4 B; KeyLease 8 B; ShapeEntry 16 B;
NamedStore 48 B; RefCell wrapper 56 B; ObjectData 96 B; Value 16 B.
The focused lifetime/layout/cache discriminator inventory and deletion counts
are in the cutover receipt. No runtime Rust references to PropertyStorage,
DynamicPropertyStorage, shared_dynamic_slot_value or literal_data_slot remain.

## Exact scan identity

```sh
QJS_AGENTS=1 QJS_CLI_PROFILE=release TEST262_CASE_TIMEOUT_SECONDS=20 \
  ./scripts/find-qjsng-gaps.sh --exact --all --out-dir target/t077-s1-exact-final
```

Rust compiler: `rustc 1.95.0 (59807616e 2026-04-14)`; native aarch64 macOS.
Release CLI has agents enabled and no perf/allocator counters. Full source and
build recipe are bound by the candidate commit; the driver builds that CLI.

SHA-256:

- Rust CLI: `8a8e8380a736c395f4bf6d07167efd928281f0936181533b560a8a39aff7a912`.
- NG Test262 runner: `a87d7408c98fbc55974789ac93edf99a24094bc9bce46dca3662420e5a9e7b2a`.
- NG CLI (differential fixtures): `4997edb9a3ce76f7458c2f32a6230456f49e4d5d49f725ec1d1f9f274032d49f`.
- NG config: `20bb96542cf064b0eb01039f2a178c6d58b0b713f2b012585195c777b8e8b2d9`.

Pinned NG: `f7830186043e4488f2998759d60a514faf07cbc9`.
Pinned Test262: `4249661388e5d3f92a85186213da140a6481490f`.
Both top-level submodules were initialized at those pins in this worktree;
reference sources/nested submodules and the shared input checkout were not edited.

Complete result: 53,572 unique corpus records; 10,900 NG-configured exclusions;
42,672 configured/eligible/executed Rust cases. Rust passes **42,672/42,672**,
with zero failures, timeouts, skipped configured cases or not-run cases. NG
passes 42,602, fails 70, times out zero. Both-pass 42,602; actionable gaps,
NG-pass/Rust-fail, NG-pass/Rust-timeout and NG-pass/Rust-not-run are all zero.
Rust passes all 70 NG nonpasses. This is all configured coverage, not a claim
about the 10,900 excluded corpus files. The exact driver exits zero.

The full unique case inventory, frozen engine sources and unchanged Rust binary
hash were checked before recording. Report hashes:

- summary.json: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
- cases.jsonl: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.

Burndown was appended with:

```sh
./scripts/test262-burndown.sh --report target/t077-s1-exact-final \
  --commit 06044f964f5bd82fe192e4fb998434b7c6c45afe \
  --source local-exact-release-agents
```

The durable [ledger](../../docs/conformance/burndown.jsonl) records that source,
complete counts and zero gaps. It does not establish standing for future engine
changes. Ephemeral raw logs are not needed to recover the conclusion above.

## Owner implementation commits

- `ace5f2ad`: key leases and isolated NamedStore/layout foundation.
- `c1c4a203`: consumable index miss proofs and compact dictionary slots.
- `4e464142`: verified foundation boundary and resume evidence.
- `d9b518ec`: explicit property domains through ordinary creation paths.
- `09fdf3ec`: owner migration verification and the former error-ABI question.
- `4e39af1f`: lease-safe key-ID recycling and decided allocation-failure policy.
- `01d25055`: string/symbol cutover, consumer migration and retired-store deletion.
- `06044f96`: intrinsic Array.prototype length/index hook correction.

## Performance boundary and next action

No iMac job or local benchmark timing was submitted. The lead owns the pending
baseline/formal whole-portfolio comparison, including S1's neutral-or-better
standing and S2 payer rule. Counters/sizes prove work/representation, not speed.
Historical retired mangled symbols remain in CLI linker-order recipes; the lead
must revalidate/regenerate those recipes with formal measurement.
Existing RC cycles remain S6 work; cyclic symbol-key graphs are not claimed reclaimed.
Disk stayed above the 8 GiB stop threshold (29 GiB reported near scan completion). No push, merge or main-checkout edit occurred.
