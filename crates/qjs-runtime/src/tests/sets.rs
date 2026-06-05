use crate::{Value, eval};

fn assert_eval(source: &str, expected: Value) {
    assert_eq!(eval(source), Ok(expected));
}

#[test]
fn evaluates_set_constructor_and_prototype() {
    assert_eval("typeof Set;", Value::String("function".to_owned()));
    assert_eval("Set.length;", Value::Number(0.0));
    assert_eval("new Set() instanceof Set;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(new Set());",
        Value::String("[object Set]".to_owned()),
    );
    assert_eval("Set.prototype.constructor === Set;", Value::Boolean(true));
    assert_eval(
        "var set = new Set(); set.extra = 11; set.extra;",
        Value::Number(11.0),
    );
    assert!(eval("Set();").is_err());
}

#[test]
fn evaluates_set_iterable_constructor_arguments() {
    assert_eval(
        "var set = new Set([1, 2]); set.size + ':' + set.has(1) + ':' + set.has(2);",
        Value::String("2:true:true".to_owned()),
    );
    assert_eval(
        "var set = new Set([1, 1, 2]); set.size + ':' + set.has(1) + ':' + set.has(2);",
        Value::String("2:true:true".to_owned()),
    );
    assert_eval(
        "var set = new Set('aba'); set.size + ':' + set.has('a') + ':' + set.has('b');",
        Value::String("2:true:true".to_owned()),
    );
    assert_eval(
        "var set = new Set({ 0: 'x', 1: 'y', length: 2 }); set.size + ':' + set.has('x') + ':' + set.has('y');",
        Value::String("2:true:true".to_owned()),
    );
}

#[test]
fn evaluates_set_basic_methods() {
    assert_eval("var set = new Set(); set.size;", Value::Number(0.0));
    assert_eval(
        "var set = new Set(); set.add('a') === set;",
        Value::Boolean(true),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.add('a'); set.size;",
        Value::Number(1.0),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.has('a') + ':' + set.has('b');",
        Value::String("true:false".to_owned()),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.delete('a') + ':' + set.delete('a') + ':' + set.has('a') + ':' + set.size;",
        Value::String("true:false:false:0".to_owned()),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.add('b'); set.clear(); set.size + ':' + set.has('a');",
        Value::String("0:false".to_owned()),
    );
}

#[test]
fn evaluates_set_same_value_zero_values() {
    assert_eval(
        "var set = new Set(); set.add(NaN); set.add(NaN); set.size + ':' + set.has(NaN);",
        Value::String("1:true".to_owned()),
    );
    assert_eval(
        "var set = new Set(); set.add(-0); set.add(0); set.size + ':' + set.has(-0) + ':' + set.has(0);",
        Value::String("1:true:true".to_owned()),
    );
    assert_eval(
        "var a = {}; var b = {}; var set = new Set(); set.add(a); set.size + ':' + set.has(a) + ':' + set.has(b);",
        Value::String("1:true:false".to_owned()),
    );
}

#[test]
fn evaluates_set_iterators_and_for_each() {
    assert_eval(
        "var set = new Set(); set.add('a'); set.add('b'); var iterator = set.values(); var first = iterator.next(); var second = iterator.next(); var last = iterator.next(); first.value + ':' + first.done + '|' + second.value + ':' + second.done + '|' + (last.value === undefined) + ':' + last.done;",
        Value::String("a:false|b:false|true:true".to_owned()),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.add('b'); var iterator = set.keys(); var first = iterator.next(); var second = iterator.next(); first.value + ':' + second.value;",
        Value::String("a:b".to_owned()),
    );
    assert_eval(
        "var set = new Set(); set.add('a'); set.add('b'); var iterator = set.entries(); var first = iterator.next(); var second = iterator.next(); first.value[0] + ':' + first.value[1] + '|' + second.value[0] + ':' + second.value[1];",
        Value::String("a:a|b:b".to_owned()),
    );
    assert_eval(
        "var seen = ''; var thisArg = { marker: 'ctx' }; var set = new Set(); set.add('a'); set.add('b'); var returned = set.forEach(function(value, key, receiver) { seen = seen + this.marker + ':' + key + ':' + value + ':' + (receiver === set) + '|'; }, thisArg); seen + ':' + (returned === undefined);",
        Value::String("ctx:a:a:true|ctx:b:b:true|:true".to_owned()),
    );
    assert!(eval("Set.prototype.values.call({});").is_err());
    assert!(eval("Set.prototype.forEach.call(new Set(), 1);").is_err());
}

#[test]
fn rejects_set_methods_with_incompatible_receivers() {
    assert!(eval("(function () { return Set.prototype.add.call({}); })();").is_err());
    assert!(eval("(function () { return Set.prototype.clear.call({}); })();").is_err());
    assert!(eval("(function () { return Set.prototype.delete.call({}); })();").is_err());
    assert!(eval("(function () { return Set.prototype.has.call({}); })();").is_err());
    assert!(eval("(function () { return Set.prototype.size; })();").is_err());
}
