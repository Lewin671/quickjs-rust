# Core rebuild: primary string units

Target for [core rebuild](core-rebuild.md), S7. Keep the semantic discriminators
and shared-subject work in [string code units](string-code-units.md). T057's
landed fixes remain; this is its structural storage successor, not a revival of
its abandoned view attempt or budget. Parser/lexer source stays UTF-8 with byte spans.

## Representation and APIs

Replace `value/js_string.rs::StringData { text:String, ascii, utf16_len,
matcher_view, ... }` with a counted String payload containing
`UnitStorage = Latin1(Vec<u8>) | Wide(Vec<u16>)` (32-byte budget on 64-bit),
plus flags/padding (8): **40 bytes**, plus S5's header and external buffer.
Vec supplies usize length/capacity; checked JS length limits apply before publish.
Assert actual layouts on both targets. This deliberately uses safe storage;
a raw 24-byte tagged buffer is not justified merely to save header bytes.
Latin-1 uses bytes 0..255, not ASCII/UTF-8. UTF-16 stores exact u16 units,
including isolated surrogates. No permanent duplicate UTF-8 or char-index buffer.

`JsString` remains one owning handle. `units() -> Units<'_>` supplies borrowed
Latin1/Wide views; `unit_at`, `len_units`, `slice_units`, `find_units`, `hash_units`
and `to_utf8_lossy` expose semantic intent. `code_point_at` and an iterator decode
a neighboring surrogate pair locally. Code-unit comparison/hashing agrees across
encodings, including Latin-1 upgraded to Wide. Equality never depends on how a
pair or a sentinel-range scalar originally entered the runtime.

An unpublished builder may grow in place; unique append preserves COW and
upgrades once to wide on demand. Shared aliases retain their old storage.
Interned keys never mutate. Remove UTF-8 Deref/Borrow/as_str assumptions from
runtime strings and migrate host callers explicitly; do not return replacement
UTF-8 and pretend it preserves lone surrogates. Host UTF-8 scalar text and an
explicit lossless code-unit transport are separate APIs. Original source text
for Function.prototype.toString remains independently owned UTF-8 source.

## Atoms, RegExp and storage deletion

KeyRecords from S1/S2 retain identity when backing changes; atom equality/hash
uses units everywhere in the same commit. Delete sentinel canonicalization from
runtime key storage only after every ingress/egress and comparison migrates.
The shared qjs-unicode source parser helpers may still represent source literals;
convert at the runtime boundary, preserving byte source spans.

RegExp prepared subjects borrow the same primary units in both modes. Match
starts/ends/captures/lastIndex stay in UTF-16 coordinates; Unicode stepping decodes
pairs without normalizing an interior-pair start to a different coordinate.
Port lookbehind/reverse scans/backreferences and d/g/y/u/v together. Delete
mode-specific `Rc<[char]>` caches, prefix byte/unit scans and permanent unit views.
Keep custom exec, species, coercion, replacement callback and lastIndex lookup
order; sharing a subject never caches an observable method/property result.

No ropes or retained-parent slices in S7. Their ownership/depth/flatten/retention
tradeoffs need independent evidence after flat strings work. Tiny substrings own
bounded output storage, not arbitrarily large parents. Builders produce output
in one forward traversal where the algorithm permits.

## Validation

Cross ASCII, Latin-1, CJK, astral pairs, lone high/low surrogates and actual
U+F0000..F07FF. Test half-pair slicing/search, empty matches, interior-pair
lastIndex, reverse captures, string keys in all object families, alias append,
hash/Map/Set equality, host round trips and both storage encodings.
Compare NG behavior and ES2025 where they differ; do not import living-draft
features to repair a discrepancy. Use deterministic work counts for indexing,
search, replaceAll and repeated matching, then the complete stage correctness
and aggregate wall gates. Report startup/compile, allocation/live bytes and RSS.
