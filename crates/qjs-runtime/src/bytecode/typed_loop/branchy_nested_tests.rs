use qjs_ast::UnaryOp;

use super::{TypedOp, compile_all};
use crate::bytecode::{compiler, ir::Op};
use crate::{Value, eval};

fn nested_function(source: &str, name: &str) -> crate::bytecode::Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction {
                name: actual,
                bytecode,
                ..
            } if actual.as_deref() == Some(name) => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("named function bytecode should be nested in the script")
}

#[test]
fn truthy_short_circuit_branches_keep_the_original_condition() {
    let source = "function run(values, n, guard) { var total = 0; for (var i = 0; i < n; i++) { if (guard || i < 0) continue; total += values[i]; } return total; }";
    let bytecode = nested_function(source, "run");
    let programs = compile_all(&bytecode);
    assert_eq!(programs.len(), 1, "{:#?}", bytecode.code);
    assert!(programs[0].ops.iter().any(|op| matches!(
        op,
        TypedOp::Unary {
            op: UnaryOp::Not,
            ..
        }
    )));

    for (guard, expected) in [("0", 10.0), ("-0", 10.0), ("NaN", 10.0), ("1", 0.0)] {
        assert_eq!(
            eval(&format!("{source} run([1, 2, 3, 4], 4, {guard});")),
            Ok(Value::Number(expected)),
            "{guard}"
        );
    }
}

#[test]
fn nested_dense_reads_keep_intermediate_receivers_boxed() {
    let source = "function run(table, n) { var total = 0; for (var i = 0; i < n; i++) { total += table[i & 1][(i + 1) & 1]; } return total; }";
    let bytecode = nested_function(source, "run");
    let programs = compile_all(&bytecode);
    assert_eq!(programs.len(), 1, "{:#?}", bytecode.code);
    assert_eq!(
        programs[0]
            .ops
            .iter()
            .filter(|op| matches!(op, TypedOp::ElementRead { .. }))
            .count(),
        2
    );
    assert_eq!(
        eval(&format!("{source} run([[1, 2], [3, 4]], 4);")),
        Ok(Value::Number(10.0))
    );

    // A non-Array intermediate receiver deoptimizes at the second read and
    // lets the ordinary property path finish the same iteration.
    assert_eq!(
        eval(&format!("{source} run([[1, 2], {{ 0: 3, 1: 4 }}], 4);")),
        Ok(Value::Number(10.0))
    );
    // An indexed descriptor prevents the first direct dense read. Its getter
    // is observed exactly once per generic iteration after deoptimization.
    assert_eq!(
        eval(&format!(
            "{source} var table = [[1, 2], [3, 4]], hits = 0; Object.defineProperty(table, '1', {{ get: function () {{ hits++; return [3, 4]; }}, configurable: true }}); run(table, 4) + ':' + hits;"
        )),
        Ok(Value::String("10:2".to_owned().into()))
    );
}

#[test]
fn branchy_nested_dense_math_region_is_admitted() {
    let source = r#"
        function run(image, kernel, width, height, kernelSize) {
            var r = 0, g = 0, b = 0, a = 0;
            for (var y = 0; y < height; y++) {
                for (var x = 0; x < width; x++) {
                    for (var j = 1 - kernelSize; j < kernelSize; j++) {
                        if (y + j < 0 || y + j >= height) continue;
                        for (var i = 1 - kernelSize; i < kernelSize; i++) {
                            if (x + i < 0 || x + i >= width) continue;
                            r += image[4 * ((y + j) * width + (x + i)) + 0] * kernel[Math.abs(j)][Math.abs(i)];
                            g += image[4 * ((y + j) * width + (x + i)) + 1] * kernel[Math.abs(j)][Math.abs(i)];
                            b += image[4 * ((y + j) * width + (x + i)) + 2] * kernel[Math.abs(j)][Math.abs(i)];
                            a += image[4 * ((y + j) * width + (x + i)) + 3] * kernel[Math.abs(j)][Math.abs(i)];
                        }
                    }
                }
            }
            return r + g + b + a;
        }
    "#;
    let bytecode = nested_function(source, "run");
    let programs = compile_all(&bytecode);
    assert!(
        programs.iter().any(|program| {
            let element_reads = program
                .ops
                .iter()
                .filter(|op| matches!(op, TypedOp::ElementRead { .. }))
                .count();
            let dense_reads = program
                .ops
                .iter()
                .filter(|op| matches!(op, TypedOp::DenseRead { .. }))
                .count();
            let truthy_branches = program
                .ops
                .iter()
                .filter(|op| {
                    matches!(
                        op,
                        TypedOp::Unary {
                            op: UnaryOp::Not,
                            ..
                        }
                    )
                })
                .count();
            element_reads >= 8 && dense_reads >= 4 && truthy_branches >= 1
        }),
        "programs={programs:#?}\nbytecode={:#?}",
        bytecode.code
    );
    assert_eq!(
        eval(&format!(
            "{source} run([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], [[1, 2], [3, 4]], 2, 2, 2);"
        )),
        Ok(Value::Number(160.0))
    );
}

