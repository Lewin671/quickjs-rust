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
fn exposes_map_species_accessor() {
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(Map, Symbol.species); let receiver = {}; [desc.get.call(receiver) === receiver, desc.set === undefined, desc.enumerable, desc.configurable, desc.get.name, desc.get.length].join(':');"
        ),
        Ok(Value::String(
            "true:true:false:true:get [Symbol.species]:0".to_owned()
        ))
    );
}

#[test]
fn evaluates_map_iterable_constructor_arguments() {
    assert_eq!(
        eval(
            "let map = new Map([['attr', 1], ['foo', 2]]); map.size + ':' + map.get('attr') + ':' + map.get('foo');"
        ),
        Ok(Value::String("2:1:2".to_owned()))
    );
    assert_eq!(
        eval("let map = new Map({ 0: ['a', 1], 1: ['b', 2], length: 2 }); map.get('b');"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let map = new Map([['a', 1], ['a', 2]]); map.size + ':' + map.get('a');"),
        Ok(Value::String("1:2".to_owned()))
    );
    assert!(eval("new Map([1]);").is_err());
    assert!(eval("new Map(['']);").is_err());
    assert!(eval("new Map([Symbol('a')]);").is_err());
    assert!(eval("new Map([null]);").is_err());
}

#[test]
fn map_constructor_uses_prototype_set_adder() {
    assert_eq!(
        eval(
            "let original = Map.prototype.set; let calls = 0; let receivers = []; let seen = ''; Map.prototype.set = function(key, value) { calls = calls + 1; receivers.push(this); seen = seen + key + ':' + value + '|'; return original.call(this, key, value); }; let map = new Map([['a', 1], ['b', 2]]); calls + ':' + seen + ':' + (receivers[0] === map) + ':' + (receivers[1] === map) + ':' + map.get('b');"
        ),
        Ok(Value::String("2:a:1|b:2|:true:true:2".to_owned()))
    );
    assert!(eval("Map.prototype.set = null; new Map([[1, 1]]);").is_err());
    assert!(
        eval(
            "Object.defineProperty(Map.prototype, 'set', { get: function() { throw new TypeError('boom'); } }); new Map([]);"
        )
        .is_err()
    );
}

#[test]
fn evaluates_map_group_by_arrays() {
    assert_eq!(eval("Map.groupBy.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval(
            "let groups = Map.groupBy([1, 2, 3, 4], function(value) { return value % 2; }); groups instanceof Map && groups.get(1).join('|') + ':' + groups.get(0).join('|');"
        ),
        Ok(Value::String("1|3:2|4".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; let groups = Map.groupBy({ 0: 'a', 1: 'b', length: 2 }, function(value, index) { seen = seen + value + index; return index; }); groups.size + ':' + groups.get(0)[0] + ':' + groups.get(1)[0] + ':' + seen;"
        ),
        Ok(Value::String("2:a:b:a0b1".to_owned()))
    );
    assert_eq!(
        eval(
            "let key = {}; let groups = Map.groupBy(['x', 'y'], function(value) { return value === 'x' ? key : {}; }); groups.get(key)[0] + ':' + groups.size;"
        ),
        Ok(Value::String("x:2".to_owned()))
    );
    assert!(eval("Map.groupBy([1], 1);").is_err());
    assert!(eval("Map.groupBy(undefined, function(value) { return value; });").is_err());
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
fn evaluates_map_get_or_insert_methods() {
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.getOrInsert('a', 2) + ':' + map.get('a') + ':' + map.size;"
        ),
        Ok(Value::String("1:1:1".to_owned()))
    );
    assert_eq!(
        eval("let map = new Map(); map.getOrInsert('a', 2) + ':' + map.get('a') + ':' + map.size;"),
        Ok(Value::String("2:2:1".to_owned()))
    );
    assert_eq!(
        eval("Map.prototype.getOrInsert.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let calls = 0; let map = new Map(); map.set('a', 1); map.getOrInsertComputed('a', function(key) { calls = calls + 1; return 2; }) + ':' + calls + ':' + map.get('a');"
        ),
        Ok(Value::String("1:0:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; let map = new Map(); map.getOrInsertComputed(-0, function(key) { seen = String(1 / key); return 3; }) + ':' + map.get(0) + ':' + seen;"
        ),
        Ok(Value::String("3:3:Infinity".to_owned()))
    );
    assert_eq!(
        eval(
            "let map = new Map(); map.getOrInsertComputed('a', function(key) { map.set(key, 1); return 2; }); map.get('a') + ':' + map.size;"
        ),
        Ok(Value::String("2:1".to_owned()))
    );
    assert_eq!(
        eval("Map.prototype.getOrInsertComputed.length;"),
        Ok(Value::Number(2.0))
    );
    assert!(eval("Map.prototype.getOrInsert.call({}, 'a', 1);").is_err());
    assert!(eval("new Map().getOrInsertComputed('a', 1);").is_err());
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
fn evaluates_map_iterators_and_for_each() {
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.set('b', 2); let iterator = map.entries(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.done + ':' + first.value[0] + ':' + first.value[1] + '|' + second.value[0] + ':' + second.value[1] + '|' + last.done + ':' + (last.value === undefined);"
        ),
        Ok(Value::String("false:a:1|b:2|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.set('b', 2); let iterator = map.keys(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.value + ':' + first.done + '|' + second.value + ':' + second.done + '|' + (last.value === undefined) + ':' + last.done;"
        ),
        Ok(Value::String("a:false|b:false|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let map = new Map(); map.set('a', 1); map.set('b', 2); let iterator = map.values(); let first = iterator.next(); let second = iterator.next(); let last = iterator.next(); first.value + ':' + first.done + '|' + second.value + ':' + second.done + '|' + (last.value === undefined) + ':' + last.done;"
        ),
        Ok(Value::String("1:false|2:false|true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; let thisArg = { marker: 'ctx' }; let map = new Map(); map.set('a', 1); map.set('b', 2); let returned = map.forEach(function(value, key, receiver) { seen = seen + this.marker + ':' + key + ':' + value + ':' + (receiver === map) + '|'; }, thisArg); seen + ':' + (returned === undefined);"
        ),
        Ok(Value::String("ctx:a:1:true|ctx:b:2:true|:true".to_owned()))
    );
    assert!(eval("Map.prototype.entries.call({});").is_err());
    assert!(eval("Map.prototype.forEach.call(new Map(), 1);").is_err());
}

#[test]
fn rejects_map_methods_with_incompatible_receivers() {
    assert!(eval("Map.prototype.get.call({}, 'x');").is_err());
    assert!(eval("Map.prototype.size;").is_err());
}
