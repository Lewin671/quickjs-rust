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
fn exposes_set_species_accessor() {
    assert_eval(
        "let desc = Object.getOwnPropertyDescriptor(Set, Symbol.species); let receiver = {}; [desc.get.call(receiver) === receiver, desc.set === undefined, desc.enumerable, desc.configurable, desc.get.name, desc.get.length].join(':');",
        Value::String("true:true:false:true:get [Symbol.species]:0".to_owned()),
    );
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
        "var values = ['x', 'y']; var iterable = {}; iterable[Symbol.iterator] = function() { return values[Symbol.iterator](); }; var set = new Set(iterable); set.size + ':' + set.has('x') + ':' + set.has('y');",
        Value::String("2:true:true".to_owned()),
    );
    assert!(eval("new Set({});").is_err());
}

#[test]
fn set_constructor_uses_prototype_add_adder() {
    assert_eval(
        "let original = Set.prototype.add; let calls = 0; let receivers = []; let seen = ''; Set.prototype.add = function(value) { calls = calls + 1; receivers.push(this); seen = seen + value + '|'; return original.call(this, value); }; let set = new Set(['a', 'b']); calls + ':' + seen + ':' + (receivers[0] === set) + ':' + (receivers[1] === set) + ':' + set.has('b');",
        Value::String("2:a|b|:true:true:true".to_owned()),
    );
    assert_eval(
        "let original = Set.prototype.add; Set.prototype.add = null; new Set().size;",
        Value::Number(0.0),
    );
    assert!(eval("Set.prototype.add = null; new Set([1]);").is_err());
    assert!(
        eval(
            "Object.defineProperty(Set.prototype, 'add', { get: function() { throw new TypeError('boom'); } }); new Set([]);"
        )
        .is_err()
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
fn evaluates_set_composition_methods_with_sets() {
    assert_eval(
        "var a = new Set([1, 2]); var b = new Set([2, 3]); var result = a.union(b); var seen = ''; result.forEach(function(value) { seen = seen + value; }); (result instanceof Set) + ':' + result.size + ':' + seen;",
        Value::String("true:3:123".to_owned()),
    );
    assert_eval(
        "var result = new Set([1, 2]).union(new Set([2, 3])); [...result].join('|') + ':' + result.size;",
        Value::String("1|2|3:3".to_owned()),
    );
    assert_eval(
        "var other = { size: 3, has: function() { throw 'has should not be called'; }, keys: function() { return [2, 3, 4].values(); } }; [...new Set([1, 2]).union(other)].join('|');",
        Value::String("1|2|3|4".to_owned()),
    );
    assert_eval(
        "var result = new Set([1, 2]).intersection(new Set([2, 3])); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:2".to_owned()),
    );
    assert_eval(
        "var result = new Set([1, 2]).difference(new Set([2, 3])); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:1".to_owned()),
    );
    assert_eval(
        "var result = new Set([1, 2]).symmetricDifference(new Set([2, 3])); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("2:13".to_owned()),
    );
    assert_eval(
        "new Set([1]).isSubsetOf(new Set([1, 2])) + ':' + new Set([1, 3]).isSubsetOf(new Set([1, 2]));",
        Value::String("true:false".to_owned()),
    );
    assert_eval(
        "new Set([1, 2]).isSupersetOf(new Set([1])) + ':' + new Set([1, 2]).isSupersetOf(new Set([1, 3]));",
        Value::String("true:false".to_owned()),
    );
    assert_eval(
        "new Set([1, 2]).isDisjointFrom(new Set([3])) + ':' + new Set([1, 2]).isDisjointFrom(new Set([2, 3]));",
        Value::String("true:false".to_owned()),
    );
    assert_eval("Set.prototype.union.length;", Value::Number(1.0));
    assert!(eval("Set.prototype.union.call({}, new Set());").is_err());
    assert!(eval("new Set().union({});").is_err());
}

