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
    assert_eq!(
        eval("typeof Symbol.toStringTag;"),
        Ok(Value::String("symbol".to_owned()))
    );
    assert_eq!(
        eval(
            "let descriptor = Object.getOwnPropertyDescriptor(Symbol, 'toStringTag'); descriptor.writable + ':' + descriptor.enumerable + ':' + descriptor.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned()))
    );
    assert!(eval("Symbol.prototype.toString.call({});").is_err());
    assert!(eval("Symbol.prototype.valueOf.call({});").is_err());
}

#[test]
fn evaluates_symbol_registry_builtins() {
    assert_eq!(
        eval("typeof Symbol.for;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Symbol.for.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("typeof Symbol.keyFor;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Symbol.keyFor.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let first = Symbol.for('shared'); let second = Symbol.for('shared'); first === second;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Symbol.for('shared') === Symbol('shared');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let symbol = Symbol.for('shared'); Symbol.keyFor(symbol);"),
        Ok(Value::String("shared".to_owned()))
    );
    assert_eq!(
        eval("Symbol.keyFor(Symbol('local'));"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let symbol = Symbol.for(7); symbol.description + ':' + Symbol.keyFor(symbol);"),
        Ok(Value::String("7:7".to_owned()))
    );
    assert!(eval("Symbol.keyFor({});").is_err());
}

#[test]
fn exposes_builtin_to_string_tag_symbol_properties() {
    assert_eq!(
        eval(
            "function attrs(object) { let d = Object.getOwnPropertyDescriptor(object, Symbol.toStringTag); return d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable; } attrs(Symbol.prototype) + '|' + attrs(Map.prototype) + '|' + attrs(Set.prototype) + '|' + attrs(WeakMap.prototype) + '|' + attrs(WeakSet.prototype) + '|' + attrs(Promise.prototype) + '|' + attrs(Math) + '|' + attrs(JSON);"
        ),
        Ok(Value::String(
            "Symbol:false:false:true|Map:false:false:true|Set:false:false:true|WeakMap:false:false:true|WeakSet:false:false:true|Promise:false:false:true|Math:false:false:true|JSON:false:false:true".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "Map.prototype[Symbol.toStringTag] = 'Changed'; Object.prototype.toString.call(new Map()) + ':' + Map.prototype[Symbol.toStringTag];"
        ),
        Ok(Value::String("[object Map]:Map".to_owned()))
    );
    assert_eq!(
        eval(
            "let deleted = delete Set.prototype[Symbol.toStringTag]; deleted + ':' + (Object.getOwnPropertyDescriptor(Set.prototype, Symbol.toStringTag) === undefined) + ':' + Object.prototype.toString.call(new Set());"
        ),
        Ok(Value::String("true:true:[object Object]".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.getOwnPropertyDescriptor(RegExp.prototype, Symbol.toStringTag) === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
}
