# T077 S1 foundation evidence (implementation milestone, not stage acceptance)

Base `dd0905b2`; sole owner covers all brief paths on the S1 branch. Disk
preflight 49 GiB available; after debug feature builds 39 GiB remains.
Source inventory: 399 property-store/revision/read-write matches across 31
files; 220 object/builder constructor calls across 88 files. Mutation entries
are `set[_shared_key]`, `create_absent_own_data_property`,
`define[_shared]_property`, `define_symbol_property`, `set_internal_non_enumerable`,
`write_existing_own_data_property[_if]`, slot writes, `append_string_property`,
`delete`, `delete_symbol`, seal/freeze, lazy String indices and constant-class
field installation. Construction also includes ordered JSON data, literals,
realm/intrinsic setup, wrappers, class instances, host/test helpers and teardown.
The next cutover must cover each; none is currently migrated.

Foundation APIs are frozen in the linked brief. One weak string/symbol/ID
namespace replaces string-only key identity. New layout code is isolated:
no production receiver consults it yet, so no receiver has dual authorities.
Type assertions on this Mac: KeyId 4 B; KeyLease 8 B; ShapeEntry 16 B;
NamedStore 48 B; RefCell<NamedStore> 56 B. Other-target sizes remain unverified.
No old property storage has been deleted yet, no iMac job was submitted, and
there is no new full-scan/burndown or performance standing. No design correction.

Commands so far: `cargo test -p qjs-runtime value::layout --lib --quiet`
12/12; `cargo test -p qjs-runtime --features perf-counters value::layout --lib --quiet`
13/13 (includes deterministic one-probe/zero-copy/zero-intern append-hit proof);
`cargo test -p qjs-runtime --features agents,perf-counters property::atom --lib --quiet`
12/12; `cargo clippy -p qjs-runtime --all-targets -- -D warnings` passed.
`TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check-touched.sh --staged --explain`
passed with 2,396 runtime tests, context/file-size/format/clippy clean; no focused
Test262 slice matched. Final combined-feature layout discriminators: `cargo test -p qjs-runtime
--features agents,perf-counters value::layout --lib --quiet` 17/17 pass, including
symbol order, epoch exhaustion, the exact dictionary-width boundary and
one shared cache owner per live domain. The final fast gate rechecks all source.
Full stage gates pending.

