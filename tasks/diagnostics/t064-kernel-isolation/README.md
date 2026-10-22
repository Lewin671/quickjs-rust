# T064 read-only allocation attribution

[Input record](../../archive/T064-kernel-isolation-inputs.md) owns conclusions;
[T064](../../archive/T064-kernel-isolation.md) records the closed outlining experiment. `allocation-attribution.json`
contains complete function-stack sites, non-overlapping family totals, exact image
and source hashes and the six source-stack JSON hashes from job222209Z.
No new runtime build/measurement/tool implementation is present.

For each native stack row, structurally decode its Rust-v0 frame names using
`tools.benchmark.layout_symbols.parse_symbol`; retain undecodable text verbatim.
Multiply count by request size and the bound host malloc_good_size. Sum every row
once. Site key is family + last two visible qjs-runtime frames + request size
(or the last two raw frames if no engine frame). Source-line PCs were not saved.

Family priority (first match in the complete stack):

1. ObjectColdData generic/type frame → cold data.
2. property::atom::AtomDomain plus intern* → atom interning.
3. property::layout plus first_lookups/build_index → layout text index.
4. Other property::layout → combined ordinary layout/payload/construction.
5. property::named_table → function/array named descriptors.
6. value::object::storage → the combined ordinary family.
7. value::object::descriptor_batch → host batching.
8. NamedPropertyCache/named_property_cache → named access cache.
9. StringData/JsString/string/regexp → remaining string/regexp.
10. ObjectRef/ObjectLiteralShape → the combined ordinary family.
11. Realm/CallEnv/function::env → remaining realm/environment.
12. function → remaining function;13. bytecode → remaining compiler/VM;14. else other.

These are stack partitions, not inferred exact Rust heap types. All complete
per-site count vectors are retained in `site_rows`; dictionaries decode family
and owner IDs, sizes map through `malloc_good_size`. Multiply counts by sizes
for requested bytes, and by class sizes for class bytes. Sum all sites to the
bound totals for each role/case; all six reconcile exactly. No inclusive frame
totals are added together; no top-N truncation or timing interpretation.
