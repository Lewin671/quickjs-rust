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
    assert_eq!(
        eval(
            "let xs = ['a', , undefined]; xs.sort(); xs.length + ':' + xs.hasOwnProperty('0') + ':' + xs.hasOwnProperty('1') + ':' + xs.hasOwnProperty('2') + ':' + xs[0] + ':' + (xs[1] === undefined) + ':' + (xs[2] === undefined);"
        ),
        Ok(Value::String("3:true:true:false:a:true:true".to_owned()))
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
        // The stable merge sort compares the left run element against the right
        // run element, so the adjacent pair [2, 1] is passed as (2, 1). The
        // comparator argument order is implementation-defined; only the sorted
        // result and stability are observable per spec.
        Ok(Value::String("2:1".to_owned()))
    );
}

#[test]
fn evaluates_array_sort_generic_receivers() {
    assert_eq!(
        eval(
            "let object = { 0: undefined, 1: 2, 2: 1, 3: 'X', 4: -1, 5: 'a', 6: true, 7: { toString: function() { return -2; } }, 8: NaN, 9: Infinity, length: 10 }; let result = Array.prototype.sort.call(object); result === object && object[0] === -1 && object[2] === 1 && object[9] === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let result = Array.prototype.sort.call(false); result instanceof Boolean && result.length === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_sort_preserves_receiver_when_compare_throws() {
    assert_eq!(
        eval(
            "let logs = []; Object.defineProperty(Object.prototype, '2', { get: function() { logs.push('get'); return 4; }, set: function(v) { logs.push('set ' + v); }, configurable: true }); let array = [undefined, 3, , 2, undefined, , 1]; let count = 0; try { array.sort(function(a, b) { if (++count === 3) { throw new Error('stop'); } return b - a; }); } catch (error) { logs.push(error.message); } let result = logs.join('|') + ':' + (array[0] === undefined) + ':' + array[1] + ':' + ('2' in array) + ':' + array.hasOwnProperty('2') + ':' + array[3] + ':' + (array[4] === undefined) + ':' + ('5' in array) + ':' + array[6]; delete Object.prototype[2]; result;"
        ),
        Ok(Value::String(
            "get|stop:true:3:true:false:2:true:false:1".to_owned()
        ))
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
