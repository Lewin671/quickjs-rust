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
    assert!(eval("'' + Symbol('test');").is_err());
    assert!(eval("Symbol(Symbol('test'));").is_err());
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
        eval("typeof Symbol.iterator;"),
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
fn exposes_well_known_symbol_static_properties() {
    assert_eq!(
        eval(
            "let names = ['asyncDispose', 'asyncIterator', 'dispose', 'hasInstance', 'isConcatSpreadable', 'iterator', 'match', 'matchAll', 'replace', 'search', 'species', 'split', 'toPrimitive', 'toStringTag', 'unscopables']; names.every(function(name) { let d = Object.getOwnPropertyDescriptor(Symbol, name); return typeof Symbol[name] === 'symbol' && d.writable === false && d.enumerable === false && d.configurable === false && String(Symbol[name]) === 'Symbol(Symbol.' + name + ')' && Symbol.keyFor(Symbol[name]) === undefined; });"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Symbol.keyFor(Symbol.hasInstance) === undefined && Symbol.hasInstance !== Symbol('Symbol.hasInstance');"
        ),
        Ok(Value::Boolean(true))
    );
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

#[test]
fn exposes_builtin_iterator_symbol_properties() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Symbol, 'iterator'); typeof Symbol.iterator + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("symbol:false:false:false".to_owned()))
    );
    assert_eq!(
        eval(
            "function attrs(object, method) { let d = Object.getOwnPropertyDescriptor(object, Symbol.iterator); return (d.value === object[method]) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable; } attrs(Array.prototype, 'values') + '|' + attrs(Map.prototype, 'entries') + '|' + attrs(Set.prototype, 'values');"
        ),
        Ok(Value::String(
            "true:true:false:true|true:true:false:true|true:true:false:true".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "let iterator = [5][Symbol.iterator](); let first = iterator.next(); first.value + ':' + first.done + ':' + iterator.next().done;"
        ),
        Ok(Value::String("5:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let iterator = new Map([['k', 7]])[Symbol.iterator](); let first = iterator.next(); first.value[0] + ':' + first.value[1] + ':' + first.done;"
        ),
        Ok(Value::String("k:7:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let iterator = new Set(['v'])[Symbol.iterator](); let first = iterator.next(); first.value + ':' + first.done;"
        ),
        Ok(Value::String("v:false".to_owned()))
    );
}
