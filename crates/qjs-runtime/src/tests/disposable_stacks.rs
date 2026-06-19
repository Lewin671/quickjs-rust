use crate::{Value, bytecode, eval};
use qjs_parser::parse_module;

fn eval_log(source: &str) -> String {
    match eval(source).expect("disposable stack evaluation should succeed") {
        Value::Array(array) => array
            .to_vec()
            .into_iter()
            .map(|value| match value {
                Value::String(text) => text.to_string(),
                Value::Number(number) => number.to_string(),
                Value::Boolean(flag) => flag.to_string(),
                Value::Undefined => "undefined".to_owned(),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => panic!("expected an array log, got {other:?}"),
    }
}

#[test]
fn disposable_stack_constructor_and_prototype_surface() {
    assert_eq!(
        eval("typeof DisposableStack;"),
        Ok(Value::String("function".to_owned().into()))
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
        Ok(Value::String("[object DisposableStack]".to_owned().into()))
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
        Ok(Value::String("false:false:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'constructor'); \
             (d.value === DisposableStack) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, Symbol.toStringTag); \
             d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "DisposableStack:false:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, Symbol.dispose); \
             (d.value === DisposableStack.prototype.dispose) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
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
        Ok(Value::String("false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DisposableStack.prototype, 'disposed'); typeof d.get + ':' + d.get.name + ':' + d.get.length + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "function:get disposed:0:false:true".to_owned().into()
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
        Ok(Value::String(
            "defer,adopt:value,use:true".to_owned().into()
        ))
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
        Ok(Value::String("true:true:true:true:true".to_owned().into()))
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
        Ok(Value::String(":true:false:second,first".to_owned().into()))
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
        Ok(Value::String(
            "function:move:0:true:false:true".to_owned().into()
        ))
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
        Ok(Value::String("true:true:false".to_owned().into()))
    );
}

#[test]
fn async_disposable_stack_move_transfers_resources_and_disposes_source() {
    assert_eq!(
        eval_log(
            "let source = new AsyncDisposableStack(); \
             let order = []; \
             let log = []; \
             source.defer(() => order.push('first')); \
             source.defer(() => order.push('second')); \
             let moved = source.move(); \
             let before = order.join(',') + ':' + source.disposed + ':' + moved.disposed; \
             moved.disposeAsync().then(() => { log.push(before); log.push(order.join(',')); }); \
             log;"
        ),
        ":true:false,second,first"
    );
    assert!(
        eval("let stack = new AsyncDisposableStack(); stack.disposeAsync(); stack.move();")
            .is_err()
    );
    assert!(eval("AsyncDisposableStack.prototype.move.call({});").is_err());
}

#[test]
fn async_disposable_stack_move_surface_and_subclassing() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(AsyncDisposableStack.prototype, 'move'); \
             typeof d.value + ':' + d.value.name + ':' + d.value.length + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "function:move:0:true:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "class MyAsyncDisposableStack extends AsyncDisposableStack {} \
             let source = new MyAsyncDisposableStack(); \
             let moved = source.move(); \
             (moved !== source) + ':' + \
             (moved instanceof AsyncDisposableStack) + ':' + \
             (moved instanceof MyAsyncDisposableStack);"
        ),
        Ok(Value::String("true:true:false".to_owned().into()))
    );
}

#[test]
fn async_disposable_stack_adopt_and_use_register_resources() {
    assert_eq!(
        eval(
            "let log = []; \
             let stack = new AsyncDisposableStack(); \
             let order = []; \
             let asyncResource = { [Symbol.asyncDispose]() { order.push('async-use'); } }; \
             let syncResource = { [Symbol.dispose]() { order.push('sync-use'); } }; \
             let adopted = {}; \
             let adoptResult = stack.adopt(adopted, value => order.push(value === adopted ? 'adopt' : 'wrong')); \
             let asyncResult = stack.use(asyncResource); \
             let syncResult = stack.use(syncResource); \
             let nullResult = stack.use(null); \
             (adoptResult === adopted) + ':' + \
               (asyncResult === asyncResource) + ':' + \
               (syncResult === syncResource) + ':' + \
               (nullResult === null);"
        ),
        Ok(Value::String("true:true:true:true".to_owned().into()))
    );
}

#[test]
fn async_disposable_stack_adopt_and_use_surface_and_errors() {
    assert_eq!(
        eval(
            "let adopt = Object.getOwnPropertyDescriptor(AsyncDisposableStack.prototype, 'adopt'); \
             let use = Object.getOwnPropertyDescriptor(AsyncDisposableStack.prototype, 'use'); \
             adopt.value.name + ':' + adopt.value.length + ':' + adopt.writable + ':' + adopt.enumerable + ':' + adopt.configurable + ';' + \
             use.value.name + ':' + use.value.length + ':' + use.writable + ':' + use.enumerable + ':' + use.configurable;"
        ),
        Ok(Value::String(
            "adopt:2:true:false:true;use:1:true:false:true"
                .to_owned()
                .into()
        ))
    );
    assert!(
        eval("let stack = new AsyncDisposableStack(); stack.disposeAsync(); stack.adopt(null, () => {});")
            .is_err()
    );
    assert!(
        eval("let stack = new AsyncDisposableStack(); stack.disposeAsync(); stack.use({ [Symbol.dispose]() {} });")
            .is_err()
    );
    assert!(eval("AsyncDisposableStack.prototype.adopt.call({}, null, () => {});").is_err());
    assert!(eval("AsyncDisposableStack.prototype.use.call({}, null);").is_err());
    assert!(eval("new AsyncDisposableStack().adopt(null, null);").is_err());
    assert!(eval("new AsyncDisposableStack().use({});").is_err());
}

#[test]
fn async_disposable_stack_dispose_async_rejects_in_returned_promise() {
    assert_eq!(
        eval_log(
            "let log = []; \
             AsyncDisposableStack.prototype.disposeAsync.call({}).then( \
               () => log.push('fulfilled'), \
               error => log.push(error instanceof TypeError)); \
             log;"
        ),
        "true"
    );
    assert_eq!(
        eval_log(
            "class MyError extends Error {} \
             let error = new MyError(); \
             let stack = new AsyncDisposableStack(); \
             stack.defer(async function () { throw error; }); \
             let log = []; \
             stack.disposeAsync().then( \
               () => log.push('fulfilled'), \
               reason => log.push(reason === error)); \
             log;"
        ),
        "true"
    );
}

#[test]
fn using_declaration_disposes_at_block_exit() {
    // Resources are disposed LIFO when the block completes normally.
    assert_eq!(
        eval(
            "let log = []; \
             { using a = { [Symbol.dispose]() { log.push('a'); } }; \
               using b = { [Symbol.dispose]() { log.push('b'); } }; \
               log.push('body'); } \
             log.join(',');"
        ),
        Ok(Value::String("body,b,a".to_owned().into()))
    );
}

#[test]
fn using_declaration_disposes_on_abrupt_completion() {
    // Disposal runs on throw, then the error propagates.
    assert_eq!(
        eval(
            "let log = []; \
             try { { using x = { [Symbol.dispose]() { log.push('d'); } }; \
                     throw new Error('boom'); } } \
             catch (e) { log.push('caught'); } \
             log.join(',');"
        ),
        Ok(Value::String("d,caught".to_owned().into()))
    );
    // And on return from a function.
    assert_eq!(
        eval(
            "let log = []; \
             function f() { { using x = { [Symbol.dispose]() { log.push('d'); } }; \
                             return 7; } } \
             f() + ':' + log.join(',');"
        ),
        Ok(Value::String("7:d".to_owned().into()))
    );
}

#[test]
fn using_declaration_rejects_non_disposable_initializers() {
    // null/undefined are allowed (no-op); other non-disposables throw a
    // TypeError when the declaration is evaluated.
    assert_eq!(
        eval("{ using x = null; using y = undefined; } 'ok';"),
        Ok(Value::String("ok".to_owned().into()))
    );
    assert!(eval("{ using x = {}; }").is_err());
    assert!(eval("{ using x = 5; }").is_err());
}

#[test]
fn using_declaration_has_empty_block_completion() {
    assert_eq!(eval("{ using x = null; }"), Ok(Value::Undefined));
    assert_eq!(eval("4; { using x = null; }"), Ok(Value::Number(4.0)));
    assert_eq!(eval("5; { 6; using x = null; }"), Ok(Value::Number(6.0)));
}

#[test]
fn await_using_block_registers_async_disposables() {
    assert_eq!(
        eval_log(
            "let log = []; \
             async function f() { \
               let asyncResource = { \
                 [Symbol.asyncDispose]() { log.push('async'); }, \
                 [Symbol.dispose]() { log.push('sync-unreached'); } \
               }; \
               let fallback = { \
                 [Symbol.asyncDispose]: null, \
                 [Symbol.dispose]() { log.push('fallback'); } \
               }; \
               { await using a = asyncResource; await using b = fallback; await using c = null; } \
               log.push('after'); \
             } \
             f(); log;"
        ),
        "fallback,async,after"
    );
}

#[test]
fn using_disposal_errors_chain_with_suppressed_error() {
    // A dispose failure that overrides a body throw is wrapped in a
    // SuppressedError carrying both errors.
    assert_eq!(
        eval(
            "try { { using x = { [Symbol.dispose]() { throw new Error('dispose'); } }; \
                     throw new Error('body'); } } \
             catch (e) { e.constructor.name + ':' + e.error.message + ':' + e.suppressed.message; }"
        ),
        Ok(Value::String(
            "SuppressedError:dispose:body".to_owned().into()
        ))
    );
}

#[test]
fn using_in_function_body_disposes_at_return() {
    // A `using` at the top level of a function body disposes when the function
    // returns, after any nested-block resources.
    assert_eq!(
        eval(
            "let log = []; \
             function f() { using a = { [Symbol.dispose]() { log.push('a'); } }; \
                            { using b = { [Symbol.dispose]() { log.push('b'); } }; } \
                            log.push('body'); return 9; } \
             f() + ':' + log.join(',');"
        ),
        Ok(Value::String("9:b,body,a".to_owned().into()))
    );
    // And when the body throws.
    assert_eq!(
        eval(
            "let log = []; \
             function f() { using x = { [Symbol.dispose]() { log.push('d'); } }; \
                            throw new Error('boom'); } \
             try { f(); } catch (e) { log.push('caught'); } log.join(',');"
        ),
        Ok(Value::String("d,caught".to_owned().into()))
    );
}

#[test]
fn using_for_initializer_disposes_when_loop_exits() {
    assert_eq!(
        eval(
            "let log = []; \
             let i = 0; \
             for (using x = { [Symbol.dispose]() { log.push('dispose'); } }; i < 2; i++) { \
               log.push('body:' + i); \
             } \
             log.join(',');"
        ),
        Ok(Value::String("body:0,body:1,dispose".to_owned().into()))
    );
}

#[test]
fn using_for_initializer_disposes_if_later_initializer_throws() {
    assert_eq!(
        eval(
            "let log = []; \
             function fail() { throw new Error('boom'); } \
             try { \
               for (using x = { [Symbol.dispose]() { log.push('dispose'); } }, y = fail(); false;) {} \
             } catch (e) { log.push('caught'); } \
             log.join(',');"
        ),
        Ok(Value::String("dispose,caught".to_owned().into()))
    );
}

#[test]
fn using_generator_body_preserves_disposal_scope_across_yield() {
    assert_eq!(
        eval(
            "let log = []; \
             function *f() { \
               using x = { [Symbol.dispose]() { log.push('dispose'); } }; \
               yield 'pause'; \
             } \
             let g = f(); \
             let first = g.next(); \
             let before = log.join(','); \
             let second = g.next(); \
             first.value + ':' + first.done + ':' + before + ':' + second.done + ':' + log.join(',');"
        ),
        Ok(Value::String("pause:false::true:dispose".to_owned().into()))
    );
}

#[test]
fn using_module_statement_list_disposes_top_level_resources() {
    let script = parse_module(
        "class MyError extends Error {} \
         const error1 = new MyError(); \
         const error2 = new MyError(); \
         const error3 = new MyError(); \
         let result; \
         try { \
           using _1 = { [Symbol.dispose]() { throw error1; } }; \
           using _2 = { [Symbol.dispose]() { throw error2; } }; \
           throw error3; \
         } catch (e) { \
           result = (e instanceof SuppressedError) + ':' + \
             (e.error === error1) + ':' + \
             (e.suppressed instanceof SuppressedError) + ':' + \
             (e.suppressed.error === error2) + ':' + \
             (e.suppressed.suppressed === error3); \
         } \
         result;",
    )
    .expect("module source should parse");
    let bytecode = bytecode::compile_module(&script).expect("module should compile");

    assert_eq!(
        bytecode::eval_bytecode(&bytecode),
        Ok(Value::String("true:true:true:true:true".to_owned().into()))
    );
}
