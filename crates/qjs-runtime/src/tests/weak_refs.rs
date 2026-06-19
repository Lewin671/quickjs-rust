use crate::{Value, eval};

#[test]
fn evaluates_finalization_registry_constructor_and_prototype() {
    assert_eq!(
        eval("typeof FinalizationRegistry;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("FinalizationRegistry.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("FinalizationRegistry.prototype.constructor === FinalizationRegistry;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(FinalizationRegistry.prototype) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new FinalizationRegistry(function() {}));"),
        Ok(Value::String(
            "[object FinalizationRegistry]".to_owned().into()
        ))
    );
    assert!(eval("FinalizationRegistry(function() {});").is_err());
    assert!(eval("new FinalizationRegistry(1);").is_err());
}

#[test]
fn finalization_registry_surface_descriptors_match_spec() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(FinalizationRegistry, 'prototype'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(FinalizationRegistry.prototype, 'constructor'); \
             (d.value === FinalizationRegistry) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(FinalizationRegistry.prototype, Symbol.toStringTag); \
             d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "FinalizationRegistry:false:false:true".to_owned().into()
        ))
    );
}

#[test]
fn finalization_registry_register_validates_targets_and_tokens() {
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             let target = {}; \
             registry.register(target, 'held');"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             let target = {}; \
             let token = {}; \
             registry.register(target, undefined, token);"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             registry.register(Symbol('target'), 'held', Symbol('token'));"
        ),
        Ok(Value::Undefined)
    );
    assert!(eval("new FinalizationRegistry(function() {}).register(1, 'held');").is_err());
    assert!(
        eval("let target = {}; new FinalizationRegistry(function() {}).register(target, target);")
            .is_err()
    );
    assert!(eval("new FinalizationRegistry(function() {}).register({}, 'held', 1);").is_err());
    assert!(
        eval("new FinalizationRegistry(function() {}).register(Symbol.for('target'), 'held');")
            .is_err()
    );
    assert!(
        eval("new FinalizationRegistry(function() {}).register({}, 'held', Symbol.for('token'));")
            .is_err()
    );
    assert!(eval("FinalizationRegistry.prototype.register.call({}, {}, 'held');").is_err());
}

#[test]
fn finalization_registry_unregister_removes_matching_tokens() {
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             let token1 = {}; \
             let token2 = {}; \
             registry.unregister(token1) + ':' + registry.unregister(token2);"
        ),
        Ok(Value::String("false:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             let target1 = {}; \
             let target2 = {}; \
             let token = {}; \
             registry.register(target1, 'one', token); \
             registry.register(target2, 'two', token); \
             registry.unregister(target1) + ':' + registry.unregister(token) + ':' + registry.unregister(token);"
        ),
        Ok(Value::String("false:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let registry = new FinalizationRegistry(function() {}); \
             let target = {}; \
             let token = Symbol('token'); \
             registry.register(target, 'held', token); \
             registry.unregister(token) + ':' + registry.unregister(token);"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
    assert!(eval("new FinalizationRegistry(function() {}).unregister(undefined);").is_err());
    assert!(
        eval("new FinalizationRegistry(function() {}).unregister(Symbol.for('token'));").is_err()
    );
    assert!(eval("FinalizationRegistry.prototype.unregister.call({}, {});").is_err());
}

#[test]
fn finalization_registry_method_descriptors_match_spec() {
    assert_eq!(
        eval("FinalizationRegistry.prototype.register.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("FinalizationRegistry.prototype.unregister.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(FinalizationRegistry.prototype, 'register'); \
             d.value.name + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("register:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(FinalizationRegistry.prototype, 'unregister'); \
             d.value.name + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "unregister:true:false:true".to_owned().into()
        ))
    );
    assert!(eval("new FinalizationRegistry.prototype.register({}, 'held');").is_err());
    assert!(eval("new FinalizationRegistry.prototype.unregister({});").is_err());
}
