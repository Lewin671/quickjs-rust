use crate::{Value, eval};

#[test]
fn evaluates_regexp_constructor_identity() {
    assert_eq!(
        eval("typeof RegExp;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("RegExp.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("new RegExp() instanceof RegExp;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("/./ instanceof RegExp;"), Ok(Value::Boolean(true)));
    assert!(eval("[].find(/./);").is_err());
    assert_eq!(
        eval("Object.prototype.toString.call(new RegExp());"),
        Ok(Value::String("[object RegExp]".to_owned()))
    );
    assert_eq!(
        eval("new RegExp('test').toString();"),
        Ok(Value::String("/test/".to_owned()))
    );
    assert_eq!(
        eval("/test/.toString();"),
        Ok(Value::String("/test/".to_owned()))
    );
    assert_eq!(
        eval("/test/.test('a test value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/missing/.test('a test value');"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_regexp_prototype_accessors() {
    assert_eq!(
        eval("/test/g.source;"),
        Ok(Value::String("test".to_owned()))
    );
    assert_eq!(eval("/test/g.global;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/i.ignoreCase;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/m.multiline;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/.global;"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("new RegExp('').source;"),
        Ok(Value::String("(?:)".to_owned()))
    );
    assert_eq!(
        eval("new RegExp('/').source;"),
        Ok(Value::String("\\/".to_owned()))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(RegExp.prototype, 'global').set;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(RegExp.prototype, 'global').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(RegExp.prototype, 'global').configurable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("RegExp.prototype.source;"),
        Ok(Value::String("(?:)".to_owned()))
    );
    assert_eq!(eval("RegExp.prototype.global;"), Ok(Value::Undefined));
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source').get; let caught = false; try { get.call({}); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_regexp_exec_literal_match() {
    assert_eq!(
        eval("/test/.exec('a test value')[0];"),
        Ok(Value::String("test".to_owned()))
    );
    assert_eq!(eval("/missing/.exec('a test value');"), Ok(Value::Null));
    assert_eq!(
        eval("/test/.exec('a test value').index;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("/test/.exec('a test value').input;"),
        Ok(Value::String("a test value".to_owned()))
    );
    assert_eq!(
        eval("RegExp('\\\\u0037+').exec('a777b')[0];"),
        Ok(Value::String("777".to_owned()))
    );
    assert_eq!(
        eval("RegExp('\\\\s+').exec('a \\t b')[0].length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("/String/i.exec('test string')[0];"),
        Ok(Value::String("string".to_owned()))
    );
}

#[test]
fn evaluates_regexp_exec_global_last_index() {
    assert_eq!(
        eval(
            "let re = /34/g; let first = re.exec('343443444'); first[0] + ':' + first.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("34:0:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let re = /34/g; re.exec('343443444'); let second = re.exec('343443444'); second[0] + ':' + second.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("34:2:4".to_owned()))
    );
    assert_eq!(
        eval(
            "let re = /34/g; re.lastIndex = 8; re.exec('343443444') === null && re.lastIndex === 0;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_regexp_exec_and_test_sticky_last_index() {
    assert_eq!(
        eval("let re = /abc/y; re.test('abc') + ':' + re.lastIndex;"),
        Ok(Value::String("true:3".to_owned()))
    );
    assert_eq!(
        eval("let re = /b/y; re.test('ab') + ':' + re.lastIndex;"),
        Ok(Value::String("false:0".to_owned()))
    );
    assert_eq!(
        eval("let re = /./y; re.lastIndex = 1; re.test('a') + ':' + re.lastIndex;"),
        Ok(Value::String("false:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let re = /b/y; re.lastIndex = 1; let result = re.exec('abc'); result[0] + ':' + result.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("b:1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let re = /c/y; Object.defineProperty(re, 'lastIndex', { writable: false }); let caught = false; try { re.test('abc'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_regexp_exec_captures() {
    assert_eq!(
        eval(r#"'Boston, MA 02134'.match(/([\d]{5})([-\ ]?[\d]{4})?$/).length;"#),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(r#"'Boston, MA 02134'.match(/([\d]{5})([-\ ]?[\d]{4})?$/)[1];"#),
        Ok(Value::String("02134".to_owned()))
    );
    assert_eq!(
        eval(r#"'Boston, MA 02134'.match(/([\d]{5})([-\ ]?[\d]{4})?$/)[2];"#),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_regexp_exec_empty_non_capturing_group() {
    assert_eq!(eval("RegExp().exec('').length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("RegExp().exec('undefined').index;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn evaluates_regexp_exec_date_format_shape() {
    assert_eq!(
        eval(
            r#"/^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$/.exec(new Date(0).toString()) !== null;"#
        ),
        Ok(Value::Boolean(true))
    );
}
