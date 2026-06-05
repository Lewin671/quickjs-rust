use crate::{Value, eval};

#[test]
fn evaluates_map_constructor_and_prototype() {
    assert_eq!(
        eval("typeof Map;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Map.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Map.prototype.constructor === Map;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("new Map() instanceof Map;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.prototype.toString.call(new Map());"),
        Ok(Value::String("[object Map]".to_owned()))
    );
    assert_eq!(
        eval("let map = new Map(); map.extra = 7; map.extra;"),
        Ok(Value::Number(7.0))
    );
    assert!(eval("Map();").is_err());
}

#[test]
fn evaluates_map_basic_methods() {
    assert_eq!(
        eval("let map = new Map(); map.size;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let map = new Map(); map.set('a', 1) === map;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let map = new Map(); map.set('a', 1); map.get('a');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let map = new Map(); map.set('a', 1); map.has('a');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.set('a', 2); map.size + ':' + map.get('a');"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.delete('a') + ':' + map.delete('a') + ':' + map.size;"
        ),
        Ok(Value::String("true:false:0".to_owned()))
    );
    assert_eq!(
        eval("let map = new Map(); map.set('a', 1); map.set('b', 2); map.clear(); map.size;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn evaluates_map_same_value_zero_keys() {
    assert_eq!(
        eval("let map = new Map(); map.set(NaN, 1); map.get(NaN);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let map = new Map(); map.set(-0, 1); map.set(0, 2); map.size + ':' + map.get(-0);"),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = {}; let b = {}; let map = new Map(); map.set(a, 1); map.get(a) + ':' + map.has(b);"
        ),
        Ok(Value::String("1:false".to_owned()))
    );
}

#[test]
fn rejects_map_methods_with_incompatible_receivers() {
    assert!(eval("Map.prototype.get.call({}, 'x');").is_err());
    assert!(eval("Map.prototype.size;").is_err());
}
