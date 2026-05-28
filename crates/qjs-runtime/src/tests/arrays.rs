use crate::{Value, eval};

#[test]
fn evaluates_array_builtins() {
    assert_eq!(
        eval("typeof Array;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Array.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.isArray.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.concat.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.copyWithin.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Array.prototype.fill.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.filter.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.find.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.forEach.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.includes.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.map.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.join.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.slice.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Array.prototype.pop.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array.prototype.push.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.shift.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.reverse.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.unshift.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("Array().length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array(1, 2)[1];"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let array = new Array('x'); array[0];"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(eval("Array.isArray([]);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Array.isArray({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Array.isArray('abc');"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("Array.prototype.constructor === Array;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("[] instanceof Array;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.prototype.isPrototypeOf([]);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf([]) === Array.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 'x', true].join();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].join('|');"),
        Ok(Value::String("1|2|3".to_owned()))
    );
    assert_eq!(
        eval("[1, null, undefined, 4].join('-');"),
        Ok(Value::String("1---4".to_owned()))
    );
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
        eval("[1, 'x', true].toString();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, [2, 3], 4].join(';');"),
        Ok(Value::String("1;2,3;4".to_owned()))
    );
    assert_eq!(eval("[1, 2, 1].indexOf(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, 1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -5);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 3].indexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].indexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[false, 'false'].indexOf('false');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1, 1);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -2);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -5);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(eval("[1, 2, 3].lastIndexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].lastIndexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(1, 4).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(2).join('|');"),
        Ok(Value::String("2|3|4".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(-3, -1).join();"),
        Ok(Value::String("2,3".to_owned()))
    );
    assert_eq!(eval("[0, 1, 2].slice(5).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("let copy = [1, 2].slice(); Array.isArray(copy) && copy[1] === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[0].concat([1, 2], 3, [4]).join();"),
        Ok(Value::String("0,1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval("[].concat([0, 1], [2, 3]).length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(eval("[0].concat('x', true)[2];"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4, 5]; let result = xs.copyWithin(0, 3); result === xs && xs.join();"
        ),
        Ok(Value::String("4,5,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(1, 3, 4).join();"),
        Ok(Value::String("1,4,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(-2, 0, 2).join();"),
        Ok(Value::String("1,2,3,1,2".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].copyWithin(1, 0, 3).join();"),
        Ok(Value::String("1,1,2,3".to_owned()))
    );
    assert_eq!(eval("[1, 2, 3].at(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(2);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-1);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-3);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at(-4);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at();"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(1.9);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 3].at(-1.9);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let xs = [1]; xs.push(2, 3) + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3:1,2,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.pop() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("2:1:1".to_owned()))
    );
    assert_eq!(eval("[].pop();"), Ok(Value::Undefined));
    assert_eq!(
        eval("let xs = [1, 2]; xs.shift() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("1:1:2".to_owned()))
    );
    assert_eq!(eval("[].shift();"), Ok(Value::Undefined));
    assert_eq!(
        eval("let xs = [3]; xs.unshift(1, 2) + ':' + xs.join();"),
        Ok(Value::String("3:1,2,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.reverse(); result === xs && xs.join();"),
        Ok(Value::String("3,2,1".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.fill(9); result === xs && xs.join();"),
        Ok(Value::String("9,9,9".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, 1, 3).join();"),
        Ok(Value::String("1,0,0,4".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, -3, -1).join();"),
        Ok(Value::String("1,0,0,4".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill(0, 5).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill().join();"),
        Ok(Value::String(",,".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, undefined, 3]; xs.reverse(); xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3,,1".to_owned()))
    );
    assert_eq!(
        eval("let xs = [2]; let ys = xs; ys.unshift(1); xs.shift() + ':' + xs.join();"),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1]; let ys = xs; ys.push(2); xs.join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1]; xs[2] = 3; xs.length + ':' + xs.join();"),
        Ok(Value::String("3:1,,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; xs.length = 1; xs.join();"),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(eval("[1, 2, 3].includes(2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].includes(4);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(1, 1);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(3, -1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[0 / 0].includes(0 / 0);"), Ok(Value::Boolean(true)));
    assert!(eval("Array(3);").is_err());
}

#[test]
fn evaluates_array_literals() {
    assert_eq!(
        eval("let xs = [1, 2 + 3, true]; xs.length + ':' + xs[0] + ':' + xs[1] + ':' + xs[2];"),
        Ok(Value::String("3:1:5:true".to_owned()))
    );
    assert_eq!(eval("[] === [];"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let xs = []; let ys = xs; xs === ys;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_member_access() {
    assert_eq!(eval("let xs = [1, 2 + 3]; xs[1];"), Ok(Value::Number(5.0)));
    assert_eq!(eval("[1, 2, 3].length;"), Ok(Value::Number(3.0)));
}
