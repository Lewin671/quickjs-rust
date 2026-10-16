# T047: Adopt the measured standalone CLI compiler recipe

- Status: closed-landed.
- Implementation: present in the release and performance-tool build recipes.
- Verified at: `6f571d27ba6fface159f379d5ca59f7243635a10`, 2026-10-16,
  macOS x86_64; finalization changes only this task record and its index.
- Evidence: [T046](T046-compiler-configuration-study.md) for the
  same-source measurements; adoption checks and identity below.
- Unresolved: cross-platform performance and stale linker order symbols remain
  outside this compiler-recipe adoption.
- Next action: none for adoption; cross-platform measurements and order-file
  repair remain separate work.

## Scope

Adopt optimization level 3, fat LTO, 16 codegen units and panic abort in the
workspace release profile. Preserve the measured Rust flag order and the
target-default CPU. The existing release workflow already builds this profile
with `agents` and an explicit target. Hosted previews now force the same
profile, features and ordered flags for both revisions, with provenance derived
from the builder recipe so its cache key and receipts describe the new build.
Formal comparisons and screens also build both revisions through that recipe,
use a fresh revision-cache namespace and derive current host recipes rather
than relabelling binaries with the historical manifest recipe. The hosted
implementation checksum is refreshed after reviewing these build changes.
No native CPU flags, PGO profiles, engine source or order-file changes.

Embedding applications own their workspace Cargo profiles; the profile here
controls this repository's standalone builds. Rust panics terminate the CLI;
JavaScript exceptions retain normal engine handling. Cargo tests retain their
normal test-profile panic behavior.

Historical manifests and sealed performance plans are unchanged. Their old
recipe is a distinct series; new measurements require matching new receipts.
[T046](T046-compiler-configuration-study.md) establishes performance on
macOS x86_64 only. Other supported targets adopt portable compiler settings,
without a claim of identical gains. Order-file resolution and cross-platform
performance remain separate verification work.

## Adoption verification

At the revision above, `./scripts/check.sh` and
`./scripts/check-touched.sh --base 163be1d8 --explain` both passed, including
412 benchmark-tool tests. The exact optimized CLI passed all 5170 curated
Test262 cases and every QuickJS-NG comparison fixture under `TZ=UTC`, `LC_ALL=C`.
Host-manifest generation also passed with the new recipe and profile identity.

The normal release command and the unified performance-tool builder both
produce SHA-256
`926ee634ca3901b89c778478702a4fe993288ef7e8e69cd93c238c694595aa85`,
identical to the measured T046 recommendation on this host/toolchain. This
establishes adoption of that binary; no new speedup is credited or extrapolated
to other platforms. The identity check reused Cargo artifacts; it is not a
clean compilation-time benchmark.

Explicit overrides of otherwise default debug/strip/incremental settings
initially produced another binary. They were removed; only the four measured
profile settings remain forced by the performance tools. The implementation
checksum and provenance describe this final recipe, not that discarded build.

The inherited host timezone produces a `date-basic.js` boundary mismatch
against NG. The original T046 release reference and the intermediate optimized
binary give identical output there; the release-default optimized binary also
exhibited the mismatch. UTC comparisons pass. No Date semantics were changed
or claimed fixed by this build-configuration task.

Local artifacts, not retained in the repository: workspace
`outputs/imac-bench/20261016T090636Z-compiler-adoption-exact-recipe/` contains the
successful combined log and binary identity JSON; the date control is in
`outputs/imac-bench/20261016T090210Z-compiler-adoption-date-check/` and the
original release-default build/hash and timezone mismatch are in
`outputs/imac-bench/20261016T085238Z-compiler-adoption-gates/`.
