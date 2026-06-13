use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_property_checks() {
    assert_eq!(
        eval("Object.keys('ab')[1];"),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(eval("Object.keys(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("({ value: 1 }).hasOwnProperty('missing');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); object.hasOwnProperty('value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("[1, 2].hasOwnProperty('1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'ab'.hasOwnProperty('1');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("({ value: 1 }).propertyIsEnumerable('value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable('toString');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable('propertyIsEnumerable');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); object.propertyIsEnumerable('value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("[1, 2].propertyIsEnumerable('length');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("'ab'.propertyIsEnumerable('1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.__lookupGetter__.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.prototype.__lookupSetter__.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { get: function getter() { return 1; }, configurable: true }); object.__lookupGetter__('value').name;"
        ),
        Ok(Value::String("getter".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { set: function setter(v) {}, configurable: true }); object.__lookupSetter__('value').name;"
        ),
        Ok(Value::String("setter".to_owned()))
    );
    assert_eq!(
        eval(
            "let proto = {}; Object.defineProperty(proto, 'value', { get: function inherited() { return 1; }, configurable: true }); Object.create(proto).__lookupGetter__('value').name;"
        ),
        Ok(Value::String("inherited".to_owned()))
    );
    assert_eq!(
        eval("let object = { value: 1 }; object.__lookupGetter__('value');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let key = Symbol(); let object = {}; Object.defineProperty(object, key, { get: function symbolic() { return 1; }, configurable: true }); object.__lookupGetter__(key).name;"
        ),
        Ok(Value::String("symbolic".to_owned()))
    );
    assert_eq!(
        eval("'ab'.__lookupGetter__('length');"),
        Ok(Value::Undefined)
    );
    assert!(eval("Object.prototype.__lookupGetter__.call(null, 'x');").is_err());
    assert_eq!(
        eval(
            "class C { get #m() { return 'Test262'; } check() { return [this.hasOwnProperty('#m'), '#m' in this, this.__lookupGetter__('#m'), this.#m].join('|'); } } new C().check();"
        ),
        Ok(Value::String("false|false||Test262".to_owned()))
    );
    assert_eq!(
        eval(
            "class C { set #m(value) { this.value = value; } check() { return [this.hasOwnProperty('#m'), '#m' in this, this.__lookupSetter__('#m')].join('|'); } } new C().check();"
        ),
        Ok(Value::String("false|false|".to_owned()))
    );
}
