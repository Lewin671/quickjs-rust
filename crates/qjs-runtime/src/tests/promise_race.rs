use crate::{Value, eval, promise};

#[test]
fn drains_promise_race_jobs_after_script() {
    let empty = eval("Promise.race([]);").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&empty),
        Some(("pending".to_owned(), Value::Undefined))
    );

    let resolved = eval(
        "Promise.race([Promise.resolve(1), 2, { then: function(resolve) { resolve(3); } }]).then(function(value) { return value + 10; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&resolved),
        Some(("fulfilled".to_owned(), Value::Number(11.0)))
    );

    let thenable = eval(
        "Promise.race([{ then: function(resolve, reject) { resolve(4); reject(5); } }]).then(function(value) { return value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&thenable),
        Some(("fulfilled".to_owned(), Value::Number(4.0)))
    );
}

#[test]
fn drains_promise_race_rejections_after_script() {
    let rejected = eval(
        "Promise.race([Promise.reject(2), Promise.resolve(1)]).catch(function(reason) { return reason + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some(("fulfilled".to_owned(), Value::Number(3.0)))
    );
}
