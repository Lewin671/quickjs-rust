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
