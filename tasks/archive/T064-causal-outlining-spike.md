# T064 causal cold-outlining spike

- Status: complete diagnostic; strict artifact invariant failed, not a performance rejection.
- Implementation: archive-only source generator on main `06e199fec9e9e51742cdae7633c5dc758c326752`; no production Rust/Cargo/recipe changes, no K1.
- Verified at: 2026-10-21 local date; build job `20261021T235604Z-t064-k-spike-v4` done0 (638s build/test work, not benchmark time),
  semantic job `20261022T000011Z-t064-k-spike-semantics-v3` done0; [producer/method](../diagnostics/t064-k-spike/README.md).
- Evidence: identities/numbers below; raw job artifacts can be fetched from the iMac queue by job ID. No raw binary/build output committed.
- Unresolved: how to own constants/import-stub placement under the final linker; value of that robustness versus T056 allocation/startup costs.
- Next action: review this partial mechanism result and literal/linker boundary before K1; no timing, PMU or attempt-budget change.

## Recipe and source discipline

Rust1.95.0/LLVM22.1.2, x86_64-apple-darwin; canonical `build_cache.RUST_RECIPE`: opt3/fat LTO/16CGU/abort/agents,
v0 symbols and align-all-functions4. Seven independent target deletions/recompiles at identical source/target paths; no executable cache reuse.
A separate debug test build verifies ObjectData/ObjectColdData sizes; probe source is removed before each canonical release build.
R0=112/400B, H+8=120/400B, C+8=112/408B in both families. H moves symbol/module brand offsets108/110→116/118;
C leaves every recorded header field offset unchanged. H word is initialized in all four constructors; C word defaults to zero.

Outlining mechanically moves all20 non-scalar match arms to one cold non-inlined service; execute itself is non-inlined and
writeback/deopt/materialization are cold non-inlined, preserving guards, ordered effects, admissions, residency and failure PC.
It is a deliberately broad diagnostic boundary, not a proposed optimization. Main's large `run` closure and outlined `execute` are different bodies:
compare R0/H/C **within** each family, never treat original→outlined byte differences as a cold-perturbation result.
The outlined images contain one service call per typed instance (Wide+13ff,Vm+1409); no crate/unsafe/FFI/dependency was added.

## A/A and positive controls

A/A whole executables are byte-identical, so both full typed bodies, selected bytes/PCs, tables and spills reproduce.
Original Wide/Vm extents13696/13792B and stack reserve0x178 repeat exactly.

| Unoutlined probe | WideLoopFrame / Vm result |
| --- | --- |
| H+8 | Both Add stores move+12B:2a66→2a72 /2b0c→2b18; Update stores1532→1538 /1597→159d (+6B). Extents13712/13808; dispatch PCs86/96 stay fixed but table positions/targets change. Both raw-byte and relative-PC controls detect drift. |
| C+8 | Body extents, all selected instruction PCs and tables stay fixed. Update's1.0 load and BitwiseAnd's−2^32 bound still change RIP displacement+16B in both roles, with identical referent bytes. __text grows16B and following __const moves16B. Both raw-byte controls detect drift. |

## Outlining result: partial mechanism, strict failure

Under **both** perturbations, all instruction boundaries in each outlined body remain equal, as do non-address bytes,
owned dispatch/BinaryOp table bytes/targets/relative positions, selected loop instruction bytes/PCs and stack layout.
Wide/Vm extents stay6272/6320B; stack reserves stay0xc8/0xd8; dispatch stays+f0. Add stores stay+931 in both roles (different scalar-bank index registers).
This supports removing the off-path object-layout coupling. It does not certify all numeric paths or future perturbations.

| Outlined probe | Remaining final-linked movement |
| --- | --- |
| H+8 | Strict **fail** in both roles:83 instruction encodings differ per body, all classified address operands.55 __TEXT/__const references (54 numeric loads and one panic literal) and each `_fmod`/`_pow` call move by+32B; the other26 are service/deopt/drop/cleanup/panic references. Even entry prologue bytes fail; they cannot be omitted because constants were hoisted there. |
| C+8 | Selected paths **including entry loads**, tables and spills pass raw equality. Whole body still differs at25 cold deopt/drop/cleanup/panic/metadata fixups per role. This is a finite selected-closure result, not whole-body byte equality or universal isolation. |

For H, Wide entry486810→486d20 and Vm484ce0→4851f0 move+0x510; __text size497c50→498180 and
__stubs/__const move+0x530. __const starts49a340→49a870, size68ee8 unchanged. The+0x20 gap exactly explains numeric literal/import call displacements.
First entry NaN load: Wide+6c `f20f102544590100`→`f20f102564590100`; Vm+77 `f20f102569740100`→`f20f102589740100`.
Both still refer to __const+1e88 (`000000000000f87f`). All55 changed constant references retain their section offsets and checked8/16-byte spans; numeric loads check their full operand width.
Full-body inspection resolves import stubs with `otool -Iv`; Wide `_fmod` at+652 and `_pow` at+12f9 also change displacement+32B.
C moves both outlined entries and __const+16B, hence unchanged numeric RIP deltas; its25 remaining differences are outside selected hot paths.

