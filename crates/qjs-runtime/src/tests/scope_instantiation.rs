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