#[test]
fn evaluates_set_composition_methods_with_set_like_objects() {
    assert_eval(
        "var result = new Set([1, 2]).difference(new Map([[2, 'two'], [3, 'three']])); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:1".to_owned()),
    );
    assert_eval(
        "var other = { size: 2, has: function(value) { return value === 2; }, keys: function() { return [2, 3].values(); } }; var result = new Set([1, 2]).difference(other); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:1".to_owned()),
    );
    assert_eval(
        "var other = { size: 2, has: function(value) { return value === 2; }, keys: function* keys() { yield 2; yield 3; } }; [...new Set([1, 2]).union(other)].join('|');",
        Value::String("1|2|3".to_owned()),
    );
    assert_eval(
        "var other = { size: 3, has: function(value) { return value === 2; }, keys: function* keys() { throw 'keys should not be called'; } }; [...new Set([1, 2]).difference(other)].join('|');",
        Value::String("1".to_owned()),
    );
    assert_eval(
        "var other = { size: 3, has: function(value) { return value === 2; }, keys: function() { throw 'keys should not be called'; } }; var result = new Set([1, 2]).difference(other); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:1".to_owned()),
    );
    assert_eval(
        "var other = { size: 1, has: function() { throw 'has should not be called'; }, keys: function() { return [-0].values(); } }; var result = new Set([0, 1]).difference(other); var seen = ''; result.forEach(function(value) { seen = seen + value; }); result.size + ':' + seen;",
        Value::String("1:1".to_owned()),
    );
    assert_eval(
        "var other = { size: 2, has: function(value) { return value === 2; }, keys: function() { return [2, 3].values(); } }; new Set([1, 2]).isSubsetOf(other) + ':' + new Set([1, 2, 3]).isSupersetOf(other) + ':' + new Set([1]).isDisjointFrom(other);",
        Value::String("false:true:true".to_owned()),
    );
    assert_eval(
        "var other = { size: 1, has: function() { throw 'has should not be called'; }, keys: function() { return [-0].values(); } }; var result = new Set([1, 2]).symmetricDifference(other); Object.is([...result][2], 0) + ':' + result.size + ':' + [...result].join('|');",
        Value::String("true:3:1|2|0".to_owned()),
    );
    assert_eval(
        "var other = [5]; other.size = 3; other.has = function() { throw 'has should not be called'; }; other.keys = function() { return [2, 3, 4].values(); }; [...new Set([1, 2]).symmetricDifference(other)].join('|');",
        Value::String("1|3|4".to_owned()),
    );
    assert_eval(
        "var iterator = { values: [4, 5, 6], nextCalls: 0, returnCalls: 0, next: function() { var done = this.nextCalls >= this.values.length; var value = this.values[this.nextCalls]; this.nextCalls = this.nextCalls + 1; return { done: done, value: value }; }, return: function() { this.returnCalls = this.returnCalls + 1; return this; } }; var other = { size: 3, has: function(value) { return iterator.values.includes(value); }, keys: function() { return iterator; } }; var overlaps = new Set([4, 5, 6, 7]).isDisjointFrom(other); var first = overlaps + ':' + iterator.nextCalls + ':' + iterator.returnCalls; iterator.nextCalls = 0; iterator.returnCalls = 0; var disjoint = new Set([0, 1, 2, 3]).isDisjointFrom(other); first + '|' + disjoint + ':' + iterator.nextCalls + ':' + iterator.returnCalls;",
        Value::String("false:1:1|true:4:0".to_owned()),
    );
    assert_eval(
        "var iterator = { values: [4, 5, 6], nextCalls: 0, returnCalls: 0, next: function() { var done = this.nextCalls >= this.values.length; var value = this.values[this.nextCalls]; this.nextCalls = this.nextCalls + 1; return { done: done, value: value }; }, return: function() { this.returnCalls = this.returnCalls + 1; return this; } }; var other = { size: 3, has: function(value) { return iterator.values.includes(value); }, keys: function() { return iterator; } }; var superset = new Set([4, 5, 6, 7]).isSupersetOf(other); var first = superset + ':' + iterator.nextCalls + ':' + iterator.returnCalls; iterator.nextCalls = 0; iterator.returnCalls = 0; var missing = new Set([0, 1, 2, 3]).isSupersetOf(other); first + '|' + missing + ':' + iterator.nextCalls + ':' + iterator.returnCalls;",
        Value::String("true:4:0|false:1:1".to_owned()),
    );
}

#[test]
fn validates_set_like_size_as_number() {
    assert_eval(
        "var calls = 0; var other = { size: { valueOf: function() { calls = calls + 1; return NaN; } }, has: function() {}, keys: function* keys() { yield 1; } }; var caught = false; try { new Set([1]).union(other); } catch (error) { caught = error instanceof TypeError; } caught + ':' + calls;",
        Value::String("true:1".to_owned()),
    );
    assert!(
        eval(
            "var other = { size: undefined, has: function() {}, keys: function* keys() { yield 1; } }; new Set([1]).union(other);"
        )
        .is_err()
    );
    assert!(
        eval(
            "var other = { size: 'string', has: function() {}, keys: function* keys() { yield 1; } }; new Set([1]).union(other);"
        )
        .is_err()
    );
    assert!(
        eval(
            "var other = { size: 0n, has: function() {}, keys: function* keys() { yield 1; } }; new Set([1]).union(other);"
        )
        .is_err()
    );
    assert!(
        eval(
            "var other = { size: -1, has: function() {}, keys: function* keys() { yield 1; } }; new Set([1]).union(other);"
        )
        .is_err()
    );
    assert_eval(
        "var other = { size: Infinity, has: function(value) { return value === 2; }, keys: function() { throw 'keys should not be called'; } }; [...new Set([1, 2]).difference(other)].join('|');",
        Value::String("1".to_owned()),
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
        "Set.prototype.keys === Set.prototype.values;",
        Value::Boolean(true),
    );
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
    assert_eval(
        "var seen = ''; var set = new Set(['a', 'b']); set.forEach(function(value) { if (value === 'a') { set.add('c'); } seen = seen + value + '|'; }); seen;",
        Value::String("a|b|c|".to_owned()),
    );
    assert_eval(
        "var seen = ''; var count = 0; var set = new Set(['a', 'b']); set.forEach(function(value) { if (count === 0) { set.delete('a'); set.add('a'); } seen = seen + value + '|'; count = count + 1; }); seen + ':' + set.size;",
        Value::String("a|b|a|:2".to_owned()),
    );
    assert_eval(
        "var seen = ''; var set = new Set([1]); set.forEach(function(value) { if (value === 1) { set.add(2); } if (value === 2) { set.add(3); } seen = seen + value + '|'; }); seen;",
        Value::String("1|2|3|".to_owned()),
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
