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
        eval("let result = Object.assign('a'); typeof result + ':' + result.valueOf();"),
        Ok(Value::String("object:a".to_owned()))
    );
    assert_eq!(
        eval("let result = Object.assign(1, { a: 2 }); result.valueOf() + result.a;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let result = Object.assign(true, { a: 2 }); result.valueOf() && result.a === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.assign(target, Object.create({ inherited: 1 })); Object.keys(target).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let target = []; let result = Object.assign(target, { 0: 'a', 2: 'c' }); (result === target) + ':' + target.length + ':' + target.join('-');"
        ),
        Ok(Value::String("true:3:a--c".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = [1]; Object.assign(target, { label: 'ok' }); target.length + ':' + target.label;"
        ),
        Ok(Value::String("1:ok".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = [1, 2, 3]; Object.assign(target, { length: 1 }); target.length + ':' + target.join('-');"
        ),
        Ok(Value::String("1:1".to_owned()))
    );
    assert!(
        eval(
            "let target = {}; Object.defineProperty(target, 'attr', { value: 0, writable: false }); Object.assign(target, { attr: 1 });"
        )
        .is_err()
    );
    assert!(eval("Object.assign('a', [1]);").is_err());
    assert_eq!(
        eval(
            "let target = {}; let seen = 0; Object.defineProperty(target, 'attr', { set: function(value) { seen = value; } }); Object.assign(target, { attr: 3 }); seen;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let target = 12; let result = Object.assign(target, 'aaa', 'bb2b', '1c'); Object.getOwnPropertyNames(result).length + ':' + result[0] + ':' + result[1] + ':' + result[2] + ':' + result[3];"
        ),
        Ok(Value::String("4:1:c:2:b".to_owned()))
    );
}
