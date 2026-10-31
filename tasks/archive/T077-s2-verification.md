# T077 S2 implementation and correctness verification

Executable candidate: `fc64629a975431cd94cc22df2930bf49ad9593a8`, tree `d7cc512a089b65d7d8252d619f597daa9549f819`;
base `b2daec46`, branch `agent/t077-s2-named-stores/codex`.
The final creation-proof correction shares structural subscriptions and disables
creation/global-object slot proofs in IC-off mode. Both exact comparisons completed on the final artifact; every configured case
uses that one CLI. Implementation and correctness are complete; performance
acceptance is blocked on benchmark-transfer authorization and lead measurement. No push, merge or
main-checkout source edit.

## Implementation and deletions

- Functions/arrays use NamedStore; old text/symbol tables and function ordering
  index are gone. Domain ownership reaches every array constructor and installer.
- `value/named.rs` is the borrowed family facade. Map/Set carriers are borrowed
  on hits. Namespace/Proxy/numeric/string/lazy/private hooks stay explicit.
- One weak shape IC serves interpreter, compact/wide, typed loops and leaves:
  mono, allocated four-way, then permanent megamorphic generic resolution.
  Entries store no Value or strong receiver. No legacy exact-value epoch remains.
- Prototype and enumeration dependencies invalidate before structural edits;
  value-only writes preserve them. The first edge guards contextual array
  prototype wrapper identity. Shared-layout broadcasts are deliberately conservative.

Deletion diff at executable candidate vs S2 base (removed/added lines, including
associated migrations rather than claiming every removed line was storage):

| Path under runtime src | Removed | Added |
| --- | ---: | ---: |
| function/value.rs | 519 | 24 |
| value/array.rs | 139 | 204 |
| bytecode/named_property_cache.rs | 597 | 389 |
| bytecode/enumerate_keys_cache.rs | 188 | 63 |
| bytecode/typed_loop/mod.rs | 127 | 2 |
| bytecode/typed_loop/execute.rs | 120 | 28 |
| bytecode/compact_fn/creation_cache.rs | 80 | 88 |
| value/object/slot_reads.rs | 106 | 31 |

The new semantic modules replace deleted implementations; no second named store,
old fallback, dependency, crate, unsafe or binding-model change was added.
Runtime Rust has no CacheProbe, CachedValue, ExactWay, InheritedWay,
LazyFunctionProperties or symbol_properties references in these subsystems.

## Sizes

Native aarch64 and executed x86_64 test binaries agree: ArrayData 64 B,
ArrayColdData 112 B, FunctionData 288 B, FunctionAuxiliaryState 24 B,
IC entry 32 B, IC handle 8 B; debug site state 88 B. Release tests measure site
state 64 B. S1 compile-time assertions remain: KeyId 4, KeyLease 8, ShapeEntry
16, NamedStore 48, RefCell wrapper 56, ObjectData 96, Value 16 bytes.
x86 tests are correctness/size verification, not iMac timing.

## Completed checks (2026-10-30 UTC)

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: passes on executable candidate: 2,418 runtime tests, 753 benchmark-tool tests, 35 script tests, feature gates, format/context/size/hooks, 5,170/5,170 curated cases.
- `QJS_VERIFY_IC=1 cargo test -p qjs-runtime --lib --quiet`: 2,418/2,418.
- `QJS_VERIFY_IC=1 TEST262_DIR=... ./scripts/test262-subset.sh`: 5,170/5,170.
- `cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet`: 2,459/2,459.
- `QJS_VERIFY_IC=1 COMPARE_QJS_JOBS=2 ./scripts/compare-qjs.sh`: 259/259 fixtures; `QJS_DISABLE_IC=1 COMPARE_QJS_JOBS=2 ./scripts/compare-qjs.sh`: also 259/259, including named-shape-hooks.
- `cargo check -p qjs-runtime --target x86_64-apple-darwin --features agents,perf-counters`: pass.
- `cargo test -p qjs-runtime --target x86_64-apple-darwin --lib --features agents,perf-counters named_property_cache::tests -- --nocapture`: 9/9.
- Native/x86 array/function size tests pass; release/perf-counters cache tests 9/9 (the cfg correction makes the test itself compile without debug verification fields).
- Final creation-proof test also executes on x86_64 with agents/perf-counters; 1/1 passes. Every implementation commit passed its staged fast gate.

Focused proofs cover weak lifetime/key recycling, shared live values, four-way
promotion/mega permanence, dictionary rollover/reconfiguration, receiver edge,
all intermediate prototype dependencies, accessor exclusion, namespace cache
exclusion, lazy function slots and array/function own slots. Differential hooks
cover Reflect/super receivers, Proxy effects, ordering/freeze, resize and detach.
Existing module tests cover live namespace cells; configured exact scans remain
the final coverage authority.

## Complete exact comparisons

Both commands exit zero on native aarch64 macOS, Rust 1.95.0 (59807616e
2026-04-14), agents enabled and perf counters disabled. The exported input is
`TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262`;
the initialized worktree input has the same pinned corpus. No exclusions,
expected failures, reference sources or corpus source files were edited.

