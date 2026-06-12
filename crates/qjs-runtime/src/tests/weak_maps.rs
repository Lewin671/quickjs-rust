use crate::{Value, eval};

#[test]
fn evaluates_weak_map_constructor_and_prototype() {
    assert_eq!(
        eval("typeof WeakMap;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("WeakMap.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("WeakMap.prototype.constructor === WeakMap;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new WeakMap() instanceof WeakMap;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new WeakMap());"),
        Ok(Value::String("[object WeakMap]".to_owned()))
    );
    assert_eq!(eval("WeakMap.prototype.size;"), Ok(Value::Undefined));
    assert_eq!(
        eval("let map = new WeakMap(); map.extra = 7; map.extra;"),
        Ok(Value::Number(7.0))
    );
    assert!(eval("WeakMap();").is_err());
}

#[test]
fn evaluates_weak_map_iterable_constructor_arguments() {
    assert_eq!(
        eval("let key = {}; let map = new WeakMap([[key, 1]]); map.get(key);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let key = {}; let entries = [[key, 1]]; let iterable = {}; iterable[Symbol.iterator] = function() { return entries[Symbol.iterator](); }; let map = new WeakMap(iterable); map.has(key);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let key = {}; let map = new WeakMap([[key, 1], [key, 2]]); map.get(key);"),
        Ok(Value::Number(2.0))
    );
    assert!(eval("new WeakMap([1]);").is_err());
    assert!(eval("new WeakMap([[1, 2]]);").is_err());
    assert!(eval("new WeakMap({});").is_err());
}

#[test]
fn weak_map_constructor_uses_prototype_set_adder() {
    assert_eq!(
        eval(
            "let first = {}; let second = {}; let calls = 0; let original = WeakMap.prototype.set; WeakMap.prototype.set = function(key, value) { calls = calls + 1; return original.call(this, key, value); }; let map = new WeakMap([[first, 1], [second, 2]]); calls + ':' + map.get(first) + ':' + map.get(second);"
        ),
        Ok(Value::String("2:1:2".to_owned()))
    );
    assert!(
        eval(
            "Object.defineProperty(WeakMap.prototype, 'set', { get: function() { throw new TypeError('boom'); } }); new WeakMap([]);"
        )
        .is_err()
    );
    assert!(eval("WeakMap.prototype.set = null; new WeakMap([]);").is_err());
}

#[test]
fn evaluates_weak_map_basic_methods() {
    assert_eq!(
        eval("let key = {}; let map = new WeakMap(); map.set(key, 1) === map;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let key = {}; let map = new WeakMap(); map.set(key, 1); map.get(key);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let key = {}; let map = new WeakMap(); map.set(key, 1); map.has(key);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let key = {}; let map = new WeakMap(); map.set(key, 1); map.set(key, 2); map.get(key);"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let key = {}; let map = new WeakMap(); map.set(key, 1); map.delete(key) + ':' + map.delete(key) + ':' + map.has(key);"
        ),
        Ok(Value::String("true:false:false".to_owned()))
    );
}

#[test]
fn evaluates_weak_map_get_or_insert() {
    assert_eq!(
        eval(
            "let key = {}; let map = new WeakMap(); map.set(key, 1); map.getOrInsert(key, 2) + ':' + map.get(key);"
        ),
        Ok(Value::String("1:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let key = {}; let map = new WeakMap(); map.getOrInsert(key, 2) + ':' + map.get(key);"
        ),
        Ok(Value::String("2:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let key = Symbol('key'); let map = new WeakMap(); map.getOrInsert(key, 3) + ':' + map.get(key);"
        ),
        Ok(Value::String("3:3".to_owned()))
    );
    assert_eq!(
        eval("WeakMap.prototype.getOrInsert.length;"),
        Ok(Value::Number(2.0))
    );
    assert!(eval("WeakMap.prototype.getOrInsert.call({}, {}, 1);").is_err());
    assert!(eval("new WeakMap().getOrInsert('key', 1);").is_err());
    assert!(eval("new WeakMap().getOrInsert(Symbol.for('key'), 1);").is_err());
}

#[test]
fn evaluates_weak_map_object_key_identity() {
    assert_eq!(
        eval(
            "let a = {}; let b = {}; let map = new WeakMap(); map.set(a, 1); map.get(a) + ':' + map.has(b);"
        ),
        Ok(Value::String("1:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let fn = function() {}; let map = new WeakMap(); map.set(array, 1); map.set(fn, 2); map.get(array) + ':' + map.get(fn);"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
}

#[test]
fn rejects_weak_map_invalid_receivers_and_primitive_set_keys() {
    assert!(eval("WeakMap.prototype.get.call({}, {});").is_err());
    assert!(eval("WeakMap.prototype.set.call({}, {}, 1);").is_err());
    assert!(eval("new WeakMap().set('key', 1);").is_err());
    assert_eq!(
        eval("let key = Symbol('key'); let map = new WeakMap(); map.set(key, 1); map.get(key);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let map = new WeakMap(); let caught = false; try { map.set(Symbol.for('key'), 1); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let map = new WeakMap(); map.get('key') === undefined && !map.has('key') && !map.delete('key');"
        ),
        Ok(Value::Boolean(true))
    );
}
