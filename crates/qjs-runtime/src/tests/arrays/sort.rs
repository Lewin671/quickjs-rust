use crate::{Value, eval};

#[test]
fn evaluates_array_sort_default_order() {
    assert_eq!(
        eval(
            "let xs = [3, 20, 100, 1]; let result = xs.sort(); (result === xs) + ':' + xs.join();"
        ),
        Ok(Value::String("true:1,100,20,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = ['b', undefined, 'a']; xs.sort(); xs.length + ':' + xs.join('|') + ':' + (xs[2] === undefined);"
        ),
        Ok(Value::String("3:a|b|:true".to_owned()))
    );
}

#[test]
fn evaluates_array_sort_with_compare_function() {
    assert_eq!(
        eval("[3, 1, 2].sort(function(left, right) { return left - right; }).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[3, 1, 2].sort(function(left, right) { return right - left; }).join();"),
        Ok(Value::String("3,2,1".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; [2, 1].sort(function(left, right) { seen = seen + left + ':' + right; return left - right; }); seen;"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
}

#[test]
fn rejects_non_callable_array_sort_comparator() {
    assert!(eval("[1, 2].sort(1);").is_err());
}

#[test]
fn evaluates_array_to_sorted_default_order() {
    assert_eq!(
        eval(
            "let xs = [3, 20, 100, 1]; let result = xs.toSorted(); result.join() + ':' + xs.join() + ':' + (result === xs);"
        ),
        Ok(Value::String("1,100,20,3:3,20,100,1:false".to_owned()))
    );
    assert_eq!(
        eval(
            "['b', undefined, 'a'].toSorted().join('|') + ':' + ([undefined].toSorted()[0] === undefined);"
        ),
        Ok(Value::String("a|b|:true".to_owned()))
    );
}

#[test]
fn evaluates_array_to_sorted_with_compare_function() {
    assert_eq!(
        eval("[3, 1, 2].toSorted(function(left, right) { return left - right; }).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[3, 1, 2].toSorted(function(left, right) { return right - left; }).join();"),
        Ok(Value::String("3,2,1".to_owned()))
    );
    assert_eq!(
        eval(
            "Array.prototype.toSorted.call({ length: 3, 0: 4, 1: 0, 2: 1 }, function(left, right) { return left - right; }).join();"
        ),
        Ok(Value::String("0,1,4".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toSorted.length;"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn rejects_non_callable_array_to_sorted_comparator() {
    assert!(eval("[1, 2].toSorted(1);").is_err());
    assert!(eval("Array.prototype.toSorted.call(null);").is_err());
}
