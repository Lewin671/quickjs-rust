use crate::{Value, eval};

#[test]
fn evaluates_error_builtins() {
    assert_eq!(
        eval("typeof Error;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("Error.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Error.isError.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Error.isError.name;"),
        Ok(Value::String("isError".to_owned().into()))
    );
    assert_eq!(
        eval("Error.prototype.constructor === Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Error.prototype.name;"),
        Ok(Value::String("Error".to_owned().into()))
    );
    assert_eq!(
        eval("Error.prototype.message;"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("Error.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.message;"),
        Ok(Value::String("boom".to_owned().into()))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.constructor === Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Error('boom').toString();"),
        Ok(Value::String("Error: boom".to_owned().into()))
    );
    assert_eq!(
        eval("Error('boom').toString();"),
        Ok(Value::String("Error: boom".to_owned().into()))
    );
    assert!(
        eval("Error(Symbol('boom'));")
            .expect_err("Symbol message conversion should throw")
            .message
            .contains("TypeError")
    );
    assert_eq!(
        eval("new Error().toString();"),
        Ok(Value::String("Error".to_owned().into()))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.name = 'Custom'; error.toString();"),
        Ok(Value::String("Custom: boom".to_owned().into()))
    );
    assert_eq!(
        eval(
            "[
                undefined,
                null,
                1,
                true,
                'string',
                Symbol()
             ].map(function (value) {
                try {
                    Error.prototype.toString.call(value);
                    return 'ok';
                } catch (error) {
                    return error instanceof TypeError;
                }
             }).join('|');"
        ),
        Ok(Value::String(
            "true|true|true|true|true|true".to_owned().into()
        ))
    );
    assert!(
        eval("Error.prototype.toString.call({ get name() { throw new Error('name'); } });")
            .expect_err("name getter abrupt completion should be propagated")
            .message
            .contains("Error: name")
    );
    assert!(
        eval("Error.prototype.toString.call({ get message() { throw new Error('message'); } });")
            .expect_err("message getter abrupt completion should be propagated")
            .message
            .contains("Error: message")
    );
    assert_eq!(
        eval("Error.prototype.toString.call(Object(Symbol('boxed')));"),
        Ok(Value::String("Error".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let fn = function named() {}; fn.message = 'callable'; Error.prototype.toString.call(fn);"
        ),
        Ok(Value::String("named: callable".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Error('boom'));"),
        Ok(Value::String("[object Error]".to_owned().into()))
    );
    assert_eq!(
        eval("Error.isError(new Error('boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Error.isError(Error('boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Error.isError({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError(Error);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError('boom');"), Ok(Value::Boolean(false)));
    assert!(
        eval("throw new Error('boom');")
            .expect_err("throwing an Error should fail evaluation")
            .message
            .contains("Error: boom")
    );
}

#[test]
fn evaluates_native_error_builtins() {
    for name in [
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "TypeError",
        "URIError",
    ] {
        assert_eq!(
            eval(&format!("typeof {name};")),
            Ok(Value::String("function".to_owned().into()))
        );
        assert_eq!(eval(&format!("{name}.length;")), Ok(Value::Number(1.0)));
        assert_eq!(
            eval(&format!("{name}.name;")),
            Ok(Value::String(name.to_owned().into()))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.name;")),
            Ok(Value::String(name.to_owned().into()))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.message;")),
            Ok(Value::String(::std::rc::Rc::new(String::new())))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.constructor === {name};")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("Object.getPrototypeOf({name}) === Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("Reflect.getPrototypeOf({name}) === Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("{name}.isError === Error.isError;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("let error = new {name}('boom'); error.message;")),
            Ok(Value::String("boom".to_owned().into()))
        );
        assert_eq!(
            eval(&format!("new {name}('boom') instanceof {name};")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("new {name}('boom') instanceof Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("{name}('boom').toString();")),
            Ok(Value::String(format!("{name}: boom").into()))
        );
        assert_eq!(
            eval(&format!(
                "Object.prototype.toString.call(new {name}('boom'));"
            )),
            Ok(Value::String("[object Error]".to_owned().into()))
        );
        assert_eq!(
            eval(&format!("Error.isError(new {name}('boom'));")),
            Ok(Value::Boolean(true))
        );
    }
}

#[test]
fn evaluates_aggregate_error_builtin() {
    assert_eq!(
        eval("typeof AggregateError;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("AggregateError.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("AggregateError.prototype.name;"),
        Ok(Value::String("AggregateError".to_owned().into()))
    );
    assert_eq!(
        eval("AggregateError.prototype.constructor === AggregateError;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(AggregateError) === Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function scanGlobal() { for (var key in this) {} } \
             scanGlobal(); \
             Object.getOwnPropertyDescriptor(this, 'AggregateError').enumerable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let error = new AggregateError([1, 2], 'boom'); error.message;"),
        Ok(Value::String("boom".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let error = new AggregateError([1, 2], 'boom'); error.errors.length + ':' + error.errors[0] + ':' + error.errors[1];"
        ),
        Ok(Value::String("2:1:2".to_owned().into()))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom') instanceof AggregateError;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom') instanceof Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom').toString();"),
        Ok(Value::String("AggregateError: boom".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new AggregateError([], 'boom'));"),
        Ok(Value::String("[object Error]".to_owned().into()))
    );
    assert_eq!(
        eval("Error.isError(new AggregateError([], 'boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let cause = { message: 'root' }; let error = new AggregateError([], 'boom', { cause: cause }); let desc = Object.getOwnPropertyDescriptor(error, 'cause'); (desc.value === cause) + ':' + desc.writable + ':' + desc.enumerable + ':' + desc.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("Object.hasOwn(new AggregateError([], 'boom', { cause: undefined }), 'cause');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn(new AggregateError([], 'boom'), 'cause');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let marker = {}; let message = { toString: function() { throw marker; } }; let caught = false; try { new AggregateError([], message); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let source = {}; source[Symbol.iterator] = function() { let index = 0; return { next: function() { index = index + 1; return index > 2 ? { done: true } : { value: index, done: false }; } }; }; let error = new AggregateError(source, 'boom'); error.errors.join();"
        ),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert!(eval("new AggregateError();").is_err());
    assert!(eval("new AggregateError(null);").is_err());
    assert!(eval("new AggregateError(1);").is_err());
}

#[test]
fn aggregate_error_uses_new_target_prototype() {
    assert_eq!(
        eval(
            "let custom = { x: 42 }; \
             let newTarget = new Proxy(function() {}, { \
               get: function(target, key) { \
                 if (key === 'prototype') { return custom; } \
                 return target[key]; \
               } \
             }); \
             let error = Reflect.construct(AggregateError, [[]], newTarget); \
             Object.getPrototypeOf(error) === custom && error.x;"
        ),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "let values = [undefined, null, 42, false, true, Symbol(), 'string']; \
             let NewTarget = new Function(); \
             values.every(function(value) { \
               let NewTargetProxy = new Proxy(NewTarget, { \
                 get: function(target, key) { \
                   if (key === 'prototype') { return value; } \
                   return target[key]; \
                 } \
               }); \
               let error = Reflect.construct(AggregateError, [[]], NewTargetProxy); \
               return Object.getPrototypeOf(error) === AggregateError.prototype; \
             });"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn aggregate_error_evaluates_message_before_errors() {
    assert_eq!(
        eval(
            "let sequence = []; \
             let message = { toString: function() { sequence.push(1); return ''; } }; \
             let errors = {}; \
             errors[Symbol.iterator] = function() { \
               sequence.push(2); \
               return { next: function() { sequence.push(3); return { done: true }; } }; \
             }; \
             new AggregateError(errors, message); \
             sequence.join(',');"
        ),
        Ok(Value::String("1,2,3".to_owned().into()))
    );
}

#[test]
fn error_cause_property() {
    // Basic cause on Error
    assert_eq!(
        eval("var e = new Error('msg', { cause: 'the cause' }); e.cause;"),
        Ok(Value::String("the cause".to_owned().into()))
    );
    // cause property descriptor: non-enumerable, writable, configurable
    assert_eq!(
        eval(
            "var e = new Error('msg', { cause: 42 }); \
             var desc = Object.getOwnPropertyDescriptor(e, 'cause'); \
             desc.value + ':' + desc.writable + ':' + desc.enumerable + ':' + desc.configurable;"
        ),
        Ok(Value::String("42:true:false:true".to_owned().into()))
    );
    // cause is undefined value but present when options has cause: undefined
    assert_eq!(
        eval("Object.hasOwn(new Error('msg', { cause: undefined }), 'cause');"),
        Ok(Value::Boolean(true))
    );
    // no cause when no options
    assert_eq!(
        eval("Object.hasOwn(new Error('msg'), 'cause');"),
        Ok(Value::Boolean(false))
    );
    // no cause when options is not an object
    assert_eq!(
        eval("Object.hasOwn(new Error('msg', 42), 'cause');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.hasOwn(new Error('msg', Symbol()), 'cause');"),
        Ok(Value::Boolean(false))
    );
    // no cause when options object has no cause property
    assert_eq!(
        eval("Object.hasOwn(new Error('msg', {}), 'cause');"),
        Ok(Value::Boolean(false))
    );
    // cause works on all native error types
    for name in [
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "TypeError",
        "URIError",
    ] {
        assert_eq!(
            eval(&format!("new {name}('msg', {{ cause: 'root' }}).cause;")),
            Ok(Value::String("root".to_owned().into())),
            "{name} should support cause"
        );
        assert_eq!(
            eval(&format!(
                "Object.hasOwn(new {name}('msg', {{ cause: undefined }}), 'cause');"
            )),
            Ok(Value::Boolean(true)),
            "{name} should have cause when cause is undefined"
        );
        assert_eq!(
            eval(&format!("Object.hasOwn(new {name}('msg'), 'cause');")),
            Ok(Value::Boolean(false)),
            "{name} should not have cause when no options"
        );
    }
    // cause getter that throws should propagate the error
    assert_eq!(
        eval(
            "var marker = {}; \
             var caught = false; \
             try { \
               new Error('msg', { get cause() { throw marker; } }); \
             } catch (e) { \
               caught = e === marker; \
             } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var marker = {}; \
             var options = new Proxy({}, { \
               has: function(target, key) { \
                 if (key === 'cause') throw marker; \
                 return key in target; \
               } \
             }); \
             var caught = false; \
             try { \
               new Error('msg', options); \
             } catch (e) { \
               caught = e === marker; \
             } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Error() called as function also supports cause
    assert_eq!(
        eval("Error('msg', { cause: 'fn cause' }).cause;"),
        Ok(Value::String("fn cause".to_owned().into()))
    );
}

#[test]
fn method_call_on_null_or_undefined_is_catchable_type_error() {
    assert_eq!(
        eval("try { null.foo(); } catch (e) { e instanceof TypeError; }").expect("eval"),
        Value::Boolean(true)
    );
    assert_eq!(
        eval("var u; try { u.bar(1, 2); } catch (e) { e instanceof TypeError; }").expect("eval"),
        Value::Boolean(true)
    );
    assert_eq!(
        eval("try { undefined.baz; } catch (e) { e.message; }").expect("eval"),
        Value::String(
            "Cannot read properties of undefined (reading 'baz')"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn spread_iterator_errors_are_catchable_at_top_level() {
    // A throw raised while evaluating a spread must unwind through an enclosing
    // try/catch in the global script frame, not escape the VM loop. Covers array
    // literal, call, new, and object spread.
    let throwing_iterable = "var bad = { [Symbol.iterator]() { return { next() { throw new TypeError('boom'); } }; } };";
    assert_eq!(
        eval(&format!(
            "{throwing_iterable} var c = ''; try {{ [...bad]; }} catch (e) {{ c = e.name; }} c;"
        )),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{throwing_iterable} var c = ''; try {{ Math.max(...bad); }} catch (e) {{ c = e.name; }} c;"
        )),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{throwing_iterable} var c = ''; try {{ new (class {{}})(...bad); }} catch (e) {{ c = e.name; }} c;"
        )),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    // Object spread invokes getters; a throwing getter is catchable too.
    assert_eq!(
        eval(
            "var c = ''; try { var o = { ...{ get x() { throw new TypeError('g'); } } }; } catch (e) { c = e.name; } c;"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
}
