//! Direct eval in returned closures captures enclosing bindings as shared cells.

use crate::{Value, eval};

fn result(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())), "{source}");
}

#[test]
fn returned_eval_only_function_captures_outer_parameter() {
    result(
        "function outer(value) { return function () { return eval('value'); }; } String(outer(8)());",
        "8",
    );
}

#[test]
fn eval_closures_share_mutation_with_parent_and_siblings() {
    result(
        r#"function outer(value) {
            const read = function () { return eval('value'); };
            const write = function () { return eval('++value'); };
            value = 9; return [read, write, () => value];
        } const c = outer(8); c[0]() + ':' + c[1]() + ':' + c[2]();"#,
        "9:10:10",
    );
}

#[test]
fn nested_eval_closures_forward_outer_cells() {
    result(
        r#"function outer(value) { return function middle() {
            return () => () => eval('++value');
        }; } const c = outer(8)()() ; c() + ':' + c();"#,
        "9:10",
    );
}

#[test]
fn eval_closures_capture_nearest_block_and_iteration_bindings() {
    result(
        r#"function outer(value) {
            let read; { let value = 11; read = function () { return eval('value'); }; }
            let list = []; for (let i = 0; i < 3; i++) {
                list.push(function () { return eval('i'); });
            }
            return read() + ':' + list.map(f => f()).join(',');
        } outer(8);"#,
        "11:0,1,2",
    );
}

#[test]
fn eval_closures_preserve_own_bindings_and_const_tdz() {
    result(
        r#"function outer(value) {
            const fixed = 3; let read = function () { return eval('later'); };
            let tdz; try { read(); } catch(e) { tdz = e instanceof ReferenceError; }
            let later = 4;
            const own = function (value) { return eval('value'); };
            const readFixed = function () { return eval('fixed'); };
            return tdz + ':' + read() + ':' + own(6) + ':' + readFixed();
        } outer(8);"#,
        "true:4:6:3",
    );
}

#[test]
fn eval_in_methods_and_default_parameters_captures_outer_bindings() {
    result(
        r#"function outer(value) {
            return [class { method() { return eval('value'); } },
                function (x = eval('value')) { return x; }];
        } const c = outer(8); new c[0]().method() + ':' + c[1]();"#,
        "8:8",
    );
}

#[test]
fn eval_outside_inner_block_captures_the_outer_binding() {
    result(
        r#"function outer(value) { return function () {
            { let value = 1; }
            try { throw 2; } catch (value) { value; }
            return eval('value');
        }; } String(outer(8)());"#,
        "8",
    );
}

#[test]
fn eval_keeps_function_name_and_top_level_lexical_shadowing() {
    result(
        r#"function outer(value, named) {
            return [function () { let value = 6; return eval('value'); },
                function named() { return eval('named') === named; }];
        } const c = outer(8, 0); c[0]() + ':' + c[1]();"#,
        "6:true",
    );
}

#[test]
fn indirect_eval_does_not_capture_outer_bindings() {
    result(
        r#"var value = 2;
        function outer(value) { return () => (0, eval)('value'); }
        String(outer(8)());"#,
        "2",
    );
}
