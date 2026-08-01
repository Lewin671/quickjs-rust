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
