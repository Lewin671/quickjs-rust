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
            "let desc = Object.getOwnPropertyDescriptor(this, 'undefined'); desc.value === undefined && desc.writable === false && desc.enumerable === false && desc.configurable === false;"
        ),
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
    assert_eq!(eval("eval(7);"), Ok(Value::Number(7.0)));
    assert_eq!(eval("eval('1 + 2;');"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let value = 1; eval('value = value + 2;'); value;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { eval('break label'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { while (true) { eval('continue label'); } } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'use strict'; let caught = false; (function() { try { eval('try {} catch (eval) {}'); } catch (error) { caught = error instanceof SyntaxError; } })(); caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("(function() { return eval('try { throw 1; } catch (eval) { eval; }'); })();"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "'use strict'; let caught = false; try { eval('var arguments;'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'use strict'; let caught = false; try { eval('arguments = 42;'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'use strict'; let caught = false; try { eval('function foo() { eval = 42; } foo();'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_uri_global_builtins() {
    assert_eq!(eval("encodeURIComponent.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(this, 'encodeURIComponent'); desc.value === encodeURIComponent && desc.writable && !desc.enumerable && desc.configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "delete encodeURI.length; !encodeURI.hasOwnProperty('length') && encodeURI.length === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("encodeURI('http://example.test/a b?x=1&y=✓');"),
        Ok(Value::String(
            "http://example.test/a%20b?x=1&y=%E2%9C%93".to_owned()
        ))
    );
    assert_eq!(
        eval("encodeURIComponent('a b?x=1&y=✓');"),
        Ok(Value::String("a%20b%3Fx%3D1%26y%3D%E2%9C%93".to_owned()))
    );
    assert_eq!(
        eval("decodeURI('http://example.test/a%20b?x=1&y=%E2%9C%93');"),
        Ok(Value::String("http://example.test/a b?x=1&y=✓".to_owned()))
    );
    assert_eq!(
        eval("decodeURI('%3B%2F%3F%3A%40%26%3D%2B%24%2C%23');"),
        Ok(Value::String(
            "%3B%2F%3F%3A%40%26%3D%2B%24%2C%23".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "decodeURI('%D0%AE%D0%BD%D0%B8%D0%BA%D0%BE%D0%B4%23%D0%92%D0%B5%D1%80%D1%81%D0%B8%D0%B8');"
        ),
        Ok(Value::String("Юникод%23Версии".to_owned()))
    );
    assert_eq!(
        eval("decodeURIComponent('a%20b%3Fx%3D1%26y%3D%E2%9C%93');"),
        Ok(Value::String("a b?x=1&y=✓".to_owned()))
    );
    assert!(eval("decodeURIComponent('%zz');").is_err());
}
