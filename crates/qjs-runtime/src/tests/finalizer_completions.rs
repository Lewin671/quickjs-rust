//! Abrupt completion routing at the shared bytecode execution boundary.
use crate::{Value, eval};
fn text(value: &str) -> Value {
    Value::String(value.to_owned().into())
}

#[test]
fn nested_finally_jumps_in_loop_and_switch_bodies() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/nested-finally-jumps.js"
        )),
        Ok(text("ok"))
    );
}

#[test]
fn finalizers_override_only_completions_they_exit() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/finally-completion-overrides.js"
        )),
        Ok(text("ok"))
    );
}

#[test]
fn iterator_close_follows_inner_finalizers_and_can_be_cancelled() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/finally-iterator-close-order.js"
        )),
        Ok(text("ok"))
    );
}

#[test]
fn generator_finalizers_retain_nested_completions_across_yield() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/finally-suspended-completions.js"
        )),
        Ok(text(
            "a0:false,x:false,y:false,b0:false,a1:false,x:false,y:false,b1:false,7:true,0,1,2:false,3:false,4:false,9:true"
        ))
    );
}

#[test]
fn async_finalizers_retain_completions_across_await() {
    let value = eval(r#"var log=[]; async function f(){for(var i=0;i<2;i++){try{try{continue;}finally{log.push('a'+i);await 0;try{await 1;}finally{await 2;}}}finally{await 3;log.push('b'+i);}}return 7;} f().then(v=>log.push(v)); log;"#).unwrap();
    let Value::Array(log) = value else {
        panic!("expected log")
    };
    assert_eq!(
        log.to_vec(),
        vec![
            text("a0"),
            text("b0"),
            text("a1"),
            text("b1"),
            Value::Number(7.0)
        ]
    );
}

#[test]
fn hot_callers_and_loop_tiers_keep_finalizer_effects() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/finally-hot-calls.js"
        )),
        Ok(text("5950:5950"))
    );
}
