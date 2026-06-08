use crate::{Value, eval};

#[test]
fn evaluates_weak_set_constructor_and_prototype() {
    assert_eq!(
        eval("typeof WeakSet;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("WeakSet.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("WeakSet.prototype.constructor === WeakSet;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new WeakSet() instanceof WeakSet;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new WeakSet());"),
        Ok(Value::String("[object WeakSet]".to_owned()))
    );
    assert_eq!(eval("WeakSet.prototype.size;"), Ok(Value::Undefined));
    assert_eq!(
        eval("let set = new WeakSet(); set.extra = 7; set.extra;"),
        Ok(Value::Number(7.0))
    );
    assert!(eval("WeakSet();").is_err());
}

#[test]
fn evaluates_weak_set_iterable_constructor_arguments() {
    assert_eq!(
        eval("let key = {}; let set = new WeakSet([key]); set.has(key);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let key = {}; let values = [key]; let iterable = {}; iterable[Symbol.iterator] = function() { return values[Symbol.iterator](); }; let set = new WeakSet(iterable); set.has(key);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let key = {}; let set = new WeakSet([key, key]); set.delete(key) + ':' + set.has(key);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert!(eval("new WeakSet([1]);").is_err());
    assert!(eval("new WeakSet({});").is_err());
}

#[test]
fn weak_set_constructor_uses_prototype_add_adder() {
    assert_eq!(
        eval(
            "let first = {}; let second = {}; let calls = 0; let original = WeakSet.prototype.add; WeakSet.prototype.add = function(value) { calls = calls + 1; return original.call(this, value); }; let set = new WeakSet([first, second]); calls + ':' + set.has(first) + ':' + set.has(second);"
        ),
        Ok(Value::String("2:true:true".to_owned()))
    );
    assert!(
        eval(
            "Object.defineProperty(WeakSet.prototype, 'add', { get: function() { throw new TypeError('boom'); } }); new WeakSet([]);"
        )
        .is_err()
    );
    assert!(eval("WeakSet.prototype.add = null; new WeakSet([]);").is_err());
}

#[test]
fn evaluates_weak_set_basic_methods() {
    assert_eq!(
        eval("let key = {}; let set = new WeakSet(); set.add(key) === set;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let key = {}; let set = new WeakSet(); set.add(key); set.has(key);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let key = {}; let set = new WeakSet(); set.add(key); set.delete(key) + ':' + set.delete(key) + ':' + set.has(key);"
        ),
        Ok(Value::String("true:false:false".to_owned()))
    );
}

#[test]
fn evaluates_weak_set_object_value_identity() {
    assert_eq!(
        eval(
            "let a = {}; let b = {}; let set = new WeakSet(); set.add(a); set.has(a) + ':' + set.has(b);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let fn = function() {}; let set = new WeakSet(); set.add(array); set.add(fn); set.has(array) + ':' + set.has(fn);"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
}

#[test]
fn rejects_weak_set_invalid_receivers_and_primitive_add_values() {
    assert!(eval("WeakSet.prototype.has.call({}, {});").is_err());
    assert!(eval("WeakSet.prototype.add.call({}, {});").is_err());
    assert!(eval("new WeakSet().add('key');").is_err());
    assert_eq!(
        eval("let set = new WeakSet(); !set.has('key') && !set.delete('key');"),
        Ok(Value::Boolean(true))
    );
}
