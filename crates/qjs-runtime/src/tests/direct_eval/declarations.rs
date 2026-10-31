//! Eval declarations shadow captured outer bindings in the eval's own scope.

use crate::{Value, eval};

#[test]
fn eval_declarations_shadow_received_outer_cells() {
    for strict in [false, true] {
        for declaration in ["var x = 5; x", "function x() { return 5; } x()"] {
            for pre_read in [false, true] {
                let directive = if strict { "'use strict';" } else { "" };
                let read = if pre_read {
                    "let before = eval('x');"
                } else {
                    "let before = 1;"
                };
                let source = format!(
                    "function outer(x) {{ return [function () {{ {directive}
                        {read} const inside = eval({declaration:?});
                        const afterValue = eval('x');
                        const after = typeof afterValue === 'function' ? afterValue() : afterValue;
                        return before + ':' + inside + ':' + after;
                    }}, () => x]; }} const c = outer(1);
                    c[0]() + ':' + c[1]();"
                );
                let after = if strict { "1" } else { "5" };
                assert_eq!(
                    eval(&source),
                    Ok(Value::String(format!("1:5:{after}:1").into())),
                    "{source}"
                );
            }
        }
    }
}

#[test]
fn eval_shadowing_preserves_outer_and_retargets_existing_inner_closures() {
    let source = r#"function outer(x) {
        return [function () {
            const get = () => x;
            eval('var x = 5');
            const child = function () { eval('var x = 7'); return eval('x'); };
            return get() + ':' + child() + ':' + x;
        }, () => x];
    } const c = outer(1); c[0]() + ':' + c[1]();"#;
    assert_eq!(eval(source), Ok(Value::String("5:7:5:1".into())));
}

#[test]
fn eval_var_without_initializer_creates_an_undefined_inner_binding() {
    assert_eq!(
        eval(
            "function outer(x) { return [function () { return String(eval('var x; x')); }, () => x]; } const c = outer(1); c[0]() + ':' + c[1]();"
        ),
        Ok(Value::String("undefined:1".into()))
    );
}

#[test]
fn eval_var_still_conflicts_with_an_owned_lexical_binding() {
    assert_eq!(
        eval(
            "function outer(x) { return function () { let x = 2; eval('var x = 5'); }; } let result; try { outer(1)(); } catch(error) { result = error instanceof SyntaxError; } result;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn eval_var_shadow_of_an_outer_const_is_mutable() {
    assert_eq!(
        eval(
            "const x = 1; function inner() { eval('var x = 5'); x = 6; return x; } inner() + ':' + x;"
        ),
        Ok(Value::String("6:1".into()))
    );
}

#[test]
fn eval_shadow_cells_survive_generator_suspension() {
    assert_eq!(
        eval(
            "function outer(x) { return [function* () { const get = () => x; yield eval('var x = 5; x'); yield get(); }, () => x]; } const c = outer(1); const it = c[0](); it.next().value + ':' + it.next().value + ':' + c[1]();"
        ),
        Ok(Value::String("5:5:1".into()))
    );
}
