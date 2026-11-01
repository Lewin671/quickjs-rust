//! Discarding a result never discards evaluation, validation or abrupt completion.
use super::{NumOp, optimize};
use crate::{Value, eval, promise};

#[test]
fn optimizer_removes_only_proven_scalar_results() {
    let (ops, _) = optimize(vec![
        NumOp::Const { dst: 0, value: 3.0 },
        NumOp::Neg { dst: 1, src: 0 },
        NumOp::Const { dst: 2, value: 7.0 },
        NumOp::Return { src: 2 },
    ]);
    assert!(!ops.iter().any(|op| matches!(op, NumOp::Neg { .. })));
}

#[test]
fn optimizer_retains_unmodeled_and_native_evaluation_obligations() {
    let (ops, _) = optimize(vec![
        NumOp::Const { dst: 0, value: 3.0 },
        NumOp::Native {
            dst: 1,
            native: crate::NativeFunction::MathRandom,
            first: 0,
            second: 0,
            arity: 0,
        },
        NumOp::Other {
            dst: 1,
            op: qjs_ast::BinaryOp::In,
            left: 0,
            right: 0,
        },
        NumOp::Bail,
        NumOp::Const { dst: 2, value: 7.0 },
        NumOp::Return { src: 2 },
    ]);
    assert!(ops.iter().any(|op| matches!(op, NumOp::Native { .. })));
    assert!(ops.iter().any(|op| matches!(op, NumOp::Other { .. })));
    assert!(ops.iter().any(|op| matches!(op, NumOp::Bail)));
}

#[test]
fn optimizer_keeps_a_call_with_an_unread_result_and_its_arguments() {
    let (ops, _) = optimize(vec![
        NumOp::Const { dst: 0, value: 3.0 },
        NumOp::Call {
            dst: 1,
            callee: 0,
            args: 0,
            argc: 1,
        },
        NumOp::Const { dst: 2, value: 7.0 },
        NumOp::Return { src: 2 },
    ]);
    assert_eq!(
        ops.iter()
            .filter(|op| matches!(op, NumOp::Call { .. }))
            .count(),
        1
    );
    assert!(
        ops.iter()
            .any(|op| matches!(op, NumOp::Move { dst: 0, .. }))
    );
}

fn check_effect(route: &str, expected: i32) {
    let source = format!(
        "var n=0;function side(){{n++;}} \
        function h(a,b){{side();return a-b;}} {route};n;"
    );
    let value = eval(&source).unwrap();
    if expected < 0 {
        assert!(
            matches!(value, Value::Number(n) if n > 0.0),
            "{source}: {value:?}"
        );
    } else {
        assert_eq!(value, Value::Number(f64::from(expected)), "{source}");
    }
}

fn check_throw(route: &str, _expected: i32) {
    let source = format!(
        "var marker={{}};function side(){{throw marker;}} \
        function h(a,b){{side();return a-b;}} \
        var caught=false;try{{{route};}}catch(e){{caught=e===marker;}}caught;"
    );
    assert_eq!(eval(&source), Ok(Value::Boolean(true)), "{source}");
}

macro_rules! routes {
    ($check:ident) => {
        #[test]
        fn direct() {
            $check("h(2,1)", 1);
        }
        #[test]
        fn call() {
            $check("h.call(null,2,1)", 1);
        }
        #[test]
        fn apply() {
            $check("h.apply(null,[2,1])", 1);
        }
        #[test]
        fn reflect_apply() {
            $check("Reflect.apply(h,null,[2,1])", 1);
        }
        #[test]
        fn bound() {
            $check("h.bind(null)(2,1)", 1);
        }
        #[test]
        fn sort() {
            $check("[2,1].sort(h)", -1);
        }
        #[test]
        fn to_sorted() {
            $check("[2,1].toSorted(h)", -1);
        }
        #[test]
        fn typed_sort() {
            $check("new Uint8Array([2,1]).sort(h)", -1);
        }
        #[test]
        fn typed_to_sorted() {
            $check("new Uint8Array([2,1]).toSorted(h)", -1);
        }
        #[test]
        fn array_from() {
            $check("Array.from([2,1],h)", 2);
        }
    };
}

mod effects {
    use super::*;
    routes!(check_effect);
}

mod throws {
    use super::*;
    routes!(check_throw);
}

#[test]
fn local_closure_calls_keep_effects_and_throw_identity() {
    assert_eq!(
        eval(
            "(function(){var n=0;function side(){n++;} \
        function h(a,b){side();return a-b;}h(2,1);return n;})()"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "(function(){var marker={};function side(){throw marker;} \
        function h(a,b){side();return a-b;}try{h(2,1);}catch(e){return e===marker;}return false;})()"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn promise_reactions_keep_discarded_calls_and_rejections() {
    let value = eval("var n=0;function side(){n++;} \
        Promise.resolve(3).then(function(a){side();return a-1;}).then(function(v){return n+':'+v;});").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&value),
        Some(("fulfilled".into(), Value::String("1:2".into())))
    );
    let value = eval("var marker={};function side(){throw marker;} \
        Promise.resolve(3).then(function(a){side();return a-1;}).then(function(){return false;},function(e){return e===marker;});").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&value),
        Some(("fulfilled".into(), Value::Boolean(true)))
    );
}

#[test]
fn discarded_property_reads_and_coercions_remain_observable() {
    for expression in [
        "var obj={get p(){n++;return 3;}};function h(a,b){obj.p;return a-b;}",
        "var obj=new Proxy({p:3},{get(t,k){n++;return t[k];}});function h(a,b){obj.p;return a-b;}",
        "var obj={valueOf(){n++;return 3;}};function h(a,b){+obj;return a-b;}",
        "var obj={valueOf(){n++;return 3;}};function h(a,b){Math.abs(obj);return a-b;}",
    ] {
        let source = format!("var n=0;{expression}h(2,1);n;");
        assert_eq!(eval(&source), Ok(Value::Number(1.0)), "{source}");
        let loop_body = expression
            .replace("obj.p;", "for(var i=0;i<32;i++){obj.p;}")
            .replace("+obj;", "for(var i=0;i<32;i++){+obj;}")
            .replace("Math.abs(obj);", "for(var i=0;i<32;i++){Math.abs(obj);}");
        let source = format!("var n=0;{loop_body}h(2,1);n;");
        assert_eq!(eval(&source), Ok(Value::Number(32.0)), "{source}");
    }
    for expression in [
        "var obj={get p(){throw marker;}};function h(a,b){obj.p;return a-b;}",
        "var obj={valueOf(){throw marker;}};function h(a,b){+obj;return a-b;}",
    ] {
        let source = format!(
            "var marker={{}};{expression}var caught=false;try{{h(2,1);}}catch(e){{caught=e===marker;}}caught;"
        );
        assert_eq!(eval(&source), Ok(Value::Boolean(true)), "{source}");
    }
}
