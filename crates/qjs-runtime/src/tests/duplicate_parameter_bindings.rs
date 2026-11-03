//! Last-occurrence binding and arguments aliasing across activation routes.
use crate::{Value, eval};

#[test]
fn duplicate_parameters_share_last_input_in_hot_calls_methods_and_callbacks() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/duplicate-parameter-inputs.js"
        )),
        Ok(Value::String("ok".to_owned().into()))
    );
}

#[test]
fn direct_eval_reads_and_assigns_the_last_duplicate_binding() {
    assert_eq!(
        eval(
            "function f(a,a){var before=eval('a');eval('a=8');return String(before)+':'+a;} f(1)+'|'+f(1,2);"
        ),
        Ok(Value::String("undefined:8|2:8".to_owned().into()))
    );
}

#[test]
fn duplicate_parameter_arguments_map_only_the_supplied_last_occurrence() {
    assert_eq!(
        eval(
            "function f(a,a){Object.defineProperty(arguments,'0',{value:10});a=6;var x=arguments[0]+':'+arguments[1];Object.defineProperty(arguments,'1',{value:9,writable:false});a=7;return x+':'+arguments[1]+':'+a;}f(1,2)+'|'+f(1);"
        ),
        Ok(Value::String("10:6:9:7|10:undefined:9:7".to_owned().into()))
    );
}
