use crate::{Value, eval};

#[test]
fn evaluates_string_search_builtins() {
    assert_eq!(eval("'abc'.startsWith('ab');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.startsWith('bc', 1);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("'abc'.startsWith('bc', 2);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("'abc'.endsWith('bc');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('ab', 2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('bc', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'abc'.indexOf('b');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.indexOf('b', 2);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("'aaaa'.indexOf('aa', 'Infinity');"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(eval("'aaaa'.indexOf('aa', {});"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("'abc'.indexOf({ toString: function() { return 'b'; } });"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.includes('b');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.includes('b', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("String.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(String.prototype, 'search'); d.value === String.prototype.search && !d.enumerable && d.writable && d.configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'abc'.search(/b/);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.search(/z/);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'canal'.lastIndexOf('a');"), Ok(Value::Number(3.0)));
    assert_eq!(eval("'canal'.lastIndexOf('a', 2);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'canal'.lastIndexOf('x');"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 1);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 99);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval(
            "'ABBABAB'.lastIndexOf({ toString: function() { return 'AB'; } }, { valueOf: function() { return NaN; } });"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "'ABBABAB'.lastIndexOf('AB', { valueOf: function() { return {}; }, toString: function() {} });"
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn evaluates_string_match_basic_regexp() {
    assert_eq!(
        eval("'abc'.match(/b/)[0];"),
        Ok(Value::String("b".to_owned()))
    );
    assert_eq!(eval("'abc'.match(/z/);"), Ok(Value::Null));
    assert_eq!(eval("'abc'.match(/b/).index;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("'abc'.match(/b/).input;"),
        Ok(Value::String("abc".to_owned()))
    );
}

#[test]
fn evaluates_string_match_coercions() {
    assert_eq!(
        eval("String.prototype.match.call(12345, /34/)[0];"),
        Ok(Value::String("34".to_owned()))
    );
    assert_eq!(
        eval("'12345'.match(34)[0];"),
        Ok(Value::String("34".to_owned()))
    );
    assert_eq!(eval("'12345'.match(34).index;"), Ok(Value::Number(2.0)));
}

#[test]
fn rejects_string_match_null_or_undefined_this() {
    assert_eq!(
        eval(
            "let caught = false; try { String.prototype.match.call(null, /./); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.prototype.match.call(undefined, /./); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