// The property-read site cache remembers up to four literal shapes, validated
// by shape identity plus the property revision. These pin what that must not
// change: a stale entry has to miss, never read a wrong slot.

#[test]
fn a_polymorphic_property_read_sees_each_shape_correctly() {
    let source = "function run(pool, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += pool[i % pool.length].step; }
            return total;
        }
        run([{ step: 1, a: 0 }, { a: 0, step: 10 }, { a: 0, b: 0, step: 100 }], 30);";
    assert_eq!(eval(source), Ok(Value::Number(1110.0)));
}

#[test]
fn a_shape_change_between_reads_invalidates_the_cached_slot() {
    // Adding a property moves the receiver to a new shape; the cached entry
    // must miss rather than read the old slot index.
    let source = "function run(o, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o.step; if (i === 4) { o.added = 1; } }
            return total;
        }
        run({ step: 3 }, 10);";
    assert_eq!(eval(source), Ok(Value::Number(30.0)));
}

#[test]
fn an_overwritten_property_is_read_at_its_new_value() {
    let source = "function run(o, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o.step; o.step = o.step + 1; }
            return total;
        }
        run({ step: 1, other: 0 }, 5);";
    assert_eq!(eval(source), Ok(Value::Number(15.0)));
}

#[test]
fn a_deleted_property_reads_undefined_rather_than_a_stale_slot() {
    let source = "function run(o, n) {
            var seen = 0;
            for (var i = 0; i < n; i++) {
                if (o.step === undefined) { seen++; }
                if (i === 2) { delete o.step; }
            }
            return seen;
        }
        run({ step: 1, other: 0 }, 6);";
    assert_eq!(eval(source), Ok(Value::Number(3.0)));
}

#[test]
fn a_megamorphic_property_read_stays_correct_past_the_cache_ways() {
    // Six shapes against four ways: the extra shapes must fall through to name
    // resolution and still read the right value.
    let source = "var pool = [
            { step: 1, a: 0 }, { a: 0, step: 2 }, { b: 0, step: 3 },
            { c: 0, d: 0, step: 4 }, { e: 0, step: 5 }, { f: 0, g: 0, h: 0, step: 6 }
        ];
        function run(pool, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += pool[i % 6].step; }
            return total;
        }
        run(pool, 12);";
    assert_eq!(eval(source), Ok(Value::Number(42.0)));
}

#[test]
fn an_accessor_shadowing_a_cached_shape_is_not_answered_from_the_cache() {
    let source = "function run(pool, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += pool[i % pool.length].step; }
            return total;
        }
        var plain = { step: 1, a: 0 };
        var withGetter = { a: 0 };
        Object.defineProperty(withGetter, 'step', { get: function () { return 10; } });
        run([plain, withGetter], 10);";
    assert_eq!(eval(source), Ok(Value::Number(55.0)));
}

// Computed string-key access inside a loop region. Three separate changes make
// this admissible -- a computed read/write that discriminates instead of
// assuming array semantics, a join that widens a scalar to meet a boxed value,
// and a seed that lets a boxed register hold any value -- and each has its own
// way of going wrong.

#[test]
fn a_dictionary_churn_loop_computes_the_same_totals() {
    let source = "function churn(keys, n) {
            var table = {};
            for (var i = 0; i < n; i++) {
                var key = keys[i % keys.length];
                table[key] = (table[key] || 0) + 1;
            }
            var total = 0;
            for (var name in table) { total += table[name]; }
            return total + ':' + table.a + ',' + table.b + ',' + table.c;
        }
        churn(['a', 'b', 'c'], 30);";
    assert_eq!(eval(source), Ok(Value::String("30:10,10,10".into())));
}

#[test]
fn a_computed_write_creates_then_overwrites() {
    // The first write to each key creates the property and every later one
    // overwrites it. Refusing creation would deoptimize on iteration one and
    // never re-enter, losing the loop to its own first write.
    let source = "function run(n) {
            var t = {};
            for (var i = 0; i < n; i++) { t['k' + (i % 2)] = i; }
            return t.k0 + ',' + t.k1 + ',' + Object.keys(t).length;
        }
        run(10);";
    assert_eq!(eval(source), Ok(Value::String("8,9,2".into())));
}

#[test]
fn a_computed_access_still_walks_the_prototype_chain() {
    let source = "function run(o, keys, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o[keys[i % keys.length]]; }
            return total;
        }
        var proto = { inherited: 5 };
        var o = Object.create(proto);
        o.own = 1;
        run(o, ['own', 'inherited'], 10);";
    assert_eq!(eval(source), Ok(Value::Number(30.0)));
}

