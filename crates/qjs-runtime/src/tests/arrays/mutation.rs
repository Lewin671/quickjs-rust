use crate::{Value, eval};

#[test]
fn evaluates_array_mutation_builtins() {
    assert_eq!(
        eval("let xs = [1]; xs.push(2, 3) + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3:1,2,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { 0: 1, length: 1 }; Array.prototype.push.call(object, 2, 3) + ':' + object.length + ':' + object[2];"
        ),
        Ok(Value::String("3:3:3".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.push.call(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Array.prototype.push.call('', 1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.pop() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("2:1:1".to_owned()))
    );
    assert_eq!(eval("[].pop();"), Ok(Value::Undefined));
    assert_eq!(
        eval(
            "let object = { 0: 1, 1: 2, length: 2 }; Array.prototype.pop.call(object) + ':' + object.length + ':' + object.hasOwnProperty('1');"
        ),
        Ok(Value::String("2:1:false".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.pop.call(false);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let proto = { length: 2 }; let object = Object.create(proto); object[1] = 7; Array.prototype.pop.call(object) + ':' + object.length + ':' + proto.length;"
        ),
        Ok(Value::String("7:1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let object = { 0: 1, length: 1 }; Object.defineProperty(object, 'length', { set: function(value) { calls = value + 1; }, configurable: true }); Array.prototype.pop.call(object); calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert!(
        eval("let object = { length: 1 }; Object.defineProperty(object, '0', { value: 1 }); Array.prototype.pop.call(object);").is_err()
    );
    assert!(
        eval("let object = { 0: 1 }; Object.defineProperty(object, 'length', { value: 1 }); Array.prototype.pop.call(object);").is_err()
    );
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
}
