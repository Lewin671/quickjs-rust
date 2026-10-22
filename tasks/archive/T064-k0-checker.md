# T064 K0 static checker

- Status: final amendment2 qualification complete; A/A passes, H+8 fails as positive control, C+8 passes with zero residuals. Checker iteration stopped.
- Implementation: maintained Python checker and archive-only H+8/C+8 transforms;
  `scripts/check-kernel-isolation.sh`; no production runtime/Cargo/recipe or performance-policy change, no CI hook.
- Verified at: frozen checker `1d12a973`; 61 focused /723 tool tests and fast/full gates including 5,170 subset passes; sole final queue job `20261022T060712Z-t064-k0-amend2-final` and offline replay agree.
- Evidence: contract/method and exact current-main identities/results below; raw queue artifacts are fetchable by job ID.
- Unresolved: none for this checker brief; K1 runtime/performance qualification is outside scope.
- Next action: lead integrates the checker and freezes the successor plan before K1. No further checker iteration or CI hook in this brief.

The [lead decision](../T064-kernel-isolation.md#lead-decision-2026-10-21)
is the acceptance contract. Both typed roles are mandatory, including every
entry/cold instruction. The original `run` closures and future outlined
`execute` entries are separate admitted families; mixed families fail.
Structural v0 names select exact roles; aliases, duplicate names, unknown
wrappers and linker clones fail. Original execute adapters have an explicit
closure inventory, never a body-size selection heuristic.

`kernel_macho.py` reads final thin x86_64 Darwin executables, nm extents,
import identities and LC_DATA_IN_CODE tables, reusing the spike's maintained
section/symbol helpers. `kernel_decode.py` checks every body byte against
Apple otool's emitted bytes and an audited opcode/operand subset. Inline
tables use their actual signed targets and referenced bases; accidental
linear decoding through tables is ignored, and tail NOP bytes are verified
independently. Every code byte is decoded; unsupported shapes fail closed.
`kernel_isolation.py` compares relative instruction PCs/lengths, non-address
bytes, CFG edges, tables/positions/padding and stack-affecting operands.
Only encoded disp32 RIP operands and external E8 calls may vary. RIP
referents retain their full memory operand width and relocation-aware graph hash; calls
retain exact local symbol/import identities, including closure/clone details.
RIP import slots also retain binding identities: equal zero pointer bytes cannot
hide retargeting to another import. Zero-fill/non-file-backed referents and
unresolved import slots fail closed. Allowed differences are individually
reported, even when another check fails.

LEA has no dereference width. For owned tables it protects the full table;
other address-only LEAs stop at their containing nm symbol extent when known,
using the next data boundary or section end only for anonymous data. That can reject unrelated metadata movement; it
never certifies an opaque pointer by checking an arbitrary short prefix.
Under [feedback-1 amendment](../T064-kernel-isolation.md#lead-decision-2026-10-21),
`kernel_relocations.py` decodes native-pointer dyld rebase/bind streams.
`kernel_referents.py` compares section/symbol/offset or exact external binding
identities, keeping non-pointer bytes raw. Confirmed code outside the protected
set is an identity-only leaf; protected code remains fully compared. Data and
protected-code contents form a breadth-first graph of depth 2, with cycles
checked once. Mach-O instruction flags, function symbols and inline data
classify local targets; imported code needs an x86 stub matching the exact bind.
Unresolved targets, partial slots, weak/chained/threaded/unknown fixups or new
nodes beyond the budget fail closed. External imports are resolved binding
identities, not a claim about bytes in external dylibs. Schema3 fingerprints
record the amendment and reject prior-schema comparisons. Pointer-value changes
with equal graph contents are explicitly allowed displacement changes.
The checker proves equality for this pair under this decoder, not future
compiler isolation, wall-time neutrality, or structural type/field separation.

`kernel_build.py` reuses the spike's archive/probe/build/smoke method. Submit
from this worktree through the workspace's iMac queue, without `--timing`:

```sh
/Users/qingyingliu/Code/co-work-space/scripts/imac-bench.sh run \
  --name t064-k0-main --with 9eba7a1d -- \
  './scripts/check-kernel-isolation.sh --full --ref 9eba7a1d --output "$BENCH_OUT/k0"'
```

Default mode builds reference plus H+8 (or `--perturbation C8`); `--full`
builds r0a/r0b/h8/c8. Each uses the same source/target paths after deleting
the canonical target; no executable cache. Independent test builds witness
sizes and header offsets, then remove probe code before canonical builds.
Canonical field/initializer insertions preserve source lines, with per-file
witnesses in builds.json; panic-location metadata retains raw comparison.
Unchanged canonical images or incorrect/missing layout witnesses reject a
perturbation. A failed A/A stops interpretation. Nonzero status 1 means
contract drift; 2 means invalid/incomplete evidence. H+8 failure on current
unoutlined main is the intended positive control, never an isolation pass.
Compiler admission is Rust1.95.0/LLVM22.1.2, Darwin x86_64; recipe/features
come unchanged from `build_cache.RUST_RECIPE`. Unknown build environment or
changed compiler/tool identities fail. Requests retain compiler, linker,
Cargo configuration, producer and source identities, logs, binaries and
fingerprints; transient build trees are removed. Offline fingerprint comparison
alone cannot certify source/recipe provenance; the queued producer supplies it.

## Proposed CI hook (not enabled)

After K1 satisfies the contract: a production-target non-timing queue job
runs the default targeted perturbation on each PR, alongside a separately
reviewed source/type firewall. Nightly and compiler/linker/recipe/ABI changes
run `--full`; wider feature/helper/combined matrices require later audited
transforms. Archive request/build/fingerprint/comparison JSON and logs even
on failure. Do not automatically refresh fingerprints, add this currently
failing positive control to required CI, or claim Linux/arm64 qualification.

## Verification

Focused synthetic tests cover allowed RIP/call movement and negatives for
3-byte insertion, historical Add29ca→29be drift, hot opcode/referent/width/
stack/table changes, missing/duplicate/aliased/cloned roles, unknown decoder,
unknown relocation, stale transforms and optimized-away/missing probes.
No engine semantics changed; touched-file Test262 slices select none.

The first local full gate stopped at Test262 because this worktree had no
initialized upstream input. The first error was:
`error: allowlist entry does not exist: third_party/test262/test/language/statements/class/definition/fn-name-accessor-get.js`.
Initialized pinned Test262 `4249661388e5d3f92a85186213da140a6481490f`, then
reran the full gate successfully. Local Cargo calls checked disk availability
before execution; debug/test symbols and incremental output were disabled
for disk budget only. All local build outputs stayed under `target/`;
`target/debug` was removed after verification. No runtime semantic file changed.

Final review added RIP import-identity and zero-fill negatives (34 focused
tests pass). This changes checker validation only; the queued canonical
builds at producer `7ed7479f` remain usable. Final checker `36ab76e3`
re-fingerprinted all four downloaded images after verifying every executable
hash against builds.json. Its comparison JSON is byte-identical to the queue
receipt below. The replay used local Apple otool cctools 1040 / LLVM 21.0.0,
SHA-256 `8353ad3c42743a93e9ca7d0d4e9d3df22305252d169c42c543ee22eeb8c33809`.
Both decoders agree on the complete roles and results; all canonical builds
were on the queued iMac, not on this laptop.

## Initial raw-pointer qualification (superseded contract, retained evidence)

Non-timing queue job `20261022T043602Z-t064-k0-main`, start
2026-10-22T05:03:43Z, completed with exit 1 after 419 s of build/check work.
This is the expected contract-failure result, not a timing or performance
rejection; 0 attempts consumed. No existing job was cancelled or bypassed.
Input main is `9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30`; producer is
`7ed7479ff9db1e984719c80ca71328af47ad6056`. All four archives are unoutlined.
Rust 1.95.0 commit 59807616e1fa2540724bfbac14d7976d7e4a3860 / LLVM 22.1.2;
Apple clang 14.0.0; canonical fat16/abort/agents/v0/align-functions4 recipe.
Inherited build-identity environment and global Cargo configs are empty.
Actual iMac tools: ld SHA-256
`c1266cbb6d485669d91a1944c455579b94bc3c40ca33d5411e63da3dc32874b5`;
otool cctools 1001.2 / LLVM 14.0.0 SHA-256
`11dee92ac5a151aeb5f896b2beb8a4ddc269b22b4113c9a35a9fbc25c4df3815`.

| Build | ObjectData / ObjectColdData | Canonical executable SHA-256 |
| --- | --- | --- |
| r0a | 112 / 400 B | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |
| r0b | 112 / 400 B | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |
| h8 | 120 / 400 B | `b6b434cbf698d43dbeb12066abe0d36b9ee66ea78a902a115a77b0569dc83912` |
| c8 | 112 / 408 B | `5cb7897cc00d275fc9df274bc7b70a787b637887a7e448990e0e4a6f9b07fe38` |

H shifts symbol/module brand offsets 108/110→116/118. C leaves all observed
header fields unchanged. All four canonical images pass the 4950 smoke;
probe source is absent from release builds. R0a/R0b transformed-source hashes
also match. Header size/offset witnesses and changed canonical images admit
both perturbations; no optimized-away probe is counted as a result.

| Comparison | Vm / WideLoopFrame result |
| --- | --- |
| A/A | PASS; whole executables also byte-identical. Complete bodies have 2937 / 2920 instructions. |
| H+8 | FAIL in both roles: extent, owned tables, padding, stack-instruction PCs and instruction boundaries/relative PCs change (5 failure categories each). |
| C+8 | FAIL in both roles: 80 RIP referent-content mismatches per role; instruction boundaries, non-address bytes, tables and stack operands match. 132 / 133 displacement-only changes pass referent/call checks and are explicitly allowed. |

| Queue receipt | SHA-256 |
| --- | --- |
| request.json | `7404f3f421e9b12001ac91e882e988f2c786a47bc0cbfaa8e4d704c8dc93c7ed` |
| builds.json | `5972cec36027e924769d7b8afa2d880604b2a6f5827a1d6cab0d65b8a841bf93` |
| comparisons.json | `2871222e1ce92e764c0d758c6380c6fe48a486530434a8dde0d4210dee986cad` |
| result.json | `aadf9c7adea2006531cd0ce258c7e2049b8f55d2dce7a406bc094bcccb1450e2` |

Each role owns five decoded tables. Base Vm/Wide extents are 13,792/13,696 B;
H grows them to 13,808/13,712 B, while C retains the base extents. Every
variant retains the 0x178 entry stack reserve and the same ordered stack
operations/operands; H's stack report fails because their relative PCs move,
not because a new spill/frame-layout change was demonstrated.
C's first mismatch is Vm+0x2b80 / Wide+0x2ad4: an address-only LEA of metadata
with a checked span of 35,360 B in both images. Its first eight bytes change
`faf7500001000000`→`0af8500001000000`. Thus this failure already exists at
byte0; it is not caused solely by conservatively checking distant data.
The 80 failures per role are address-span LEA referents, not rejected numeric
literal displacement changes. This qualification detects current coupling;
it does not establish that outlining or a future K1 would pass.

## Feedback 1: amended qualification (superseded contract, retained evidence)

The lead supplied the pointer comparison amendment and a macOS temporary-path
failure. `admit_host` now resolves both BENCH_OUT and output. Its test creates
its own physical/alias directories and controls environment and platform;
it does not rely on caller TMPDIR or BENCH_OUT. Original 34 focused tests and
696 benchmark-tool tests pass in both a minimal environment and a deliberate
`/var/folders/.../T/.../alias` TMPDIR resolving through `/private/var/folders`.
The caller BENCH_OUT is deliberately invalid in the alias run. The complete
suite exposed two adjacent fixture dependencies: preview expected an unresolved
error path, and the mock compiler test accidentally invoked real Cargo against
a fake HOME. Those fixtures now resolve the expected path and provide mock
Cargo respectively; no production preview/cache behavior changed.

New synthetic cases cover moved identical targets (pass and reported allowed),
retargeting to a different symbol with identical bytes (fail), changed target
bytes, non-pointer changes, unresolved/partial slots, imports, symbol bounds,
cycles and depth exhaustion. Dyld stream cases cover opcode families, segment
bounds, lazy-state reset, unsigned backward additions, truncation/overflow,
resource bounds and unsupported chained/threaded forms. The final suite has
52 focused / 714 benchmark cases; minimal and symlink TMPDIR runs both pass.
The full local gate also passes 35 script tests and 5,170 Test262 subset cases.
Before every local Cargo invocation the disk guard checked available space;
`target/debug` was removed after the final full gate.

Fresh non-timing job `20261022T053712Z-t064-k0-amend1-main` uses producer
`5c18f47b` and fixed main `4fa3f739902bf4a78035e36522f4c2419f7bb45e`.
That main differs from the previous `9eba7a1d` only in T040 documentation;
no source, Cargo or recipe input changes. It ran at 2026-10-22T05:37:14Z,
finished all four clean builds in 426 s and returned 1 for contract differences;
no timing, cancellation, retry or attempt debit. The four executable SHA-256
identities and layout witnesses exactly reproduce the initial table above.

After fetching, final checker 5c18f47b verified each image SHA, decoded all four
images again and produced byte-identical comparisons.json to the new queue.
The iMac's cctools 1001.2 / LLVM 14 decoder and local cctools 1040 / LLVM 21 decoder
therefore agree; their exact identities are recorded above.

| Amended comparison | Vm / WideLoopFrame |
| --- | --- |
| A/A | PASS; full executable byte equality and complete body equality repeat. |
| H+8 | FAIL; same five categories, including instruction boundaries/relative PCs. Positive control survives the amendment. |
| C+8 | FAIL; 80/80 original mismatches remain in each role: 77 recursive target-content differences, 3 direct non-pointer differences. 132 / 133 direct displacement changes remain allowed. |

Both roles reference the exact same 80 failing address/width pairs. Every
pointer identity, target-node inventory and graph edge matches across C+8.
The pointer-address changes themselves no longer determine the failures:

- **77 referents:** the immediate non-pointer bytes match, but recursively
  reachable symbol-owned function bytes differ. Across the 80 referents,
  173 distinct __TEXT/__text target extents differ; all changes are explained
  by 649 RIP-disp32, 286 E8-call and 48 E9-tail-branch fields, with unchanged
  instruction boundaries. Examples include `core::fmt::Debug` formatters and
  `drop_in_place<Vec<TypedOp>>`. This is diagnostic attribution, not a relaxed
  comparison: the amendment keeps non-pointer target bytes raw, including
  these 4-byte instruction operands. The checker consequently rejects them.
- **3 referents:** Vm+0x3104/+0x3110/+0x311c and Wide+0x30a5/+0x30b1/+0x30bd additionally
  cover raw panic-location metadata via the anonymous LEA fallback. All 63
  changed non-pointer bytes belong to 62 location records for
  `crates/qjs-runtime/src/value/object.rs`; each u32 line number increases by 1
  (for example 777→778 and 743→744), with unchanged filename length/column.
  These ranges also share the recursive code-target changes.

The first failing metadata LEA (Vm+0x2b80 / Wide+0x2ad4) illustrates the closure:
35,360 B anonymous span, 625 reachable nodes and 1,707 pointer occurrences;
its immediate normalized bytes and all pointer identities match, while 166
reachable function targets change bytes. No known containing data symbol was
available for these anonymous spans; symbol-bounded LEAs are covered by the
new focused fixture. Bounds were not tuned to make this finite matrix pass.

Thus the amendment allows a moved pointer with unchanged identity/content
(the positive synthetic fixture reports `referent-pointer-relocation`), but
does not make this unoutlined C+8 artifact isolated. Further treatment of
instruction-address operands inside recursively referenced code, or tighter
ownership of anonymous metadata, needs a new lead decision before K1.

| Amended queue receipt | SHA-256 |
| --- | --- |
| request.json | `d0eb495c87e20fb1c957742504529bfcff24b2f3c48690ad06ba0040265c6315` |
| builds.json | `5972cec36027e924769d7b8afa2d880604b2a6f5827a1d6cab0d65b8a841bf93` |
| comparisons.json (queue and final replay) | `01fc8fe8135eb55ccdfd19b5a15b5ddca19375bba5d817041d5b33033ef190ad` |
| result.json | `70e79e9771374d2479c21195269d80403eebd11991b5c142e7c7e3ae8fa57b73` |

## Feedback 2: final single-run qualification

The lead classified the previous two C+8 failure classes as checker artifacts
and authorized amendment 2 in T064's lead decision. Code outside the protected
body inventory now compares by symbol/import identity plus symbol-relative
offset. No bytes of those functions participate in referent comparison.
Protected code remains fully compared; data keeps depth 2. Missing/ambiguous
classification fails closed, including imports without a proven code stub.
Inline-data records cannot become identity-only code leaves. Schema 3 prevents
mixing this contract with either earlier fingerprint schema.

H+8/C+8 fields and constructor initializers now append on existing lines.
The transform validates both input files before writing either: file line
counts match, and removing only the two declared inline insertions restores
every pre-existing line at its original number, including its line ending.
R0 also records this witness. builds.json retains per-file before/after line
counts and the original SHA. The separate test-only layout probe is removed
before release builds; no panic-location or line-number exception exists.

61 focused tests pass before the final tooling commit. Added cases include
code-pointer retargeting to another function (fail), protected-code byte
changes (fail), changed external-code bytes (pass), code offsets, inline data,
unknown classification, line insertion/movement, CRLF and a final line without
a newline. Development replay of the previous binaries passes A/A and removes
77/80 C+8 failures per role; the three old source-line artifacts remain raw.
This replay is not the required new current-main qualification. Full/fast
gates now pass 723 benchmark-tool cases, 35 script tests and 5,170 subset cases.
The disk guard ran before every local Cargo invocation; target/debug was
removed after verification. The actual source transform checks 1,897 unchanged
lines in object.rs and 184 in ordered_data_builder.rs for both H/C probes.

The implementation is frozen at `1d12a973`. The sole final non-timing queue
job is `20261022T060712Z-t064-k0-amend2-final`, using fixed main
`4fa3f739902bf4a78035e36522f4c2419f7bb45e`. It started at
2026-10-22T06:07:13Z, completed all four clean builds in 446 s and returned 1
solely because H+8 is a failing positive control. No retry, cancellation,
measurement timing, performance attempt or post-run checker edit occurred.
The canonical recipe/toolchain identities exactly match feedback 1 above.

| Final comparison | Vm | WideLoopFrame |
| --- | --- | --- |
| A/A | PASS, 0 failures | PASS, 0 failures |
| H+8 | FAIL, 5 categories including relative-PC drift | FAIL, 5 categories including relative-PC drift |
| C+8 | PASS, 0 residuals; 212 allowed | PASS, 0 residuals; 213 allowed |

C+8's allowed changes are 132/133 instruction-displacement changes plus 80
referent-pointer-relocation changes per role. All 80/80 previous C+8 failures
are eliminated. Source witnesses independently rechecked after fetching prove
1,897/184 original lines preserved for all four archives; no line-number or
panic-location exception was added to comparison. Layout witnesses remain
R0=112/400 B, H=120/400 B, C=112/408 B for ObjectData/ObjectColdData.
The complete function extents/instruction counts match prior controls:
Vm 13,792 B/2937 and Wide 13,696 B/2920 for R0/C; H grows extents by 16 B in both.

| Final canonical image | SHA-256 |
| --- | --- |
| r0a/r0b | `f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb` |
| h8 | `2db22bd91aaab5125d11d618e82694f1a3cf754f1c6bb79fec380db3cc3414f0` |
| c8 | `556f27e45819735eb9459cf32a895ba01a14471c611dd952f4f568f841422277` |

After fetching, unmodified `1d12a973` rechecked all image/source SHA values,
validated the line witnesses against the pinned source revision and decoded
all four images. Its comparison JSON is byte-identical to the iMac result.
The lead's amendment 2 supersedes the earlier interpretation of the 80 C+8
mismatches as kernel coupling: those were checker/perturbation artifacts.
H+8 still demonstrates actual instruction-PC coupling in unoutlined main;
this finite result does not qualify a future K1 runtime change.

| Final queue receipt | SHA-256 |
| --- | --- |
| request.json | `1624ee7d1de286c6ede6c8a89da77898b56ba2bad509743863188e2a3067b380` |
| builds.json | `7e2b181206baec2f1b56182b4a8fb8f9319a17d7791ceea367268dd549a4649e` |
| comparisons.json (queue and final replay) | `7724512692fceaced95798ec388cdaa778d3d0a3e2e8d51ea4f19be00a484490` |
| result.json | `937d834c8cbcbd7f159aada31a1e7811f9ea85ea8e427582d9c8f99b3968ae13` |
