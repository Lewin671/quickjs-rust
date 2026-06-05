use crate::{Value, eval};

#[test]
fn evaluates_object_constructor_and_assign() {
    assert_eq!(
        eval("typeof Object;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Object.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.assign.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval(
            "let target = { foo: 1 }; let result = Object.assign(target, { a: 2 }); result === target;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let target = { foo: 1 }; Object.assign(target, { a: 2 }); target.a;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let target = { a: 1 }; Object.assign(target, { a: 5 }, { b: 6 }); target.a + target.b;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval("let target = {}; Object.assign(target, 'ab', null, undefined); target[1];"),
        Ok(Value::String("b".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.assign(target, Object.create({ inherited: 1 })); Object.keys(target).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let result = Object.assign(true, { a: 1 }); typeof result + ':' + result.valueOf() + ':' + result.a;"
        ),
        Ok(Value::String("object:true:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let result = Object.assign('ab'); typeof result + ':' + result.valueOf() + ':' + result.length;"
        ),
        Ok(Value::String("object:ab:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = [7, 8, 9]; let result = Object.assign(target, [1]); (result === target) + ':' + target.join('|');"
        ),
        Ok(Value::String("true:1|8|9".to_owned()))
    );
}
