use crate::{Value, eval};

#[test]
fn default_parameter_initializers_use_parameter_tdz() {
    let self_ref = eval(
        "let calls = 0; function f(x = x) { calls = calls + 1; } \
         let name; try { f(); } catch (error) { name = error.name; } \
         name + ':' + calls;",
    );
    assert_eq!(
        self_ref,
        Ok(Value::String("ReferenceError:0".to_owned().into()))
    );

    let later_ref = eval(
        "let calls = 0; function f(x = y, y) { calls = calls + 1; } \
         let name; try { f(); } catch (error) { name = error.name; } \
         name + ':' + calls;",
    );
    assert_eq!(
        later_ref,
        Ok(Value::String("ReferenceError:0".to_owned().into()))
    );

    assert_eq!(
        eval("function f(x = 1, y = x + 1) { return x + y; } f();"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function f(x = y, y) { return x + y; } f(1, 2);"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn default_parameter_closures_do_not_capture_body_var_environment() {
    assert_eq!(
        eval(
            "var x = 'outside'; var probeParams, probeBody; \
             function f(_ = probeParams = function() { return x; }) { \
               var x = 'inside'; \
               probeBody = function() { return x; }; \
             } \
             f(); \
             probeParams() + ':' + probeBody();"
        ),
        Ok(Value::String("outside:inside".to_owned().into()))
    );

    assert_eq!(
        eval(
            "let name; \
             function f(read = () => x) { \
               var x = 'inside'; \
               try { read(); } catch (error) { name = error.name; } \
             } \
             f(); \
             name;"
        ),
        Ok(Value::String("ReferenceError".to_owned().into()))
    );

    assert_eq!(
        eval("function f(read = () => typeof x) { var x = 'inside'; return read(); } f();"),
        Ok(Value::String("undefined".to_owned().into()))
    );
}

#[test]
fn default_parameter_eval_closures_capture_parameter_eval_bindings() {
    assert_eq!(
        eval(
            "const f = (p = eval(\"var arguments = 'param'\"), q = () => arguments) => { \
               var arguments = 'local'; \
               return arguments + ':' + q(); \
             }; \
             f();"
        ),
        Ok(Value::String("local:param".to_owned().into()))
    );

    assert_eq!(
        eval(
            "const f = (p = eval(\"var arguments = 'param'\"), q = () => arguments) => { \
               let arguments = 'local'; \
               return arguments + ':' + q(); \
             }; \
             f();"
        ),
        Ok(Value::String("local:param".to_owned().into()))
    );

    assert_eq!(
        eval(
            "const f = (p = eval(\"var arguments = 'param'\"), q = () => arguments) => { \
               function arguments() {} \
               return typeof arguments + ':' + q(); \
             }; \
             f();"
        ),
        Ok(Value::String("function:param".to_owned().into()))
    );

    assert_eq!(
        eval(
            "function f(read = (eval(\"var x = 'eval'\"), () => x)) { \
               var x = 'body'; \
               return read; \
             } \
             f()();"
        ),
        Ok(Value::String("eval".to_owned().into()))
    );
}

#[test]
fn unmapped_arguments_callee_is_restricted() {
    assert_eq!(
        eval(
            "function strictArgs() { 'use strict'; try { arguments.callee; } catch (error) { return error instanceof TypeError; } return false; } \
             function nonSimple(x = 1) { try { arguments.callee; } catch (error) { return error instanceof TypeError; } return false; } \
             function sloppySimple() { return arguments.callee === sloppySimple; } \
             [strictArgs(), nonSimple(), sloppySimple()].join(':');"
        ),
        Ok(Value::String("true:true:true".to_owned().into()))
    );
}

#[test]
fn sloppy_simple_arguments_callee_is_a_data_property() {
    // A sloppy-mode simple-parameter function's `arguments.callee` is a data
    // property holding the executing function.
    assert_eq!(
        eval(
            "function f(a) { \
                 var d = Object.getOwnPropertyDescriptor(arguments, 'callee'); \
                 return [d.value === f, d.writable, d.enumerable, d.configurable].join(':'); \
             } f(1);"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
}

#[test]
fn arguments_symbol_iterator_is_array_prototype_values() {
    // An arguments object's `[Symbol.iterator]` is the same function object as
    // `Array.prototype.values` / `Array.prototype[Symbol.iterator]`.
    assert_eq!(
        eval(
            "(function () { \
                 return arguments[Symbol.iterator] === [][Symbol.iterator] \
                     && arguments[Symbol.iterator] === Array.prototype.values; \
             })();"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn throw_type_error_poison_is_a_single_shared_intrinsic() {
    // %ThrowTypeError% is one object: the strict `arguments.callee` poison
    // getter is the same function as `Function.prototype.arguments`/`caller`'s.
    assert_eq!(
        eval(
            "var callee = (function () { 'use strict'; \
                 return Object.getOwnPropertyDescriptor(arguments, 'callee').get; })(); \
             var args = Object.getOwnPropertyDescriptor(Function.prototype, 'arguments').get; \
             var caller = Object.getOwnPropertyDescriptor(Function.prototype, 'caller').get; \
             [callee === args, args === caller].join(':');"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn evaluates_destructured_parameters() {
    assert_eq!(
        eval(
            "function pick({x, y: {z} = {z: 9}}, [p = 5]) { return x + z + p; } pick({x: 1}, []);"
        ),
        Ok(Value::Number(15.0))
    );
    assert_eq!(
        eval("let sum = ([a, b], {c}) => a + b + c; sum([1, 2], {c: 3});"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let method = { add([a, b]) { return a + b; } }; method.add([4, 5]);"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn evaluates_rest_parameter_patterns() {
    assert_eq!(
        eval("function tail(a, ...[b, c]) { return a + b + c; } tail(1, 2, 3, 4);"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("function size(...{length}) { return length; } size(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_binding_pattern_rest_elements() {
    assert_eq!(
        eval(
            "function f([first, ...others]) { return first + ':' + others.join('|'); } f([1, 2, 3]);"
        ),
        Ok(Value::String("1:2|3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function f({a, ...rest}) { return a + ':' + Object.keys(rest).join('|') + ':' + rest.b; } f({a: 1, b: 2, c: 3});"
        ),
        Ok(Value::String("1:b|c:2".to_owned().into()))
    );
}

#[test]
fn parameter_defaults_apply_only_to_undefined() {
    assert_eq!(
        eval("function f(x = 5) { return x; } f(null) === null;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function f(x = 5, y = x + 1) { return x + y; } f();"),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "var log = []; function t(v) { log.push(v); return v; } function f(a = t(1), {b} = {b: t(2)}, c = t(3)) {} f(); log.join(',');"
        ),
        Ok(Value::String("1,2,3".to_owned().into()))
    );
}

#[test]
fn destructured_parameters_iterate_iterables() {
    assert_eq!(
        eval("function f([a, b]) { return a + b; } f('xy');"),
        Ok(Value::String("xy".to_owned().into()))
    );
    assert_eq!(
        eval("function f([[k, v]]) { return k + '=' + v; } f(new Map([['a', 1]]));"),
        Ok(Value::String("a=1".to_owned().into()))
    );
    // A hand-rolled iterable stands in for a generator until generator
    // evaluation lands in T010 S2.
    assert_eq!(
        eval(
            "function range() {
               var n = 0;
               return { [Symbol.iterator]() { return this; },
                        next() { n = n + 1; return { value: n, done: n > 3 }; } };
             }
             function f([head, ...tail]) { return head + ':' + tail.join('|'); } f(range());"
        ),
        Ok(Value::String("1:2|3".to_owned().into()))
    );
}

#[test]
fn destructured_parameter_coercion_errors_are_type_errors() {
    assert_eq!(
        eval("try { (function({x}) {})(undefined); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("try { (function({}) {})(null); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("try { (function([a]) {})(5); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn non_simple_parameter_lists_unmap_arguments() {
    assert_eq!(
        eval("function f(a) { a = 99; return arguments[0]; } f(1);"),
        Ok(Value::Number(99.0))
    );
    assert_eq!(
        eval("function f(a, b = 2) { a = 99; return arguments[0]; } f(1);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function f([a], {b}) { return arguments.length + ':' + arguments[0][0] + ':' + arguments[1].b; } f([7], {b: 8});"
        ),
        Ok(Value::String("2:7:8".to_owned().into()))
    );
}

#[test]
fn destructuring_temporaries_stay_frame_local() {
    assert_eq!(
        eval(
            "function g({p} = {p: 0}, {q} = {q: 0}) { return p + q; } function f([a] = [g({p: 1}, {q: 2})], [b]) { return a + ':' + b; } f(undefined, [33]);"
        ),
        Ok(Value::String("3:33".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function g([p, q]) { return p + q; } function f([a = g([1, 2]), b]) { return a + ':' + b; } f([undefined, 7]);"
        ),
        Ok(Value::String("3:7".to_owned().into()))
    );
}

#[test]
fn destructured_parameter_function_length_skips_defaults() {
    assert_eq!(
        eval("(function({a}, [b], c = 1) {}).length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("(function(...rest) {}).length;"),
        Ok(Value::Number(0.0))
    );
}
