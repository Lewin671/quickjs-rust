use crate::{Value, eval};

#[test]
fn evaluates_array_iteration_builtins() {
    assert_eq!(
        eval("[1, 2, 3].map(function(value) { return value * 2; }).join();"),
        Ok(Value::String("2,4,6".to_owned()))
    );
    assert_eq!(
        eval("[10, 20].map(function(value, index) { return value + index; }).join('|');"),
        Ok(Value::String("10|21".to_owned()))
    );
    assert_eq!(
        eval(
            "let receiver = [5]; [5].map(function(value, index, array) { return this === receiver && array[0] === value && index === 0; }, receiver)[0];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2]; let ys = xs.map(function(value) { return value + 1; }); xs !== ys && xs[0] === 1 && ys[0] === 2;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].filter(function(value) { return value > 2; }).join();"),
        Ok(Value::String("3,4".to_owned()))
    );
    assert_eq!(
        eval("[10, 20, 30].filter(function(value, index) { return index === 1; })[0];"),
        Ok(Value::Number(20.0))
    );
    assert_eq!(
        eval(
            "let receiver = [2]; [1, 2].filter(function(value, index, array) { return this === receiver && array[index] === value && value === receiver[0]; }, receiver).join();"
        ),
        Ok(Value::String("2".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let ys = xs.filter(function(value) { return value < 3; }); xs !== ys && xs.join() === '1,2,3' && ys.join() === '1,2';"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].find(function(value) { return value > 2; });"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("[1, 2, 3].find(function(value) { return value > 5; });"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let receiver = { target: 20 }; [10, 20].find(function(value, index, array) { return this === receiver && index === 1 && array[index] === value && value === this.target; }, receiver);"
        ),
        Ok(Value::Number(20.0))
    );
    assert_eq!(
        eval(
            "let total = 0; [1, 2, 3].forEach(function(value) { total = total + value; }); total;"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "let seen = ''; [10, 20].forEach(function(value, index, array) { seen = seen + value + ':' + index + ':' + (array[index] === value) + '|'; }); seen;"
        ),
        Ok(Value::String("10:0:true|20:1:true|".to_owned()))
    );
    assert_eq!(
        eval(
            "let receiver = { total: 0 }; [1, 2].forEach(function(value) { this.total = this.total + value; }, receiver); receiver.total;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("[1].forEach(function() { return 42; });"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("[1, 2, 3].some(function(value) { return value > 2; });"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3].some(function(value) { return value > 5; });"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let receiver = { target: 20 }; [10, 20].some(function(value, index, array) { return this === receiver && index === 1 && array[index] === value && value === this.target; }, receiver);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3].every(function(value) { return value > 0; });"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3].every(function(value) { return value < 3; });"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let receiver = { limit: 30 }; [10, 20].every(function(value, index, array) { return this === receiver && array[index] === value && value < this.limit; }, receiver);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[].every(function() { return false; });"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3].reduce(function(accumulator, value) { return accumulator + value; });"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("[1, 2, 3].reduce(function(accumulator, value) { return accumulator + value; }, 10);"),
        Ok(Value::Number(16.0))
    );
    assert_eq!(
        eval(
            "let seen = ''; [10, 20].reduce(function(accumulator, value, index, array) { seen = seen + accumulator + ':' + value + ':' + index + ':' + (array[index] === value) + '|'; return accumulator + value; }, 5); seen;"
        ),
        Ok(Value::String("5:10:0:true|15:20:1:true|".to_owned()))
    );
    assert_eq!(
        eval("[].reduce(function(accumulator, value) { return accumulator + value; }, 7);"),
        Ok(Value::Number(7.0))
    );
    assert!(
        eval("[].reduce(function(accumulator, value) { return accumulator + value; });").is_err()
    );
    assert_eq!(
        eval(
            "[1, 2, 3].reduceRight(function(accumulator, value) { return accumulator + '-' + value; });"
        ),
        Ok(Value::String("3-2-1".to_owned()))
    );
    assert_eq!(
        eval(
            "[1, 2, 3].reduceRight(function(accumulator, value) { return accumulator + value; }, 10);"
        ),
        Ok(Value::Number(16.0))
    );
    assert_eq!(
        eval(
            "let seen = ''; [10, 20].reduceRight(function(accumulator, value, index, array) { seen = seen + accumulator + ':' + value + ':' + index + ':' + (array[index] === value) + '|'; return accumulator + value; }, 5); seen;"
        ),
        Ok(Value::String("5:20:1:true|25:10:0:true|".to_owned()))
    );
    assert_eq!(
        eval("[].reduceRight(function(accumulator, value) { return accumulator + value; }, 7);"),
        Ok(Value::Number(7.0))
    );
    assert!(
        eval("[].reduceRight(function(accumulator, value) { return accumulator + value; });")
            .is_err()
    );
}
