# Repaired S3b owner checks

Candidate **`ca5ab643`**, runtime **`1e91be92`**, baseline main **`3cc12c6f`**.
The candidate adds only the regenerated x86 order and documentation to the runtime
revision. Runtime/harness/tests/configuration are identical; ARM's build script
does not consume the x86 order. A post-order ARM rebuild preserves agents binary
SHA-256 `fb492869e525b8def3f02b023853481b2faa2f9a1b520c8890dc3d8f9a0ce202`.
This is an in-progress checkpoint: complete exact cached/off and qualified
public86 results remain required. Lead owns formal/held-out/RSS acceptance.

`./scripts/check.sh` passes, including **5,174 curated Test262 cases**. The
repair's staged gate and commit hook pass **2,637 default runtime tests** and the
curated suite; the order commit passes its CLI fast gate. Full agents/perf-counters
runtime suite: **2,692**. Each separate binding verification, forced exposure,
released-scope poisoning and IC verification suite passes **2,637**. Combined
binding modes plus IC-off pass **2,595**, with the same 42 IC-enabled mechanism
checks instead covered by the full IC verification suite. No test/exclusion
removal. All **299 NG scripts** pass in all six configurations with the standard
10-second per-case timeout and one serial comparison worker. [Modules](modules.json)
match NG exit/stdout in all six modes, including job completion, TZ=UTC.

The [rejected 73c checkpoint](../T077-s3b-exact-73c-rejected/README.md) preserves
its complete 42,669/3 exact result and burndown row. The repairs select late global
lexicals through checked cells and use a callable nonconstructor for the synthetic
Function prototype, removing its spurious inherited `prototype` property. All
three formerly failing upstream cases now belong to the curated allowlist.
Complete affected exact slices pass **100/100 Function.bind** and **42/42
global-code**, without gaps/timeouts/not-run; these are not burndown scans.

[Candidate counts](candidate-counts.json) reproduce all ten pinned development
sources/stdout/full counter maps twice. Every map equals the previous freeze's
map. The unchanged [baseline maps](../T077-s3b-owner-73c/base-counts.json) also
reproduce twice; the same input manifest/instrumentation/collector is used.
Manifest SHA-256:
`2f35fddc49930a43644cfe9452a6c994da2059bc59e2fc855f706f602076fd13`.

| Event | Main baseline | Repaired candidate |
| --- | ---: | ---: |
| CallEnv constructions / clones | 19,031,946 / 72,543 | 0 / 0 |
| Caller-environment equality proofs | 26,477,573 | 0 |
| Binding snapshots / frame forks | 72,101 / 100,578 | 0 / 0 |
| Apply-back calls | 5,082,372 | 0 |

These are event counts, not avoided mallocs. New work remains separately counted:
457,653 CellStorage allocations (including global caches), 328,929 promotions,
5,368 ScopeLease allocations, 2,137,508 pool requests / 673 actual new local
vectors. Logical variable tokens 29,298,771 equal 29,298,761 ordinary JS entries
plus ten host roots; 199,691 forwarding calls do not double-count entries. S3a
native argument-vector pop/family probe/decline counts remain zero on both sides.
Historical 24,987,337 belongs to the older dd0905b2 receipt, not current main.

[Depth maxima](candidate-depth.json) match the prior candidate exactly. Against
the unchanged [baseline](../T077-s3b-owner-73c/base-depth.json), the [explicit parity
probes](depth-parity.json) pass all sixteen base-success maxima and every
successful frozen landmark on ordinary main-thread CLI stacks. No stack override
or engine limit. Release maxima: forEach/map/reduce **872/865/919**,
call/apply/Reflect.apply **3338/2160/2294**, yield* **784**, plain callback recursion
**>=4096** (probe cap). Baseline: 796/790/829, 3234/2066/2188, 714, >=4096.

Default/agents/perf-counters production builds pass on both targets. The
[x86 builds](x86-builds.json) use the [regenerated order](../T077-s3b-repaired-order/README.md).
Six release warnings remain inherited from main's debug-only layout/identity
helpers; strict debug feature Clippy passes. [Deletion audit](deletion-audit.json)
finds no removed execution API, preserves the exact 592-arm native entry source
and leaves main's protected layout/IC modules, dependencies and third-party pins
unchanged. [Metadata](metadata.json) records binary/source/toolchain/pin hashes.

Complete exact cached/off scan commands use this frozen agents binary,
QJS_AGENTS=1, jobs=6, timeout=20 seconds, one timeout retry, no other QJS flags
except QJS_DISABLE_IC=1 for off. Only complete summaries will enter burndown.
Public86 remains blocked by [hostname qualification](../T077-s3b-host-qualification/README.md),
with no valid timing result and no guard/policy bypass. No origin push or merge
into main. Final exact and public86 receipts must be recorded separately.
