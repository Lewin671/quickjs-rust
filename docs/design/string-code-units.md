# String code-unit authority

The target is one immutable logical UTF-16 sequence shared by String APIs,
RegExp and string keys. Preserve `JsString` reference sharing and copy on
write while removing repeated whole-subject copies and prefix decoding.
[T057](../../tasks/T057-string-code-units.md) owns implementation status,
revision-bound evidence and decisions. This design does not establish a
performance result or freeze a performance plan.

## Invariants

- JavaScript lengths, positions, captures and keys count UTF-16 code units.
  A scalar above U+FFFF occupies two units; a lone surrogate occupies one.
- The current UTF-8 container represents lone surrogates with U+F0000..F07FF
  sentinels and escapes actual scalars in that range. Stored text is canonical:
  a high-surrogate sentinel immediately followed by a low-surrogate sentinel
  is merged to one scalar unless that scalar is in U+F0000..F07FF. This makes
  byte equality and hashing agree with UTF-16 sequence equality.
  `qjs_unicode::wtf16` owns the shared lexer/runtime append and validation
  helpers. Appending canonical fragments checks only their seam; code-unit
  builders merge as they emit. `JsString::new` debug-asserts the invariant.
  Plain `encode_utf16` on sentinel text is not a unit decoder.
- Typed internal coercion returns a shared `JsString`. Keep ToPrimitive,
  GetMethod, species, custom exec, callback and lastIndex observation order.
  Share immutable data; never cache an observable property lookup.
- Cached ASCII classification makes byte/unit/scalar coordinates identical
  only on the ASCII branch. Non-ASCII literal search uses canonical byte
  matching for nonempty sentinel-free needles, with UTF-16 offset conversion.
  Sentinel-bearing needles use temporary unit buffers and KMP so they can
  match inside stored pairs; empty needles retain unit boundaries. Split and
  replacement matching advance through disjoint matches without rescanning
  prefixes. No per-string unit view is introduced by this correctness fix.
- Mutation invalidates dependent metadata, or explicitly composes it from
  the appended suffix. Aliases retain their old value and caches. Preserve
  the existing unique append path and slot/shared-cell binding model.
- Parser/lexer source spans remain UTF-8 byte offsets. Host UTF-8 conversion
  is an explicit boundary; it must preserve the sentinel round trip.

## Historical T057 decomposition

