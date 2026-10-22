# T064 K0 static checker

- Status: implementation present; current-main qualification pending.
- Implementation: maintained Python checker and archive-only H+8/C+8 transforms;
  `scripts/check-kernel-isolation.sh`; no runtime/Cargo/recipe/policy change, no CI hook.
- Verified at: branch based on main `9eba7a1d`; 32 focused Python tests and the staged fast gate pass before first tooling commit (694 benchmark tests).
- Evidence: contract/method below; queued build identities/results will be recorded here.
- Unresolved: current-main A/A, H+8 and C+8 results and full local gate pending.
- Next action: queue the full non-timing canonical matrix, then record exact results and gates.

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
Allowed differences are individually reported, even when another check fails.

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
