//! Live-scope and immutable-cell discriminators recovered from T071.

use crate::{Value, eval};

#[test]
fn inherited_eval_maps_observe_declarations_after_closure_creation() {
    for source in [
        "function outer(){ function inner(){ eval(''); return () => b; } var g = inner(); eval('var b = 5'); return g(); } outer();",
        "function outer(){ function inner(){ eval(''); return () => b(); } var g = inner(); eval('function b(){return 5}'); return g(); } outer();",
        "function outer(){ function* inner(){ eval(''); yield; return b; } var g = inner(); g.next(); eval('var b = 5'); return g.next().value; } outer();",
        "function outer(){ function* inner(){ eval(''); yield; return b(); } var g = inner(); g.next(); eval('function b(){return 5}'); return g.next().value; } outer();",
    ] {
        assert_eq!(eval(source), Ok(Value::Number(5.0)), "{source}");
    }
}

#[test]
fn nested_eval_maps_share_later_rebindings() {
    assert_eq!(
        eval(
            "function a(){ var x = 1; return function b(){ eval(''); function c(){ eval(''); return () => x; } var f = c(); eval('var x = 2'); return [f(), x].join(','); } } a()();"
        ),
        Ok(Value::String("2,2".into()))
    );
}

#[test]
fn eval_only_assignment_does_not_mutate_an_outer_const_cell() {
    assert_eq!(
        eval(
            "function o(){ const t = 3; var f = function(){ eval('t = 4'); }; let threw = false; try { f() } catch (e) { threw = e instanceof TypeError; } return t === 3 && threw; } o();"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn eval_only_assignment_does_not_initialize_an_outer_tdz_cell() {
    assert_eq!(
        eval(
            "function o(){ var f = function(){ eval('t = 4'); }; let threw = false; try { f() } catch(e) { threw = e instanceof ReferenceError; } var result; try { t; result = false; } catch(e) { result = e instanceof ReferenceError; } let t = 3; return result && threw && t === 3; } o();"
        ),
        Ok(Value::Boolean(true))
    );
}
