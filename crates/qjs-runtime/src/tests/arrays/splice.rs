use crate::{Value, eval};

#[test]
fn evaluates_array_splice_deletion() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(1, 2); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2,3:1,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4]; let removed = xs.splice(-2, 1); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("3:1,2,4".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let removed = xs.splice(1); removed.join() + ':' + xs.join();"),
        Ok(Value::String("2,3:1".to_owned()))
    );
}

#[test]
fn evaluates_array_splice_insertion_and_replacement() {
    assert_eq!(
        eval(
            "let xs = [1, 4]; let removed = xs.splice(1, 0, 2, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 5]; let removed = xs.splice(1, 1, 3, 4); removed.join() + ':' + xs.join();"
        ),
        Ok(Value::String("2:1,3,4,5".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, undefined, 3]; let removed = xs.splice(1, 1, 2); (removed[0] === undefined) + ':' + xs.join();"
        ),
        Ok(Value::String("true:1,2,3".to_owned()))
    );
}

#[test]
fn evaluates_array_splice_bounds() {
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(10, 1, 3); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:1,2,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2]; let removed = xs.splice(0, -1, 0); removed.length + ':' + xs.join();"
        ),
        Ok(Value::String("0:0,1,2".to_owned()))
    );
}