The remaining structural storage target is now [T077 S7](../../tasks/T077-core-rebuild.md#s7--primary-latin-1utf-16-strings);
its primary-unit design supersedes the lazy-view route below. For separate
T057 semantic/local work, each unit still needs fresh evidence and a validated committed
plan before runtime edits, focused correctness/work-count tests and the
[performance workflow](../performance-workflow.md). Stages can have separate
plans; the six rows do not themselves freeze a single six-stage budget.

| Stage | Ownership and result | Required discriminators |
| --- | --- | --- |
| 1. Handles and ASCII boundaries | String/RegExp-local typed receivers/coercions; preserve subject handles across match, matchAll, custom exec, callback and iterator plumbing; reuse cached ASCII search classification. Keep wide indexing and storage unchanged. | Coercion hook order; boxed receiver sharing; matchAll subject sharing; every custom exec lookup; ASCII offset work independent of subject length; species and lastIndex. |
| 2. Unit views and String primitives | Lazy mode-independent compact view; migrate at/char/index properties, slices/search/reverse search, string split, substitution ranges and String iterator cursor. | All surrogate forms and real sentinel-range scalars; unit search indices and half-surrogate needles; empty split; one index build for random access; pair/lone iteration; half-pair slices. |
| 3. RegExp coordinates | Prepared input borrows unit storage; start/end/captures stay in unit coordinates in both modes. Unicode stepping locally decodes a pair; remove prefix conversions and mode eviction. | Arbitrary/interior-pair lastIndex; d/g/y/u/v; named indices; empty advance; alternating split views; lookbehind, reverse captures and backreferences. |
| 4. Builders and replacement bookkeeping | Compose append metadata; carry subject/match lengths through replacement preflight and assembly; retain one forward output cursor. | Append after cached reads, aliases and joined surrogate halves; one subject count in literal replaceAll; callback input sharing; prefix/suffix output and overflow. |
| 5. Primary 8/16-bit storage | Explicit cross-crate API migration to Latin-1/UTF-16 primary data; retire UTF-8-dependent Deref/Borrow contracts. | Latin-1 0..255; host round trips; scalar/pair equality and hash; COW; key enumeration, proxies and descriptors; byte source spans. |
| 6. Optional concat structure | Independently justified bounded-depth ropes for real alias/prepend costs, with iterative flatten/drop and cursors. | Old aliases unchanged; deep drop; one flatten; pairs across nodes; bounded tiny-slice retention; unique append control. |

Stages 1–4 stay local to String/RegExp. Primary storage now follows T077 S1–S6: atoms already use code-unit keys,
and the atom/string equality/hash backing must change together in S7.
No stage changes GC, frame ownership, property storage or the value word.

## Historical hybrid-view policy

This describes the superseded T057 hybrid proposal, not T077's primary storage.

Retain flat UTF-8 backing during the hybrid migration. ASCII APIs borrow
bytes after cached classification. Non-ASCII indexing lazily builds one
mode-independent Latin-1 `Rc<[u8]>` or wide `Rc<[u16]>` view; stream its
construction and replace the current four-byte-per-character matcher view
instead of keeping permanent mode-specific duplicates. Latin-1 eligibility
does not make UTF-8 byte offsets direct indices.

Prefer operations on units and encode only selected output. Where a UTF-8
mapping remains necessary, use sparse checkpoints and a forward cursor,
including an explicit half-scalar state; never index a continuation byte.
Do not allocate both a dense byte-offset table and dense units by default.
The checkpoint stride needs a separately justified implementation choice.

Unknown metadata may be established lazily once. Builders compute metadata
while already traversing output, without an extra eager pass for unused
literals. Do not add a global side map. Record view/cache bytes and peak RSS
for short strings and large documents when adding views. Primary storage
eventually removes permanent UTF-8 plus unit duplication; UTF-8 caches stay
cold. A future slice policy must bound retained parent size/ratio so tiny
tokens do not pin entire documents. Ropes remain optional and separately
measured; ordinary transforming output remains output-proportional work.

## Correctness boundary

String search positions, literal split and string-needle replacement use
UTF-16 units, including matching a lone surrogate inside a stored pair.
ASCII retains the byte paths. String-pattern search/match delegate to RegExp;
`at` returns one unit and String iteration returns code points.
[T057](../../tasks/T057-string-code-units.md) owns base reproductions and checks.
This local correctness repair does not implement the stage-2 view/cache plan.

RegExp's interior-pair coordinate behavior remains stage 3 work: at the
initial investigation, `/./uy` on `'😀x'` with lastIndex 1 yielded
index/length/new lastIndex [2,1,3] vs NG [1,1,2]. Reproduce and confirm against
ES2025 before changing matcher coordinates; this String fix makes no claim
about that independent boundary.

## Verification contract

Cross ASCII, Latin-1, CJK, astral, lone high/low surrogates, sentinel pairs
and actual U+F0000..F07FF. Cover negative/NaN/Infinity/clamped bounds,
empty matches, captures, coercion/callback mutation, aliases and invalidation.
Use deterministic scan/build counts rather than timing assertions.
Curate upstream String methods, StringIteratorPrototype and RegExp
exec/test/symbol methods through the harness; add NG comparison fixtures.
Run the focused gates and full `check.sh`; promotion additionally needs
the full configured zero-gap Test262 record and held-out acceptance.
Development programs may select/profile work; held-out programs never do.