The residual observed mechanism is **text/constant/import-section placement**, not a demonstrated new internal table change, spill change, constant merge or IPO rewrite.
Final artifacts alone do not identify every compiler/linker pass. A separate crate does not control these final section distances under fat LTO.
LTO-opaque object/extern C remains an unapproved no-FFI conflict and would still need linker-closure auditing; this experiment does not prove it is the only solution.
Owning literals/local call targets or an opaque constant-input ABI are design questions for review, not implementations or permission to relax raw-byte acceptance.
No padding search, winning-layout selection or normalization was used to pass H.

## Semantic checks, limits and interruptions

Every exact image passes256/256 unchanged QuickJS-NG fixtures (1792 comparisons). Generated o-c8 source passes2346 runtime tests with agents.
All six unique source variants first pass local cargo check; production worktree `check.sh` passes (including662 benchmark-tool tests and5170 Test262 subset cases).
Final document/context/file-size/staged gates are recorded in the commit. No production semantics changed, so touched-file semantics slices select none;
the full curated subset ran anyway. These receipts do not substitute for future K1 full configured exact Test262/performance gates.

The path decoder reads emitted tables, resolves the actual ops-length loop-header comparison and models true/false Boolean tag/payload branches.
Its selected Add/StrictEq/And/Lt/Update/Exit/Jump paths are not a complete CFG; dispatch linear blocks and entry loads are also protected.
`closure.py` additionally aligns **every** outlined-body instruction start/length and classifies all raw differences; address normalization is attribution only.
There is no full production checker, all-feature/target matrix, pre-link IR firewall proof or performance result here.

Retained interruptions:234712Z console smoke failure;234929Z print/final-undefined smoke failure;235048Z A/A complete then H constructor precompile failure.
235403Z semantic job cancelled before execution to fix ROOT_DIR;235424Z checked two R0s then stopped on the incomplete matrix.
No missing role is counted as a pass. The final complete matrix uses corrected CLI/preflight/constructor handling; all jobs are non-timing and consume0 attempts.
The frozen old layout-edges plan hash remains d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284:1/2 failed, one shared A+B reserve unchanged.

## Artifact identities

| Role | Canonical executable SHA-256 |
| --- | --- |
| r0a | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |
| r0b | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |
| h8 | `b6b434cbf698d43dbeb12066abe0d36b9ee66ea78a902a115a77b0569dc83912` |
| c8 | `5cb7897cc00d275fc9df274bc7b70a787b637887a7e448990e0e4a6f9b07fe38` |
| o-r0 | `9dcdd7b63f192a4463a86831ae8596f0767e0329debbb03bcf485a0f13485cf2` |
| o-h8 | `4d171831dbf237d1e51fd51b7076492fd8a27415bf8770be478b3ac65e406bca` |
| o-c8 | `f944501a82d751be4bdb5ec6efe8411b3057e24fa1df0c9a27e8a54a6e94ee37` |

| Receipt / producer | SHA-256 |
| --- | --- |
| request | `d01b4e02b859468e78af16248d8fc72cd39f169786e3c8fafd339b35efe59ee9` |
| builds | `b3492134c30fe2df12bff3002d03a5645b54918546f40fc625ccddd471af4b07` |
| comparison | `0f6cab54c38d461c54a19167d348b0fcf97fa519a6589b4d8aa6e6dce5fbc714` |
| build.py | `f3e78f4818768dc4874f105786e5d750f2a4abdc9a6d496685f73554ae10b1f0` |
| transform.py | `c06443d47aba45b83382665cbc6210b92642c6cb3c212343eded7c8456d92799` |

Analysis/semantic producer bindings:

| File | SHA-256 |
| --- | --- |
| closure.json | `58ffa1e20b638c283166b0517ecd30af9ebd3b4b2754c9714b138efa501be322` |
| decode.py | `6becb61bf2e22b7f2e20bb605aaec6318fdfa49d74119a2abdb4bdd766811296` |
| compare.py | `8b3db84423c96eb51bc803fbd56eb1d42f3829f81cbaed374525b9691a4b1bb3` |
| closure.py | `84529726831e4fc03bfa56b3d65244a4df9c8fc93b1fdc96e231669d40b33f4b` |
| semantics.py | `fe1c85f03ccb3a409af6ff8cd226831964bd55ab574d66a9618f5bb877d33798` |
| NG result | `ec6076b445b5320bde5de50883089a76e75895e334ae990a0c398749f35c2529` |
