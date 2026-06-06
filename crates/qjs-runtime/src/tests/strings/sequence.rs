use crate::{Value, eval};

#[test]
fn evaluates_string_sequence_builtins() {
    assert_eq!(
        eval("'a'.concat('b', 3, true);"),
        Ok(Value::String("ab3true".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.slice(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.slice(-3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; f.toString = function() { return f; }; Function.prototype.slice = String.prototype.slice; f.slice(null, Function().slice(f, 5).length);"
        ),
        Ok(Value::String("gnull".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.split.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'hello'.split('l').join('|');"),
        Ok(Value::String("he||o".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split('l', 2).join('|');"),
        Ok(Value::String("he|".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(undefined).join('|');"),
        Ok(Value::String("hello".to_owned()))
    );
    assert_eq!(
        eval("'abc'.split('', 2).join('|');"),
        Ok(Value::String("a|b".to_owned()))
    );
    assert_eq!(eval("'abc'.split('x').length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.split('b', 0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("'hello'.split(/l/).join('|');"),
        Ok(Value::String("he||o".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(/l/, 2).join('|');"),
        Ok(Value::String("he|".to_owned()))
    );
    assert_eq!(
        eval("'one two three'.split(/ /, 2).join('|');"),
        Ok(Value::String("one|two".to_owned()))
    );
    assert_eq!(
        eval("'abc'.split(/[a-z]/).join('|');"),
        Ok(Value::String("|||".to_owned()))
    );
    assert_eq!(
        eval("'x'.split(/.?/).join('|');"),
        Ok(Value::String("|".to_owned()))
    );
    assert_eq!(
        eval("'x'.split(/\\w/).join('|');"),
        Ok(Value::String("|".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let separator = {}; separator[Symbol.split] = function(input, limit) { calls++; return this === separator && input === 'abc' && limit === 'limit' ? 'ok' : 'bad'; }; 'abc'.split(separator, 'limit') + ':' + calls;"
        ),
        Ok(Value::String("ok:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let separator = { toString: function() { return '2'; }, valueOf: function() { throw 'bad'; } }; separator[Symbol.split] = null; 'a2b2c'.split(separator).join('|');"
        ),
        Ok(Value::String("a|b|c".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(String.prototype, Symbol.split, { configurable: true, get: function() { throw 'bad'; } }); let out = 'a,b,c'.split(',').join('|'); delete String.prototype[Symbol.split]; out;"
        ),
        Ok(Value::String("a|b|c".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; let separator = {}; separator[Symbol.split] = 1; try { 'abc'.split(separator); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'hello'.split(new RegExp).join('|');"),
        Ok(Value::String("h|e|l|l|o".to_owned()))
    );
    assert_eq!(
        eval(
            "let called = false; let separator = { toString: function() { called = true; return 'x'; } }; 'abc'.split(separator, 0); called;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.substring.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("String.prototype.substr.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'abcdef'.substring(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(4, 1);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(-3, 2);"),
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; Function.prototype.substring = String.prototype.substring; f.substring(null, Function());"
        ),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("'abcdef'.substr(1, 3);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-2);"),
        Ok(Value::String("ef".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-20, 2);"),
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, -1);"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, 2.8);"),
        Ok(Value::String("cd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(Infinity, 1);"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { ''.repeat(Infinity); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { ''.repeat(-1); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_string_html_builtins() {
    assert_eq!(
        eval("'x'.bold() + ':' + 'x'.italics() + ':' + 'x'.fixed();"),
        Ok(Value::String("<b>x</b>:<i>x</i>:<tt>x</tt>".to_owned()))
    );
    assert_eq!(
        eval("'x'.big() + ':' + 'x'.small() + ':' + 'x'.blink();"),
        Ok(Value::String(
            "<big>x</big>:<small>x</small>:<blink>x</blink>".to_owned()
        ))
    );
    assert_eq!(
        eval("'x'.strike() + ':' + 'x'.sub() + ':' + 'x'.sup();"),
        Ok(Value::String(
            "<strike>x</strike>:<sub>x</sub>:<sup>x</sup>".to_owned()
        ))
    );
    assert_eq!(
        eval("'x'.anchor('a') + ':' + 'x'.link('https://e.test') + ':' + 'x'.fontcolor('red') + ':' + 'x'.fontsize(3);"),
        Ok(Value::String(
            "<a name=\"a\">x</a>:<a href=\"https://e.test\">x</a>:<font color=\"red\">x</font>:<font size=\"3\">x</font>".to_owned()
        ))
    );
    assert_eq!(
        eval("'x'.anchor('a\"b') + ':' + String.prototype.bold.call(7);"),
        Ok(Value::String(
            "<a name=\"a&quot;b\">x</a>:<b>7</b>".to_owned()
        ))
    );
    assert!(eval("String.prototype.bold.call(null);").is_err());
    assert!(eval("String.prototype.link.call(undefined, 'x');").is_err());
}
