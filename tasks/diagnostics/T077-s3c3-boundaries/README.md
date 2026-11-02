# C3 explicit binding boundaries and indexed cells

This is a semantic cut; C3 remains incomplete until its method instruction
misses, 109-case corpus and single runner checkpoint are closed. The commit
containing this receipt pins runtime source and binaries in [builds](builds.json).
Parent is ae11f20e; comparison milestone remains 8322fc98.

Compilation inserts BindingBoundary before scope/reference start/read/write and
cleanup points, after canonicalizing empty scope records with no declarations,
with object or distinct variable target. Only edges crossing a meaningful scope
or leaving a live reference interval need extra targets. One relocation updates
branches, catch/finally entries, prologue/exposure PCs, reference intervals/events
and every existing PC-bearing Op. Tier maps/CFGs are built from the final stream.
ReferenceEvent.pc identifies the actual source operation; its preceding boundary
carries the event index. No source byte span is changed.

The general checking loop no longer polls pc_scopes/events/lifetimes before
ordinary operations. Explicit boundaries synchronize scopes, select/read/write
references and prune only at cleanup edges. A promotion before an event restarts
at its boundary; completed effects restart after it, never replaying selection.
Wide reference start/cleanup is one cold boundary operation. Existing tier
admission proofs still govern elision and deferred global publication.

Own non-global, non-eval-source Var/Lexical/Catch/Function cells get compiled read
routes; actual cells stay in local_upvalues. Parameters/arguments joins, imports,
received/dynamic names and selected global roots retain their existing routing.
General initialized mutable cells link at preparation/prologue/promotion; the
mask keeps the same cells and excludes deletable/eval-source declarations.
No value cache, name snapshot, unsafe operation or global-store shortcut is added.

Verification at this runtime source:

- default runtime **2,740**, new relocation/cleanup/yield/scope/TDZ tests pass;
- counter test proves general Var/Parameter fast writes and indexed own reads;
- full gate passes, including **5,174** curated Test262; x86_64 check passes;
- [stress](stress.json): seven modes **2,740** each, combined-off **2,698** with
  the existing 42 IC-control skips;
- [NG](compare.json): **322 + 10 modules** in each of nine modes;
- global-store fuzz **2,000**: **1,933 MATCH / 67 PRE**, stable inputs, no new
  regression or mode failure. PRE remains a visible base/NG difference;
- [depth](depth.json): debug/release all >=S3b. Debug reduce **243 vs 236**,
  call **673 vs 437**; plain recursion reaches the 4,096 cap in both profiles.

[Call slopes](call-cost.json) still save **87.1 / 170.9 / 403.5** instructions/call
in upw0/1/4. [Controls](controls.json): fannkuch 0.999755, tofte 0.918086,
supplied method repro 0.976772 pass; hash-map **1.005015** and broad/method_call
**1.010097** remain required fixes. No C3 acceptance or runner job is claimed.