#[test]
fn a_computed_access_declines_an_accessor_rather_than_reading_a_slot() {
    let source = "function run(o, keys, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o[keys[i % keys.length]]; }
            return total;
        }
        var calls = 0;
        var o = { plain: 1 };
        Object.defineProperty(o, 'lazy', { get: function () { calls++; return 10; } });
        run(o, ['plain', 'lazy'], 10) + ':' + calls;";
    assert_eq!(eval(source), Ok(Value::String("55:5".into())));
}

#[test]
fn a_computed_write_is_refused_by_a_frozen_receiver() {
    let source = "function run(o, n) {
            for (var i = 0; i < n; i++) { o['x'] = i; }
            return o.x;
        }
        run(Object.freeze({ x: -1 }), 10);";
    assert_eq!(eval(source), Ok(Value::Number(-1.0)));
}

#[test]
fn a_numeric_index_still_reads_an_array_after_its_producer_is_boxed() {
    // Demanding a boxed key forces the producing element read boxed too, so an
    // ordinary `a[b[i]]` now flows through the computed access. It must still
    // read the array rather than deoptimizing.
    let source = "function run(values, indices, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += values[indices[i % indices.length]]; }
            return total;
        }
        run([10, 20, 30], [0, 1, 2], 9);";
    assert_eq!(eval(source), Ok(Value::Number(180.0)));
}

#[test]
fn a_join_widening_a_scalar_keeps_both_branch_values() {
    let source = "function run(o, keys, n) {
            var out = '';
            for (var i = 0; i < n; i++) { out += (o[keys[i % keys.length]] || 'X'); }
            return out;
        }
        run({ a: 'A' }, ['a', 'missing'], 4);";
    assert_eq!(eval(source), Ok(Value::String("AXAX".into())));
}

// A property found on the receiver's immediate prototype is remembered, so a
// method call site stops walking the chain every iteration. Three things can
// invalidate that, and each has to be caught.

#[test]
fn an_inherited_method_is_read_from_the_prototype_every_iteration() {
    let source = "function Stepper(step) { this.step = step; }
        Stepper.prototype.advance = function (v) { return v + this.step; };
        function run(pool, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += pool[i % pool.length].advance(i); }
            return total;
        }
        run([new Stepper(1), new Stepper(1)], 10);";
    assert_eq!(eval(source), Ok(Value::Number(55.0)));
}

#[test]
fn an_own_property_shadows_the_remembered_prototype_one() {
    let source = "var proto = { tag: 'proto' };
        function run(a, b, n) {
            var out = '';
            for (var i = 0; i < n; i++) { out += (i % 2 === 0 ? a : b).tag; }
            return out;
        }
        var plain = Object.create(proto);
        var shadowed = Object.create(proto);
        shadowed.tag = 'own';
        run(plain, shadowed, 4);";
    assert_eq!(eval(source), Ok(Value::String("protoownprotoown".into())));
}

#[test]
fn mutating_the_prototype_between_iterations_is_observed() {
    let source = "var proto = { v: 1 };
        var o = Object.create(proto);
        function run(o, proto, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o.v; if (i === 2) { proto.v = 10; } }
            return total;
        }
        run(o, proto, 6);";
    assert_eq!(eval(source), Ok(Value::Number(33.0)));
}

#[test]
fn adding_an_own_property_mid_loop_shadows_the_cached_prototype() {
    let source = "var proto = { v: 1 };
        var o = Object.create(proto);
        function run(o, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o.v; if (i === 2) { o.v = 100; } }
            return total;
        }
        run(o, 6);";
    assert_eq!(eval(source), Ok(Value::Number(303.0)));
}

#[test]
fn replacing_the_prototype_mid_loop_is_observed() {
    let source = "var first = { v: 1 };
        var second = { v: 7 };
        var o = Object.create(first);
        function run(o, second, n) {
            var total = 0;
            for (var i = 0; i < n; i++) {
                total += o.v;
                if (i === 2) { Object.setPrototypeOf(o, second); }
            }
            return total;
        }
        run(o, second, 6);";
    assert_eq!(eval(source), Ok(Value::Number(24.0)));
}

#[test]
fn an_inherited_accessor_is_not_answered_from_the_slot_cache() {
    let source = "var calls = 0;
        var proto = {};
        Object.defineProperty(proto, 'v', { get: function () { calls++; return 2; } });
        var o = Object.create(proto);
        function run(o, n) {
            var total = 0;
            for (var i = 0; i < n; i++) { total += o.v; }
            return total;
        }
        run(o, 5) + ':' + calls;";
    assert_eq!(eval(source), Ok(Value::String("10:5".into())));
}
