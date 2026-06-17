use crate::{Value, eval};

#[test]
fn evaluates_boolean_builtins() {
    assert_eq!(
        eval("typeof Boolean;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Boolean.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Boolean();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Boolean('');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean('x');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Boolean.prototype.constructor === Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Boolean.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.toString();"),
        Ok(Value::String("false".to_owned()))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("true.toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(eval("false.valueOf();"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("(new Boolean(true)).toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(
        eval("(new Boolean(0)).valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let o = Object.create(Boolean.prototype); o.valueOf();").is_err());
}

#[test]
fn evaluates_global_undefined_binding() {
    assert_eq!(eval("undefined;"), Ok(Value::Undefined));
    assert_eq!(eval("undefined === undefined;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'undefined'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned()))
    );
}

#[test]
fn global_nan_is_non_writable() {
    // Sloppy mode: assignment silently fails, NaN remains a number.
    assert_eq!(
        eval("NaN = true; typeof NaN;"),
        Ok(Value::String("number".to_owned()))
    );
    assert_eq!(eval("NaN = true; NaN !== NaN;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("var NaN = 1.0; NaN = 'asdf'; NaN = true; NaN !== NaN;"),
        Ok(Value::Boolean(true))
    );
    // Descriptor check.
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'NaN'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned()))
    );
    // Strict mode: TypeError on assignment to non-writable NaN.
    assert!(eval("'use strict'; NaN = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; NaN = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn global_infinity_is_non_writable() {
    // Sloppy mode: assignment silently fails, Infinity remains a number.
    assert_eq!(
        eval("Infinity = true; typeof Infinity;"),
        Ok(Value::String("number".to_owned()))
    );
    assert_eq!(
        eval("Infinity = true; Infinity === 1/0;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var Infinity = 1.0; Infinity = 'asdf'; Infinity = true; Infinity === 1/0;"),
        Ok(Value::Boolean(true))
    );
    // Descriptor check.
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'Infinity'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:false".to_owned()))
    );
    // Strict mode: TypeError on assignment to non-writable Infinity.
    assert!(eval("'use strict'; Infinity = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; Infinity = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn global_undefined_is_non_writable() {
    // Sloppy mode: assignment silently fails, undefined stays undefined.
    assert_eq!(
        eval("undefined = true; typeof undefined;"),
        Ok(Value::String("undefined".to_owned()))
    );
    // Strict mode: TypeError on assignment to non-writable undefined.
    assert!(eval("'use strict'; undefined = 1;").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (function() { 'use strict'; undefined = 1; })(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_global_this_binding() {
    assert_eq!(eval("globalThis === this;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("globalThis.Object === Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("this.globalThis === globalThis;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').writable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'globalThis').configurable;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn exposes_print_host_global() {
    assert_eq!(
        eval("typeof print;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("print.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("this.print === print;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(this, 'print').enumerable;"),
        Ok(Value::Boolean(false))
    );
    // Returns undefined regardless of argument count, and is reachable from a
    // nested call frame (the $DONE async channel prints from a reaction).
    assert_eq!(eval("print('quiet', 1, 2);"), Ok(Value::Undefined));
    assert_eq!(
        eval("(function () { return print('nested'); })();"),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_test262_same_value_host_helper() {
    assert_eq!(
        eval("__quickjsRustAssertSameValue(NaN, NaN);"),
        Ok(Value::Undefined)
    );
    assert!(eval("__quickjsRustAssertSameValue(+0, -0, 'zero');").is_err());
}

#[test]
fn evaluates_global_eval_builtin() {
    assert_eq!(
        eval("typeof eval;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("eval.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("this.eval === eval;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'eval'); (d.value === eval) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'Object'); (d.value === Object) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(eval("eval(7);"), Ok(Value::Number(7.0)));
    assert_eq!(eval("eval('1 + 2;');"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let value = 1; eval('value = value + 2;'); value;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('var leaked = 1;'); leaked;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("eval('{ let lexical = 1; }'); typeof lexical;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert!(eval("eval('{ let lexical = 1; } lexical;')").is_err());
    assert!(eval("eval('{ let f = 123; { function f() {} } } f;')").is_err());
    assert!(
        eval("eval('{ let f = 123; if (true) function f() {} else function _f() {} } f;')")
            .is_err()
    );
    assert!(eval("eval('for (let i = 0; i < 1; i++) {} i;')").is_err());
    assert!(eval("eval('for (let f; ; ) { { function f() {} } break; } f;')").is_err());
    assert!(eval("eval('for (let k in { a: 1 }) {} k;')").is_err());
    assert!(eval("eval('for (let v of [1]) {} v;')").is_err());
    assert!(eval("eval('switch (1) { case 1: let s = 1; } s;')").is_err());
    assert_eq!(
        eval(
            "eval('var before = f; { function f() { return 7; } } before === undefined && f() === 7;');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "{ function evalLexCollisionFn() {} } \
             let caught = false; \
             try { eval('var evalLexCollisionVar; let evalLexCollisionFn;'); } catch (error) { caught = error instanceof SyntaxError; } \
             caught + ':' + (typeof evalLexCollisionVar);"
        ),
        Ok(Value::String("true:undefined".to_owned()))
    );
}

#[test]
fn evaluates_global_eval_pure_regexp_literals() {
    assert_eq!(
        eval(
            "let RegExp = function() { throw new Error('shadowed'); }; eval('/\\\\u0041/i').source + ':' + eval('/a/i').ignoreCase;"
        ),
        Ok(Value::String("\\u0041:true".to_owned()))
    );
    assert_eq!(
        eval("eval('/[\\\\/]/').test('/');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("eval('  /a/g;  ').global;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("eval('/a/; 1');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("eval('// comment');"), Ok(Value::Undefined));
}

#[test]
fn evaluates_direct_eval_annex_b_function_bindings_in_function_frames() {
    assert_eq!(
        eval(
            "var init, changed; \
             (function() { eval('init = f; f = 123; changed = f; { function f() {} }'); }()); \
             String(init) + ':' + changed + ':' + typeof f;"
        ),
        Ok(Value::String("undefined:123:undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "var init, after; \
             (function(f) { eval('init = f; { function f() {} } after = typeof f;'); }(123)); \
             init + ':' + after + ':' + typeof f;"
        ),
        Ok(Value::String("123:function:undefined".to_owned()))
    );
}

#[test]
fn eval_annex_b_function_declarations_capture_block_scoped_binding() {
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('{ function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } }'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned()))
    );
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('if (true) function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } else function _f() {}'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned()))
    );
    assert_eq!(
        eval(
            "var initialBV, currentBV; \
             eval('switch (1) { case 1: function f() { initialBV = f; f = 123; currentBV = f; return \"decl\"; } }'); \
             f(); \
             initialBV() + ':' + currentBV + ':' + f();"
        ),
        Ok(Value::String("decl:123:decl".to_owned()))
    );
}

#[test]
fn evaluates_global_eval_annex_b_bindings_as_configurable() {
    assert_eq!(
        eval(
            "eval('if (true) { function test262Fn() {} }'); \
             let d = Object.getOwnPropertyDescriptor(this, 'test262Fn'); \
             typeof test262Fn + ':' + d.configurable;"
        ),
        Ok(Value::String("function:true".to_owned()))
    );
}

#[test]
fn evaluates_indirect_eval_against_global_scope() {
    assert_eq!(
        eval(
            "let local = 1; \
             (function() { let local = 2; return (0, eval)('typeof local'); }());"
        ),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "(function(source) { return (0, eval)(source); }('let indirectLexical = 1;')); \
             indirectLexical + ':' + Object.prototype.hasOwnProperty.call(this, 'indirectLexical');"
        ),
        Ok(Value::String("1:false".to_owned()))
    );
}

#[test]
fn initializes_global_hoisted_bindings_before_script_execution() {
    assert_eq!(
        eval("var before = f; { function f() { return 9; } } before === undefined && f() === 9;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "{ function f() {} } var d = Object.getOwnPropertyDescriptor(this, 'f'); d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "eval(\"Object.defineProperty(this, 'annexGlobalFn', { value: 'x', enumerable: false, writable: true, configurable: true });\"); \
             eval(\"{ function annexGlobalFn() { return 9; } }\"); \
             let d = Object.getOwnPropertyDescriptor(this, 'annexGlobalFn'); \
             annexGlobalFn() + ':' + d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("9:false:true:true".to_owned()))
    );
}

#[test]
fn skips_annex_b_function_binding_for_parameter_collisions() {
    assert_eq!(
        eval(
            "var init, after; (function(f) { init = f; if (false) function _f() {} else function f() {} after = f; }(123)); init + ':' + after;"
        ),
        Ok(Value::String("123:123".to_owned()))
    );
    assert_eq!(
        eval(
            "var init, after; (function(f = 123) { init = f; if (false) function _f() {} else function f() {} after = f; }()); init + ':' + after;"
        ),
        Ok(Value::String("123:123".to_owned()))
    );
}

#[test]
fn evaluates_uri_coding_builtins() {
    assert_eq!(eval("encodeURI.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("decodeURI.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("encodeURIComponent.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("decodeURIComponent.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(this, 'decodeURIComponent'); (d.value === decodeURIComponent) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval("encodeURI('https://example.test/a b?x=1&y=\\u00E9#frag');"),
        Ok(Value::String(
            "https://example.test/a%20b?x=1&y=%C3%A9#frag".to_owned()
        ))
    );
    assert_eq!(
        eval("encodeURIComponent('a b?x=1&y=\\u00E9');"),
        Ok(Value::String("a%20b%3Fx%3D1%26y%3D%C3%A9".to_owned()))
    );
    assert_eq!(
        eval("decodeURI('https://example.test/a%20b?x=1&y=%C3%A9%23frag');"),
        Ok(Value::String(
            "https://example.test/a b?x=1&y=\u{00E9}%23frag".to_owned()
        ))
    );
    assert_eq!(
        eval("decodeURIComponent('a%20b%3Fx%3D1%26y%3D%C3%A9');"),
        Ok(Value::String("a b?x=1&y=\u{00E9}".to_owned()))
    );
    assert_eq!(
        eval("encodeURIComponent(String.fromCodePoint(0x1D306));"),
        Ok(Value::String("%F0%9D%8C%86".to_owned()))
    );
    assert_eq!(
        eval("encodeURIComponent(decodeURIComponent('%F0%9D%8C%86'));"),
        Ok(Value::String("%F0%9D%8C%86".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { decodeURIComponent('%E0%A4%A'); } catch (error) { caught = error instanceof URIError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_annex_b_escape_builtins() {
    assert_eq!(eval("escape.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("unescape.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("escape('');"), Ok(Value::String(String::new())));
    assert_eq!(
        eval("escape('AZaz09@*_+-./');"),
        Ok(Value::String("AZaz09@*_+-./".to_owned()))
    );
    assert_eq!(
        eval("escape(' #éĀ');"),
        Ok(Value::String("%20%23%E9%u0100".to_owned()))
    );
    assert_eq!(
        eval("escape(String.fromCodePoint(0x1D306));"),
        Ok(Value::String("%uD834%uDF06".to_owned()))
    );
    assert_eq!(
        eval("unescape('%20%23%E9%u0100');"),
        Ok(Value::String(" #éĀ".to_owned()))
    );
    assert_eq!(
        eval(
            "let value = unescape('%uD834%uDF06'); value.length === 2 && value.charCodeAt(0) === 0xD834 && value.charCodeAt(1) === 0xDF06;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("unescape('%zz%u12xz');"),
        Ok(Value::String("%zz%u12xz".to_owned()))
    );
}

#[test]
fn keeps_global_object_properties_and_bindings_in_sync() {
    assert_eq!(
        eval("let global = Function('return this;')(); global === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let global = Function('return this;')(); global.customGlobal = 7; customGlobal;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let global = Function('return this;')(); global.Object = function FakeObject() {}; Object === global.Object;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "toString = Object.prototype.toString; typeof toString + ':' + typeof this.toString + ':' + this.toString();"
        ),
        Ok(Value::String(
            "function:function:[object Object]".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, 'prop', { value: 1001, writable: false, configurable: false }); var prop = 1002; this.hasOwnProperty('prop') + ':' + prop + ':' + this.prop;"
        ),
        Ok(Value::String("true:1002:1002".to_owned()))
    );
    assert_eq!(
        eval("function f() { var localOnly = 1; return this.hasOwnProperty('localOnly'); } f();"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn direct_function_eval_rejects_var_arguments() {
    // A non-arrow function always binds `arguments`, so a direct eval hoisting
    // a `var`/`function` declaration named `arguments` is a SyntaxError.
    assert!(
        eval("function f() { eval('var arguments = 1'); } f();").is_err(),
        "non-arrow eval declaring var arguments must throw"
    );
    assert!(
        eval("function f() { eval('function arguments() {}'); } f();").is_err(),
        "non-arrow eval declaring function arguments must throw"
    );
    // Reading `arguments` through eval is fine.
    assert_eq!(
        eval("function f() { return eval('arguments.length'); } f(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    // Declaring a non-`arguments` var in a function eval is fine.
    assert_eq!(
        eval("function f() { return eval('var nonArg = 5; nonArg'); } f();"),
        Ok(Value::Number(5.0))
    );
    // Global eval may declare `var arguments` freely.
    assert_eq!(
        eval("eval('var arguments = 7'); arguments;"),
        Ok(Value::Number(7.0))
    );
}
