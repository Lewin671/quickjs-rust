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
        eval(
            "let object = {}; object.length = 2; object[0] = 3; object[1] = 4; Array.prototype.map.call(object, function(value, index, receiver) { return receiver === object ? value + index : 0; }).join('|');"
        ),
        Ok(Value::String("3|5".to_owned()))
    );
    assert_eq!(
        eval(
            "Math.length = 1; Math[0] = 7; Array.prototype.map.call(Math, function(value, index, receiver) { return receiver === Math && index === 0 ? value + 1 : 0; })[0];"
        ),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval(
            "function capture() { return Array.prototype.map.call(arguments, function(value, index, receiver) { return Object.prototype.toString.call(receiver) === '[object Arguments]' ? value + index : 0; }).join('|'); } capture(4, 8);"
        ),
        Ok(Value::String("4|9".to_owned()))
    );
    assert_eq!(
        eval(
            "Boolean.prototype.length = 1; Boolean.prototype[0] = true; Array.prototype.map.call(true, function(value, index, receiver) { return receiver instanceof Boolean && index === 0 && value === true; })[0];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Number.prototype.length = 1; Number.prototype[0] = 6; Array.prototype.map.call(1, function(value, index, receiver) { return receiver instanceof Number && index === 0 ? value + 1 : 0; })[0];"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "function pair(a, b) {} pair[0] = 11; pair[1] = 9; Array.prototype.map.call(pair, function(value, index, receiver) { return receiver instanceof Function ? value + index : 0; }).join('|');"
        ),
        Ok(Value::String("11|10".to_owned()))
    );
    assert_eq!(
        eval(
            "let proto = {}; Object.defineProperty(proto, 'length', { get: function() { return 2; }, configurable: true }); let object = Object.create(proto); object[0] = 5; object[1] = 7; Array.prototype.map.call(object, function(value) { return value + 1; }).join('|');"
        ),
        Ok(Value::String("6|8".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { 0: 5, 1: 7 }; Object.defineProperty(object, 'length', { set: function(_) {}, configurable: true }); Array.prototype.map.call(object, function(value) { return value + 1; }).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let object = { 10: 10 }; let lengthAccessed = false; let loopAccessed = false; Object.defineProperty(object, 'length', { get: function() { lengthAccessed = true; return 20; }, configurable: true }); Object.defineProperty(object, '0', { get: function() { loopAccessed = true; return 10; }, configurable: true }); try { Array.prototype.map.call(object); } catch (error) {} lengthAccessed + ':' + loopAccessed;"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let receiver = new Array(); receiver.res = true; [1].map(function(value) { return this.res; }, receiver)[0];"
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
        eval("[1, 2, 3, 4].findLast(function(value) { return value > 2; });"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("[1, 2, 3].findLast(function(value) { return value > 5; });"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let receiver = { target: 10 }; [10, 20].findLast(function(value, index, array) { return this === receiver && index === 0 && array[index] === value && value === this.target; }, receiver);"
        ),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].findLastIndex(function(value) { return value > 2; });"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("[1, 2, 3].findLastIndex(function(value) { return value > 5; });"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let receiver = { target: 10 }; [10, 20].findLastIndex(function(value, index, array) { return this === receiver && index === 0 && array[index] === value && value === this.target; }, receiver);"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].findIndex(function(value) { return value > 2; });"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("[1, 2, 3].findIndex(function(value) { return value > 5; });"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let receiver = { target: 20 }; [10, 20].findIndex(function(value, index, array) { return this === receiver && index === 1 && array[index] === value && value === this.target; }, receiver);"
        ),
        Ok(Value::Number(1.0))
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
        eval(
            "let object = {}; object.length = 2; object[0] = 5; object[1] = 7; Array.prototype.reduce.call(object, function(accumulator, value, index, receiver) { return accumulator + value + index + receiver.length; }, 0);"
        ),
        Ok(Value::Number(17.0))
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