```sh
QJS_CLI_BIN="$PWD/target/release/qjs" QJS_AGENTS=1 QJS_CLI_PROFILE=release \
  TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=4 \
  ./scripts/find-qjsng-gaps.sh --exact --all --out-dir target/t077-s2/exact-final
QJS_CLI_BIN="$PWD/target/release/qjs" QJS_DISABLE_IC=1 QJS_AGENTS=1 \
  QJS_CLI_PROFILE=release TEST262_CASE_TIMEOUT_SECONDS=20 TEST262_BASELINE_JOBS=4 \
  ./scripts/find-qjsng-gaps.sh --exact --all --out-dir target/t077-s2/exact-off
```

Each report has 53,572 unique records, 10,900 unchanged NG-configured exclusions,
and **42,672/42,672 configured Rust passes**. Failures, timeouts, not-run cases,
actionable gaps and lost passes are all zero. NG passes 42,602 and fails 70;
Rust passes every NG nonpass. Both summaries and full case inventories are
byte-identical. This establishes configured coverage, not the excluded files.

Frozen CLI SHA-256:
`5181de100e8c5b838c1fe0ada28e620b2c9713e646d21080f0762f407ccfa717`.
It was built from the staged tree `d7cc512a089b65d7d8252d619f597daa9549f819`,
verified to equal committed `fc64629a`. Independent earlier builds differed, so
no claim of their byte identity is made. Both drivers were paused in metadata,
verified to have no workers or executed configured cases, then atomically pinned
to this final artifact before resuming at 2026-10-30 09:37 UTC. Earlier records
were NG exclusions only. The CLI hash remained unchanged through completion.

Other SHA-256 identities:

- NG CLI: `4997edb9a3ce76f7458c2f32a6230456f49e4d5d49f725ec1d1f9f274032d49f`.
- NG runner: `a87d7408c98fbc55974789ac93edf99a24094bc9bce46dca3662420e5a9e7b2a`.
- NG config: `20bb96542cf064b0eb01039f2a178c6d58b0b713f2b012585195c777b8e8b2d9`.
- Each summary.json: `cffdf394963ec66c41d006192593e9975e6b082a55483748550b7a9fb866e80f`.
- Each cases.jsonl: `2263183d8df546cb37338205c107cc4614a7ddee069a711be6181c0e27c2f61a`.

Pinned NG `f7830186043e4488f2998759d60a514faf07cbc9`; Test262
`4249661388e5d3f92a85186213da140a6481490f`.

Burndowns were recorded with `./scripts/test262-burndown.sh --report <directory>
--commit <revision> --source <label>`. The canonical ledger permits one entry
per commit. Cached mode binds `fc64629a` / `local-exact-release-agents-s2-cached`;
IC-off binds the documentation-only descendant `55cd82ac3ee4640f6bf8d542850a8e9ff249bbf5`
/ `local-exact-release-agents-s2-ic-off`. An explicit diff verified all engine,
Cargo, build, harness and reference-pointer files identical between those
commits. Both records therefore describe the same frozen executable source;
no committed historical record was replaced. See the [ledger](../../docs/conformance/burndown.jsonl).

## Performance boundary and lead action

[Ten-program final replay](T077-s2-creation-counters.md) and [initial receipt](T077-s2-counters.md) records checksums,
exact diagnostic binaries, per-program/family events, before/after work and
instrumentation limits. Higher materialization/intern/field-copy work is an
open risk. No timing was collected and no job name or speedup is asserted.
The iMac runner pushes temporary refs; clarification of the user's no-push rule
is pending. The lead owns formal four-aggregate acceptance and historical linker
order revalidation. S2 is not accepted on counters or curated correctness alone.


No triage number or job name is available: no iMac timing job was submitted.
The runner's `submit()` invokes `git push --no-verify` to private temporary
`refs/bench/...`; the user explicitly prohibited pushing. Clarification was
requested and not received, so the owner did not perform that transfer or
substitute local timing. The iMac quiet window is also mandatory.

The lead must authorize/perform that transfer, run quick triage against
`dd0905b2`, then the formal development/external/equal-case-selection/held-out
comparisons against the accepted S1 milestone and rebuild baseline. Freeze this
source before acceptance-only held-out collection. All four aggregates must
improve; counters and correctness alone do not accept S2. Historical CLI linker
order entries also need lead revalidation under the formal recipe.

## Owner commits

- `dd06aa05` Unify function and array named property storage.
- `3577b2c4` Replace named value caches with weak shape and prototype proofs.
- `fc5da912` Borrow Map and Set property carriers on shape cache hits.
- `34a63be1` Record S2 development work counters and frozen candidate.
- `884d3ba2` Verify shape cache diagnostics in release builds.
- `fc64629a` Share structural dependencies with named creation proofs.
- `55cd82ac` Record final structural creation proof validation and counters.

Design corrections and their reasons are in T077 and the object contract:
cold array stores with explicit domains; conservative, bounded shared-layout
subscriptions; weak contextual array prototype edges; creation proofs share the
same subscriptions. No S3-facing semantic error ABI, new crate/dependency, unsafe,
FFI, Rust async or environment/binding-model change. Existing RC cycles remain
S6 work. Every disk check stayed above the 8 GiB stop threshold. Generated build/corpus bundles are not committed.
