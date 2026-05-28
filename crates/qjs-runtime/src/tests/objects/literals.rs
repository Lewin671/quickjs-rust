use crate::{Value, eval};

#[test]
fn evaluates_object_literals_and_member_access() {
    assert_eq!(
        eval("let o = { answer: 40 + 2 }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let answer = 42; let o = { answer }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "let first = 1; let second = 2; let o = { first, second: first + second }; o.first + o.second;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let key = 'answer'; let o = { [key]: 42 }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let o = { [1 + 1]: 'two' }; o[2];"),
        Ok(Value::String("two".to_owned()))
    );
    assert_eq!(
        eval("let object = { value: 7, method() { return this.value; } }; object.method();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let object = { add(a, b) { return a + b; } }; object.add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let method = { method() {} }.method; method.prototype;"),
        Ok(Value::Undefined)
    );
    assert!(eval("let method = { method() {} }.method; new method();").is_err());
    assert_eq!(eval("({ 'a': 1 })['a'];"), Ok(Value::Number(1.0)));
    assert_eq!(eval("({ true: 1 }).true;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("({}).missing;"), Ok(Value::Undefined));
}
