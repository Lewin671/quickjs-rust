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
        eval("[5].map(function() { 'use strict'; return this === undefined; })[0];"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[11].map(function() { return this === eval; }, eval)[0];"),
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
            "let xs = [1, 2, 3, 4, 5]; xs.map(function(value) { xs[4] = -1; return value > 0 ? 1 : 0; })[4];"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4, 5]; xs.map(function(value) { delete xs[4]; return value > 0 ? 1 : 0; })[4];"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let calls = 0; let xs = new Array(10); xs[1] = 1; xs[2] = 2; xs.map(function(value) { calls = calls + 1; return value; }); calls;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let xs = [0, 1, true, null, {}, 'five']; xs[999999] = -6.6; let calls = 0; xs.map(function(value, index, array) { if (array[index] === value) { calls = calls + 1; } }); calls;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "Array.prototype[1] = 13; let result = [, , ,].map(function(value, index) { return index === 1 && value === 13; }); result[1];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = []; Object.defineProperty(xs, '0', { get: function() { return 'abc'; }, configurable: true }); xs.map(function(value) { return value === 'abc'; })[0];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = [0, , 2]; Object.defineProperty(xs, '0', { get: function() { Object.defineProperty(xs, '1', { get: function() { return 1; }, configurable: true }); return 0; }, configurable: true }); xs.map(function(value, index) { return index === 1 && value === 1 ? false : true; }).join('|');"
        ),
        Ok(Value::String("true|false|true".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [0, , 2]; Object.defineProperty(xs, '0', { get: function() { Object.defineProperty(Array.prototype, '1', { get: function() { return 6.99; }, configurable: true }); return 0; }, configurable: true }); xs.map(function(value, index) { return index === 1 && value === 6.99 ? false : true; }).join('|');"
        ),
        Ok(Value::String("true|false|true".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2]; Object.defineProperty(xs, '1', { get: function() { return '6.99'; }, configurable: true }); Object.defineProperty(xs, '0', { get: function() { delete xs[1]; return 0; }, configurable: true }); xs.map(function(value, index) { return index === 1 ? false : true; })[1];"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let calls = 0; let xs = []; xs.constructor = 1; let caught = false; try { xs.map(function() { calls = calls + 1; }); } catch (error) { caught = error.constructor === TypeError; } caught + ':' + calls;"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let marker = { ok: true }; let xs = []; Object.defineProperty(xs, 'constructor', { get: function() { throw marker; }, configurable: true }); let caught = false; try { xs.map(function() { calls = calls + 1; }); } catch (error) { caught = error === marker; } caught + ':' + calls;"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
    assert!(eval("Array.prototype.map.call({ length: 4294967296 }, function() {});").is_err());
    assert!(eval("Array.prototype.map.call({ length: 4294967297 }, function() {});").is_err());
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
            "let object = { 0: 5 }; object.length = []; Array.prototype.map.call(object, function() { return 1; }).length;"
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
        eval(
            "function Foo() {} Foo.prototype = new Array(1, 2, 3); let receiver = new Foo(); receiver.length = 1; Array.isArray(receiver.map(function() {})) && receiver.map(function() {}).length;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let xs = [1, , 3]; let result = xs.map(function(value) { return value * 2; }); result.length + ':' + Object.prototype.hasOwnProperty.call(result, '1') + ':' + result[0] + ':' + result[2];"
        ),
        Ok(Value::String("3:false:2:6".to_owned()))
    );
    assert_eq!(
        eval(
            "let target; function C(length) { this.lengthValue = length; target = this; } let xs = [2, 4]; xs.constructor = {}; xs.constructor[Symbol.species] = C; let result = xs.map(function(value) { return value + 1; }); result === target && result.lengthValue === 2 && result[0] === 3 && result[1] === 5 && !Object.prototype.hasOwnProperty.call(result, 'length');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.preventExtensions(target); function C() { return target; } let xs = [1]; xs.constructor = {}; xs.constructor[Symbol.species] = C; let caught = false; try { xs.map(function(value) { return value; }); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.defineProperty(target, '0', { value: 1, configurable: false }); function C() { return target; } let xs = [2]; xs.constructor = {}; xs.constructor[Symbol.species] = C; let caught = false; try { xs.map(function(value) { return value; }); } catch (error) { caught = error.constructor === TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].filter(function(value) { return value > 2; }).join();"),
        Ok(Value::String("3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let xs = []; xs.constructor = 1; let caught = false; try { xs.filter(function() { calls = calls + 1; }); } catch (error) { caught = error.constructor === TypeError; } caught + ':' + calls;"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "function Inner() { this.flag = true; function callback() { return this.flag; } let result = [1].filter(callback); this.retVal = result.length === 0; } new Inner().retVal;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let obj = new RegExp(); obj.length = 2; obj[1] = true; Array.prototype.filter.call(obj, function(value, index, receiver) { return receiver instanceof RegExp; })[0];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let xs = [0, , 2]; Object.defineProperty(xs, '0', { get: function() { Object.defineProperty(xs, '1', { get: function() { return 1; }, configurable: true }); return 0; }, configurable: true }); xs.filter(function(value) { return value === 1; })[0];"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let obj = { length: 2 }; Object.defineProperty(obj, '0', { get: function() { Object.defineProperty(Object.prototype, '1', { get: function() { return 6.99; }, configurable: true }); return 0; }, configurable: true }); let result = Array.prototype.filter.call(obj, function() { return true; }); result.length + ':' + Array[1];"
        ),
        Ok(Value::String("2:6.99".to_owned()))
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
            "let calls = 0; [undefined, , , 'foo'].find(function() { calls = calls + 1; return false; }); calls;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(
            "let xs = ['Shoes', 'Car', 'Bike']; let seen = []; xs.find(function(value) { if (seen.length === 0) { xs.splice(1, 1); } seen.push(value); return false; }); seen.length + ':' + seen[0] + ':' + seen[1] + ':' + seen[2];"
        ),
        Ok(Value::String("3:Shoes:Bike:undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = ['Skateboard', 'Barefoot']; let seen = []; xs.find(function(value) { if (seen.length === 0) { xs.push('Motorcycle'); xs[1] = 'Magic Carpet'; } seen.push(value); return false; }); seen.length + ':' + seen[0] + ':' + seen[1];"
        ),
        Ok(Value::String("2:Skateboard:Magic Carpet".to_owned()))
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
            "let xs = [1, 2, 3]; let seen = []; xs.findLast(function(value) { if (seen.length === 0) { delete xs[1]; xs[0] = 9; } seen.push(value); return false; }); seen.join('|');"
        ),
        Ok(Value::String("3||9".to_owned()))
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
            "let calls = 0; [undefined, , , 'foo'].findIndex(function() { calls = calls + 1; return false; }); calls;"
        ),
        Ok(Value::Number(4.0))
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
        eval(
            "let calls = 0; let xs = new Array(10); xs[1] = undefined; xs.forEach(function() { calls = calls + 1; }); calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let seen = false; let xs = []; Object.defineProperty(xs, '2', { get: function() { return 12; }, configurable: true }); xs.forEach(function(value, index) { if (index === 2 && value === 12) { seen = true; } }); seen;"
        ),
        Ok(Value::Boolean(true))
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
        eval(
            "let xs = [0, 1, true, null, {}, 'five']; xs[999999] = -6.6; let calls = 0; xs.some(function(value, index, array) { if (arguments.length === 3 && array[index] === value) { calls = calls + 1; } return false; }); calls;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let obj = { length: 2 }; let slot = 11; Object.defineProperty(obj, '1', { get: function() { return slot; }, set: function(value) { slot = value; }, configurable: true }); Object.defineProperty(obj, '0', { get: function() { obj[1] = 12; return 11; }, configurable: true }); Array.prototype.some.call(obj, function(value, index) { return index === 1 && value === 12; });"
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
        eval(
            "let accessed = false; let obj = { 0: 9, length: 'Infinity' }; Array.prototype.every.call(obj, function(value) { accessed = true; return value > 10; }) + ':' + accessed;"
        ),
        Ok(Value::String("false:true".to_owned()))
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
        eval("[, 2, , 4].reduce(function(accumulator, value) { return accumulator + value; });"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "let xs = [, , 3]; Array.prototype[1] = 2; let result = xs.reduce(function(accumulator, value, index) { return accumulator + ':' + index + ':' + value; }); delete Array.prototype[1]; result;"
        ),
        Ok(Value::String("2:2:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let seen = ''; xs.reduce(function(accumulator, value, index) { if (index === 1) { delete xs[2]; } seen = seen + value + '|'; return accumulator + value; }, 0); seen;"
        ),
        Ok(Value::String("1|2|".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, , 3]; let seen = ''; xs.reduce(function(accumulator, value, index) { if (index === 0) { xs[1] = 2; } seen = seen + value + ':' + index + '|'; return accumulator + value; }, 0); seen;"
        ),
        Ok(Value::String("1:0|2:1|3:2|".to_owned()))
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
        eval(
            "[1, , 3].reduceRight(function(accumulator, value, index) { return accumulator + ':' + index + ':' + value; });"
        ),
        Ok(Value::String("3:0:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, , 3]; let seen = ''; xs.reduceRight(function(accumulator, value, index) { if (index === 2) { xs[1] = 2; } seen = seen + value + ':' + index + '|'; return accumulator + value; }, 0); seen;"
        ),
        Ok(Value::String("3:2|2:1|1:0|".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let seen = ''; xs.reduceRight(function(accumulator, value, index) { if (index === 2) { delete xs[1]; } seen = seen + value + ':' + index + '|'; return accumulator + value; }, 0); seen;"
        ),
        Ok(Value::String("3:2|1:0|".to_owned()))
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
