use crate::{Value, eval};

#[test]
fn evaluates_array_of_static_constructor() {
    assert_eq!(eval("Array.of.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval(
            "let values = Array.of(1, 'x', true, null, undefined); values.length + ':' + values[0] + ':' + values[1] + ':' + values[2] + ':' + (values[3] === null) + ':' + (values[4] === undefined);"
        ),
        Ok(Value::String("5:1:x:true:true:true".to_owned()))
    );
    assert_eq!(eval("Array.of(3).length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.of(3)[0];"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Array.isArray(Array.of(1, 2));"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_from_static_constructor() {
    assert_eq!(eval("Array.from.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let source = [0, 'foo', undefined, Infinity]; let result = Array.from(source); result.length + ':' + result[0] + ':' + result[1] + ':' + (result[2] === undefined) + ':' + result[3] + ':' + (result === source);"
        ),
        Ok(Value::String("4:0:foo:true:Infinity:false".to_owned()))
    );
    assert_eq!(
        eval("Array.from('Test').join('');"),
        Ok(Value::String("Test".to_owned()))
    );
    assert_eq!(
        eval("Array.from({ length: 3, 0: 'a', 2: 'c' }).join('|');"),
        Ok(Value::String("a||c".to_owned()))
    );
}

#[test]
fn exposes_array_species_accessor() {
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(Array, Symbol.species); let receiver = {}; [desc.get.call(receiver) === receiver, desc.set === undefined, desc.enumerable, desc.configurable, desc.get.name, desc.get.length].join(':');"
        ),
        Ok(Value::String(
            "true:true:false:true:get [Symbol.species]:0".to_owned()
        ))
    );
}

#[test]
fn evaluates_array_from_mapping() {
    assert_eq!(
        eval("Array.from([1, 2], function(value, index) { return value + index; }).join();"),
        Ok(Value::String("1,3".to_owned()))
    );
    assert_eq!(
        eval("Array.from([1], function(value) { return value + this.offset; }, { offset: 4 })[0];"),
        Ok(Value::Number(5.0))
    );
    assert!(eval("Array.from([1], null);").is_err());
    assert!(eval("Array.from(null);").is_err());
}
