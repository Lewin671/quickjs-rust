//! Work-count obligations of the property layout authority (T056 M1).
//!
//! Each test runs one workload at two repetition counts and compares the
//! counters. Realm setup is identical in both runs, so the difference is the
//! cost of the extra repetitions: repeated construction must create no
//! shapes, copy no fields and intern no atoms after the first object, and no
//! object, function, array or JSON value may create an atom domain.
//! Gated on `perf-counters` like the other counter tests; the name keeps it
//! inside the `tests::diagnostics` filter the full gate runs.

#![cfg(feature = "perf-counters")]

use crate::{Counters, counters, eval, reset_counters};

const FEW: u32 = 20;
const MANY: u32 = 220;

fn counted(source: &str) -> Counters {
    reset_counters();
    eval(source).unwrap_or_else(|error| panic!("{source}: {error:?}"));
    counters().expect("perf-counters build exposes counters")
}

/// Counters of `workload(n)` for the two repetition counts.
fn repeated(workload: impl Fn(u32) -> String) -> (Counters, Counters) {
    (counted(&workload(FEW)), counted(&workload(MANY)))
}

/// Steady-state construction allocates no layout state per object.
fn assert_constant_layout_work(name: &str, few: &Counters, many: &Counters) {
    let pairs = [
        (
            "shapes",
            few.property_shapes_created,
            many.property_shapes_created,
        ),
        (
            "field copies",
            few.property_shape_fields_copied,
            many.property_shape_fields_copied,
        ),
        (
            "atoms",
            few.property_atoms_created,
            many.property_atoms_created,
        ),
        (
            "interns",
            few.property_atom_interns,
            many.property_atom_interns,
        ),
        (
            "domains",
            few.property_domains_created,
            many.property_domains_created,
        ),
        (
            "field-vector compares",
            few.property_field_vector_compares,
            many.property_field_vector_compares,
        ),
        (
            "dictionaries",
            few.property_dictionaries_created,
            many.property_dictionaries_created,
        ),
    ];
    for (what, few, many) in pairs {
        assert_eq!(few, many, "{name}: {what} grew with repetitions");
    }
}

fn loop_of(n: u32, setup: &str, body: &str) -> String {
    format!("{setup}\nvar sink;\nfor (var i = 0; i < {n}; i++) {{ sink = {body}; }}\nsink;")
}

#[test]
fn repeated_construction_adds_no_layout_work_after_the_first_object() {
    let cases = [
        (
            "constructor",
            "function P(a, b, c) { this.a = a; this.b = b; this.c = c; }",
            "new P(i, i, i)",
        ),
        (
            "class fields",
            "class C { a = 1; b = 2; c = 3; constructor(x) { this.d = x; } }",
            "new C(i)",
        ),
        (
            "JSON shape",
            "var text = '{\"a\":1,\"b\":[1,2],\"c\":{\"d\":true,\"e\":null},\"f\":\"x\"}';",
            "JSON.parse(text)",
        ),
        ("literal", "", "({ a: i, b: i, c: i, d: i })"),
        ("literal pair", "", "({ x: i, y: i })"),
        (
            "host map",
            "var source = [1, 2];",
            "source[Symbol.iterator]().next()",
        ),
        (
            "host descriptor",
            "var o = { v: 1 };",
            "Object.getOwnPropertyDescriptor(o, 'v')",
        ),
        (
            "grown past the small limit",
            "function grow() { var o = {}; for (var k = 0; k < 20; k++) o['k' + k] = k; return o; }",
            "grow()",
        ),
        (
            "literal pair grown",
            "function more(v) { var o = { x: v, y: v }; o.z = v; return o; }",
            "more(i)",
        ),
        (
            "delete and re-add",
            "function churn(v) { var o = { a: v, b: v, c: v }; delete o.b; o.b = v; return o; }",
            "churn(i)",
        ),
        (
            "bulk freeze",
            "function frozen(v) { return Object.freeze({ a: v, b: v, c: v, d: v }); }",
            "frozen(i)",
        ),
        (
            "seal and reconfigure",
            "function sealed(v) { var o = { a: v, b: v }; Object.defineProperty(o, 'a', { enumerable: false }); return Object.seal(o); }",
            "sealed(i)",
        ),
    ];
    for (name, setup, body) in cases {
        let (few, many) = repeated(|n| loop_of(n, setup, body));
        assert_constant_layout_work(name, &few, &many);
    }
}

#[test]
fn wrapper_function_array_and_regexp_result_creation_create_no_domain() {
    let cases = [
        ("string wrapper", "new String('ab')"),
        ("number wrapper", "Object(i)"),
        ("closure", "(function f() { return i; })"),
        ("closure prototype", "(function g() {}).prototype"),
        ("function property", "(function h() {}).tag = i"),
        ("array", "[i, i + 1]"),
        (
            "array named property",
            "(function () { var a = [i]; a.tag = i; return a; })()",
        ),
        ("regexp result", "/b(c)?/.exec('abc')"),
        ("arguments", "(function () { return arguments; })(i)"),
    ];
    for (name, body) in cases {
        let (few, many) = repeated(|n| loop_of(n, "", body));
        assert_eq!(
            few.property_domains_created, many.property_domains_created,
            "{name}: creation made an atom domain"
        );
        assert_eq!(
            few.property_atoms_created, many.property_atoms_created,
            "{name}: creation made atoms"
        );
        assert_eq!(
            few.property_shapes_created, many.property_shapes_created,
            "{name}: creation made shapes"
        );
    }
}

#[test]
fn an_add_probes_its_store_once() {
    // A null-prototype receiver has no inherited setter to look up, so every
    // probe counted here is the store's own.
    let adds = |n: u32| {
        format!(
            "var o = Object.create(null);
             for (var i = 0; i < {n}; i++) {{ o['k' + i] = i; }}
             var d = Object.create(null);
             for (var j = 0; j < {n}; j++) {{
                 Object.defineProperty(d, 'p' + j, {{ value: j, writable: true, enumerable: true, configurable: true }});
             }}
             0;"
        )
    };
    let (few, many) = repeated(adds);
    let added = u64::from(2 * (MANY - FEW));
    assert!(
        many.property_add_probes - few.property_add_probes <= added,
        "{} add probes for {added} adds",
        many.property_add_probes - few.property_add_probes
    );
}

#[test]
fn small_shape_lookups_hash_nothing_and_large_ones_hash_once() {
    let reads = |fields: u32| {
        move |n: u32| {
            format!(
                // Locals, so the large global object's hashed lookups do not count.
                "(function () {{
                     var o = {{}};
                     for (var f = 0; f < {fields}; f++) o['f' + f] = f;
                     var keys = []; for (var f = 0; f < {fields}; f++) keys.push('f' + f);
                     var s = 0;
                     for (var i = 0; i < {n}; i++) {{ s += o[keys[i % {fields}]]; }}
                     return s;
                 }})();"
            )
        }
    };
    let (few, many) = repeated(reads(6));
    assert_eq!(
        few.property_lookup_hashes, many.property_lookup_hashes,
        "a six-field shape hashed a probe"
    );
    let (few, many) = repeated(reads(40));
    let probes = u64::from(MANY - FEW);
    let hashes = many.property_lookup_hashes - few.property_lookup_hashes;
    assert!(
        hashes <= probes,
        "{hashes} hashes for {probes} large-shape probes"
    );
}
