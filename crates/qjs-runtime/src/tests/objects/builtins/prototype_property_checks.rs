use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_property_checks() {
    assert_eq!(
        eval("Object.keys('ab')[1];"),
        Ok(Value::String("1".to_owned().into()))
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
        eval("Object.prototype.__defineGetter__.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.prototype.__defineSetter__.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.prototype.__defineGetter__.name;"),
        Ok(Value::String("__defineGetter__".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.__defineSetter__.name;"),
        Ok(Value::String("__defineSetter__".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; \
             function getter() { return 7; } \
             let result = object.__defineGetter__('value', getter); \
             let desc = Object.getOwnPropertyDescriptor(object, 'value'); \
             [result, object.value, desc.get.name, desc.set, desc.enumerable, desc.configurable].join('|');"
        ),
        Ok(Value::String("|7|getter||true|true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; \
             function setter(value) { this.stored = value; } \
             let result = object.__defineSetter__('value', setter); \
             object.value = 9; \
             let desc = Object.getOwnPropertyDescriptor(object, 'value'); \
             [result, object.stored, desc.get, desc.set.name, desc.enumerable, desc.configurable].join('|');"
        ),
        Ok(Value::String("|9||setter|true|true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; \
             function originalGet() { return 1; } \
             function originalSet(value) {} \
             function newGet() { return 2; } \
             Object.defineProperty(object, 'value', { get: originalGet, set: originalSet, enumerable: false, configurable: true }); \
             object.__defineGetter__('value', newGet); \
             let desc = Object.getOwnPropertyDescriptor(object, 'value'); \
             [object.value, desc.set.name, desc.enumerable, desc.configurable].join('|');"
        ),
        Ok(Value::String("2|originalSet|true|true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; \
             function originalGet() { return 1; } \
             function originalSet(value) {} \
             function newSet(value) { this.stored = value; } \
             Object.defineProperty(object, 'value', { get: originalGet, set: originalSet, enumerable: false, configurable: true }); \
             object.__defineSetter__('value', newSet); \
             object.value = 3; \
             let desc = Object.getOwnPropertyDescriptor(object, 'value'); \
             [object.value, object.stored, desc.get.name, desc.enumerable, desc.configurable].join('|');"
        ),
        Ok(Value::String("1|3|originalGet|true|true".to_owned().into()))
    );
    assert!(eval("({}).__defineGetter__('value', 1);").is_err());
    assert!(eval("({}).__defineSetter__('value', 1);").is_err());
    assert!(
        eval("let key = { toString() { throw new Error('key'); } }; ({}).__defineGetter__(key, function() {});").is_err()
    );
    assert!(eval("Object.prototype.__defineGetter__.call(null, 'value', function() {});").is_err());
    assert!(
        eval("let object = {}; Object.preventExtensions(object); object.__defineGetter__('value', function() {});").is_err()
    );
    assert!(
        eval("let object = {}; Object.defineProperty(object, 'value', { value: 1, configurable: false }); object.__defineSetter__('value', function() {});").is_err()
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { get: function getter() { return 1; }, configurable: true }); object.__lookupGetter__('value').name;"
        ),
        Ok(Value::String("getter".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { set: function setter(v) {}, configurable: true }); object.__lookupSetter__('value').name;"
        ),
        Ok(Value::String("setter".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = {}; Object.defineProperty(proto, 'value', { get: function inherited() { return 1; }, configurable: true }); Object.create(proto).__lookupGetter__('value').name;"
        ),
        Ok(Value::String("inherited".to_owned().into()))
    );
    assert_eq!(
        eval("let object = { value: 1 }; object.__lookupGetter__('value');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let key = Symbol(); let object = {}; Object.defineProperty(object, key, { get: function symbolic() { return 1; }, configurable: true }); object.__lookupGetter__(key).name;"
        ),
        Ok(Value::String("symbolic".to_owned().into()))
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
        Ok(Value::String("false|false||Test262".to_owned().into()))
    );
    assert_eq!(
        eval(
            "class C { set #m(value) { this.value = value; } check() { return [this.hasOwnProperty('#m'), '#m' in this, this.__lookupSetter__('#m')].join('|'); } } new C().check();"
        ),
        Ok(Value::String("false|false|".to_owned().into()))
    );
}
