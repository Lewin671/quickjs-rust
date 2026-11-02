//! Region-owned cleanup preserves lexical visibility and eager disposal.
use crate::{Value, eval};
fn text(s: &str) -> Value {
    Value::String(s.to_owned().into())
}

#[test]
fn with_exits_follow_inner_finalizers_for_all_abrupt_completions() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/with-abrupt-finalizer-scope.js"
        )),
        Ok(text("w/1/w/g/b0/b1/z/2"))
    );
}

#[test]
fn labeled_using_loop_exits_dispose_before_the_target_runs() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/for-using-labeled-cleanup.js"
        )),
        Ok(text("d1,after1,dy0,dy1,after2,dz,mid,end,1000"))
    );
}

#[test]
fn repeated_using_loop_exits_leave_no_deferred_resources() {
    assert_eq!(
        eval(
            "function f(n){var disposed=0;outer:for(var i=0;i<n;i++){for(using x={[Symbol.dispose](){disposed++;}};;){continue outer;}}var before=disposed;return before+':'+disposed;}f(100000);"
        ),
        Ok(text("100000:100000"))
    );
}

#[test]
fn suspended_with_finalizers_keep_their_object_until_completion() {
    assert_eq!(
        eval(
            "function* g(){with({x:'w'}){try{return yield 0;}finally{yield x;}}}var q=g();q.next().value+':'+q.next(9).value+':'+q.next().value;"
        ),
        Ok(text("0:w:9"))
    );
    let Value::Array(log) = eval("var log=[];async function f(){with({x:'w'}){try{return 1;}finally{await 0;log.push(x);}}}f().then(v=>log.push(v));log;").unwrap() else { panic!("expected array") };
    assert_eq!(log.to_vec(), vec![text("w"), Value::Number(1.0)]);
}
