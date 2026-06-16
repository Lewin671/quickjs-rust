use crate::{Value, eval};

#[test]
fn disposable_stack_constructor_and_prototype_surface() {
    assert_eq!(
        eval("typeof DisposableStack;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("DisposableStack.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("DisposableStack.prototype.constructor === DisposableStack;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(DisposableStack.prototype) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new DisposableStack());"),
        Ok(Value::String("[object DisposableStack]".to_owned()))
    );
    assert!(eval("DisposableStack();").is_err());
}

#[test]
fn disposable_stack_surface_descriptors_match_spec() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack, 'prototype'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'constructor'); \
             (d.value === DisposableStack) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, Symbol.toStringTag); \
             d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("DisposableStack:false:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, Symbol.dispose); \
             (d.value === DisposableStack.prototype.dispose) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
}

#[test]
fn disposable_stack_empty_dispose_marks_receiver() {
    assert_eq!(
        eval(
            "let stack = new DisposableStack(); \
             let before = stack.disposed; \
             stack.dispose(); \
             before + ':' + stack.disposed;"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'disposed'); typeof d.get + ':' + d.get.name + ':' + d.get.length + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "function:get disposed:0:false:true".to_owned()
        ))
    );
    assert!(eval("let get = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'disposed').get; get.call([]);").is_err());
    assert!(eval("DisposableStack.prototype.dispose.call({});").is_err());
}
