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
