//! Declaration instantiation and capture boundaries of lexical statement scopes.
use crate::{Value, eval};

#[test]
fn try_and_finally_instantiate_function_declarations_before_statements() {
    for strict in ["", "'use strict';"] {
        let source = format!(
            "{strict}var seen=[];function t(){{try{{return f();function f(){{return 7;}}}} \
            finally{{seen.push(g());function g(){{return 8;}}}}}}t()+':'+seen.join();"
        );
        assert_eq!(eval(&source), Ok(Value::String("7:8".into())), "{source}");
    }
    assert_eq!(
        eval("var f;try{function f(){return 1;}}finally{}f();"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'use strict';try{function f(){return 1;}}finally{}typeof f;"),
        Ok(Value::String("undefined".into()))
    );
}

#[test]
fn inner_block_shadows_do_not_hide_captures_outside_the_block() {
    for shadow in [
        "{let z='block';}",
        "{const z='block';}",
        "for(let z=0;z<1;z++){}",
        "try{throw 0;}catch(z){}",
    ] {
        for write in [false, true] {
            let use_z = if write { "z=1;return z;" } else { "return z;" };
            let source = format!(
                "function outer(){{var z='outer';function inner(){{{shadow}{use_z}}}return inner()+':'+z;}}outer();"
            );
            let expected = if write { "1:1" } else { "outer:outer" };
            assert_eq!(
                eval(&source),
                Ok(Value::String(expected.into())),
                "{source}"
            );
        }
    }
}

#[test]
fn named_expression_self_scope_precedes_an_outer_homonym_despite_inner_shadow() {
    assert_eq!(
        eval("var nf='outer';(function nf(){{let nf='block';}return typeof nf;})()"),
        Ok(Value::String("function".into()))
    );
}
