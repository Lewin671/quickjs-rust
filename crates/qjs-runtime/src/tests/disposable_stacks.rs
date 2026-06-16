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
    assert!(eval("DisposableStack.prototype.dispose.call(new AsyncDisposableStack());").is_err());
}

#[test]
fn disposable_stack_disposes_resources_in_reverse_order() {
    assert_eq!(
        eval(
            "let stack = new DisposableStack(); \
             let order = []; \
             let resource = { [Symbol.dispose]() { order.push('use'); } }; \
             stack.use(resource); \
             stack.adopt('value', value => order.push('adopt:' + value)); \
             stack.defer(() => order.push('defer')); \
             stack.dispose(); \
             order.join(',') + ':' + stack.disposed;"
        ),
        Ok(Value::String("defer,adopt:value,use:true".to_owned()))
    );
}

#[test]
fn disposable_stack_dispose_is_not_reentrant_after_disposed() {
    assert_eq!(
        eval(
            "let stack = new DisposableStack(); \
             let count = 0; \
             stack.defer(() => count++); \
             stack.dispose(); \
             stack.dispose(); \
             count;"
        ),
        Ok(Value::Number(1.0))
    );
    assert!(
        eval("let stack = new DisposableStack(); stack.dispose(); stack.defer(() => {});").is_err()
    );
}

#[test]
fn disposable_stack_dispose_error_completion_matches_sync_disposal() {
    assert_eq!(
        eval(
            "class MyError extends Error {} \
             let err = new MyError(); \
             let stack = new DisposableStack(); \
             stack.defer(() => { throw err; }); \
             try { stack.dispose(); 'no throw'; } catch (e) { e === err; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "class MyError extends Error {} \
             let error1 = new MyError(); \
             let error2 = new MyError(); \
             let error3 = new MyError(); \
             let stack = new DisposableStack(); \
             stack.defer(() => { throw error1; }); \
             stack.defer(() => { throw error2; }); \
             stack.defer(() => { throw error3; }); \
             try { stack.dispose(); 'no throw'; } \
             catch (e) { \
               (e instanceof SuppressedError) + ':' + \
               (e.error === error1) + ':' + \
               (e.suppressed instanceof SuppressedError) + ':' + \
               (e.suppressed.error === error2) + ':' + \
               (e.suppressed.suppressed === error3); \
             }"
        ),
        Ok(Value::String("true:true:true:true:true".to_owned()))
    );
}

#[test]
fn disposable_stack_move_transfers_resources_and_disposes_source() {
    assert_eq!(
        eval(
            "let source = new DisposableStack(); \
             let order = []; \
             source.defer(() => order.push('first')); \
             source.defer(() => order.push('second')); \
             let moved = source.move(); \
             let before = order.join(',') + ':' + source.disposed + ':' + moved.disposed; \
             moved.dispose(); \
             before + ':' + order.join(',');"
        ),
        Ok(Value::String(":true:false:second,first".to_owned()))
    );
    assert!(eval("let stack = new DisposableStack(); stack.dispose(); stack.move();").is_err());
    assert!(eval("DisposableStack.prototype.move.call({});").is_err());
}

#[test]
fn disposable_stack_move_surface_and_subclassing() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'move'); \
             typeof d.value + ':' + d.value.name + ':' + d.value.length + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("function:move:0:true:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "class MyDisposableStack extends DisposableStack {} \
             let source = new MyDisposableStack(); \
             let moved = source.move(); \
             (moved !== source) + ':' + \
             (moved instanceof DisposableStack) + ':' + \
             (moved instanceof MyDisposableStack);"
        ),
        Ok(Value::String("true:true:false".to_owned()))
    );
}
