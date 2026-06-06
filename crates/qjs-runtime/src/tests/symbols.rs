use crate::{Value, eval};

#[test]
fn evaluates_symbol_prototype_builtins() {
    assert_eq!(
        eval("typeof Symbol;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Symbol.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Symbol('test').toString();"),
        Ok(Value::String("Symbol(test)".to_owned()))
    );
    assert_eq!(
        eval("String(Symbol('test'));"),
        Ok(Value::String("Symbol(test)".to_owned()))
    );
    assert_eq!(
        eval("Symbol().toString();"),
        Ok(Value::String("Symbol()".to_owned()))
    );
    assert_eq!(
        eval("Symbol('test').description;"),
        Ok(Value::String("test".to_owned()))
    );
    assert_eq!(eval("Symbol().description;"), Ok(Value::Undefined));
    assert_eq!(eval("Symbol(undefined).description;"), Ok(Value::Undefined));
    assert_eq!(
        eval("Symbol('').description;"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("let symbol = Symbol('id'); symbol.valueOf() === symbol;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(Symbol.prototype, 'description').get; get.call(Symbol('x'));"
        ),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(Symbol('x'));"),
        Ok(Value::String("[object Symbol]".to_owned()))
    );
    assert!(eval("Symbol.prototype.toString.call({});").is_err());
    assert!(eval("Symbol.prototype.valueOf.call({});").is_err());
}
