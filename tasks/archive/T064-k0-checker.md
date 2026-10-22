# T064 K0 static checker

- Status: complete tooling qualification; current unoutlined main fails isolation as expected.
- Implementation: maintained Python checker and archive-only H+8/C+8 transforms;
  `scripts/check-kernel-isolation.sh`; no runtime/Cargo/recipe/policy change, no CI hook.
- Verified at: final checker `36ab76e3`; 34 focused tests, staged fast gate and full local `check.sh` pass (696 benchmark tests, 35 script tests, 5,170 Test262 subset cases).
- Evidence: contract/method and exact current-main identities/results below; raw queue artifacts are fetchable by job ID.
- Unresolved: K1 isolation is outside this tooling brief; LEA bounds remain deliberately conservative.
- Next action: lead reviews these static results with the fresh queue/profiles and freezes the successor plan before K1; CI remains prospective.

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
referents retain their full memory operand width and content hash; calls
retain exact local symbol/import identities, including closure/clone details.
RIP import slots also retain binding identities: equal zero pointer bytes cannot
hide retargeting to another import. Zero-fill/non-file-backed referents and
unresolved import slots fail closed. Allowed differences are individually
reported, even when another check fails.

LEA has no dereference width. For owned tables it protects the full table;
other address-only LEAs conservatively protect through the next nm data
boundary or section end. That can reject unrelated metadata movement; it
never certifies an opaque pointer by checking an arbitrary short prefix.
Pointer-containing referent bytes remain raw, as required by the contract.
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

## Current-main canonical qualification

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
