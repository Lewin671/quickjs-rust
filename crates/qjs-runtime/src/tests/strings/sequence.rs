use crate::{Value, eval};

#[test]
fn evaluates_string_sequence_builtins() {
    assert_eq!(
        eval("'a'.concat('b', 3, true);"),
        Ok(Value::String("ab3true".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.slice(1, 4);"),
        Ok(Value::String("bcd".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.slice(-3);"),
        Ok(Value::String("def".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; f.toString = function() { return f; }; Function.prototype.slice = String.prototype.slice; f.slice(null, Function().slice(f, 5).length);"
        ),
        Ok(Value::String("gnull".to_owned().into()))
    );
    assert_eq!(
        eval("String.prototype.split.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'hello'.split('l').join('|');"),
        Ok(Value::String("he||o".to_owned().into()))
    );
    assert_eq!(
        eval("'hello'.split('l', 2).join('|');"),
        Ok(Value::String("he|".to_owned().into()))
    );
    assert_eq!(
        eval("'hello'.split(undefined).join('|');"),
        Ok(Value::String("hello".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.split('', 2).join('|');"),
        Ok(Value::String("a|b".to_owned().into()))
    );
    assert_eq!(eval("'abc'.split('x').length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.split('b', 0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("'hello'.split(/l/).join('|');"),
        Ok(Value::String("he||o".to_owned().into()))
    );
    assert_eq!(
        eval("'hello'.split(/l/, 2).join('|');"),
        Ok(Value::String("he|".to_owned().into()))
    );
    assert_eq!(
        eval("'one two three'.split(/ /, 2).join('|');"),
        Ok(Value::String("one|two".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.split(/[a-z]/).join('|');"),
        Ok(Value::String("|||".to_owned().into()))
    );
    assert_eq!(
        eval("'x'.split(/.?/).join('|');"),
        Ok(Value::String("|".to_owned().into()))
    );
    assert_eq!(
        eval("'x'.split(/\\w/).join('|');"),
        Ok(Value::String("|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let separator = {}; separator[Symbol.split] = function(input, limit) { calls++; return this === separator && input === 'abc' && limit === 'limit' ? 'ok' : 'bad'; }; 'abc'.split(separator, 'limit') + ':' + calls;"
        ),
        Ok(Value::String("ok:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let separator = { toString: function() { return '2'; }, valueOf: function() { throw 'bad'; } }; separator[Symbol.split] = null; 'a2b2c'.split(separator).join('|');"
        ),
        Ok(Value::String("a|b|c".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(String.prototype, Symbol.split, { configurable: true, get: function() { throw 'bad'; } }); let out = 'a,b,c'.split(',').join('|'); delete String.prototype[Symbol.split]; out;"
        ),
        Ok(Value::String("a|b|c".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; let separator = {}; separator[Symbol.split] = 1; try { 'abc'.split(separator); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'hello'.split(new RegExp).join('|');"),
        Ok(Value::String("h|e|l|l|o".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let original = RegExp.prototype[Symbol.split]; RegExp.prototype[Symbol.split] = function(input, limit) { return this.source + ':' + input + ':' + limit; }; let result = 'abc'.split(/b/, 7); RegExp.prototype[Symbol.split] = original; result;"
        ),
        Ok(Value::String("b:abc:7".to_owned().into()))
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
        Ok(Value::String("bcd".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substring(4, 1);"),
        Ok(Value::String("bcd".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substring(-3, 2);"),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substring(3);"),
        Ok(Value::String("def".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; Function.prototype.substring = String.prototype.substring; f.substring(null, Function());"
        ),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("'abcdef'.substr(1, 3);"),
        Ok(Value::String("bcd".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-2);"),
        Ok(Value::String("ef".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-20, 2);"),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, -1);"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, 2.8);"),
        Ok(Value::String("cd".to_owned().into()))
    );
    assert_eq!(
        eval("'abcdef'.substr(Infinity, 1);"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("'a😀bc'.slice(1, 3) + ':' + 'a😀bc'.substr(1, 2) + ':' + 'a😀bc'.substring(3, 1);"),
        Ok(Value::String("😀:😀:😀".to_owned().into()))
    );
    assert_eq!(
        eval("'😀x'.slice(0, 1).charCodeAt(0) + ':' + '😀x'.slice(1, 2).charCodeAt(0);"),
        Ok(Value::String("55357:56832".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'abcdef'.slice(NaN, Infinity) + ':' + 'abcdef'.slice(-Infinity, -1) + ':' + 'abcdef'.slice(Infinity, -Infinity);"
        ),
        Ok(Value::String("abcdef:abcde:".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let log = ''; let receiver = { toString: function() { log += 'this'; return 'abcdef'; } }; let start = { valueOf: function() { log += ':start'; return 1; } }; let end = { valueOf: function() { log += ':end'; return 4; } }; let out = String.prototype.slice.call(receiver, start, end); out + ':' + log;"
        ),
        Ok(Value::String("bcd:this:start:end".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let converted = false; let index = { valueOf: function() { converted = true; return 1; } }; let caught = false; try { String.prototype.slice.call(Symbol('x'), index, 2); } catch (error) { caught = error instanceof TypeError; } caught + ':' + converted;"
        ),
        Ok(Value::String("true:false".to_owned().into()))
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
fn preserves_utf16_at_the_surrogate_sentinel_boundary() {
    let direct_source = format!(
        "let direct = '{}'; let escaped = '\\u{{F0000}}'; let point = String.fromCodePoint(0xF0000); let units = String.fromCharCode(0xDB80, 0xDC00); [direct.length, direct.codePointAt(0), direct.charCodeAt(0), direct.charCodeAt(1), direct === escaped, direct === point, direct === units].join(':');",
        '\u{F0000}'
    );
    assert_eq!(
        eval(&direct_source),
        Ok(Value::String(
            "2:983040:56192:56320:true:true:true".to_owned().into()
        ))
    );

    assert_eq!(
        eval(
            "let value = '\\u{F0000}'; [value.slice(0, 1).charCodeAt(0), value.slice(1, 2).charCodeAt(0), value.substring(0, 1).charCodeAt(0), value.substring(1, 2).charCodeAt(0), value.substr(0, 1).charCodeAt(0), value.substr(1, 1).charCodeAt(0)].join(':');"
        ),
        Ok(Value::String(
            "56192:56320:56192:56320:56192:56320".to_owned().into()
        ))
    );

    assert_eq!(
        eval(
            "let lone = String.fromCharCode(0xD800); [lone.length, lone.codePointAt(0), lone.charCodeAt(0), lone.slice(0, 1) === lone, lone.substring(0, 1) === lone, lone.substr(0, 1) === lone].join(':');"
        ),
        Ok(Value::String(
            "1:55296:55296:true:true:true".to_owned().into()
        ))
    );

    let template_source = format!(
        "function tag(strings) {{ return strings[0].length + ':' + strings.raw[0].length; }} [tag`{}`, tag`\\u{{F0000}}`].join(':');",
        '\u{F0000}'
    );
    assert_eq!(
        eval(&template_source),
        Ok(Value::String("2:2:2:9".to_owned().into()))
    );

    assert_eq!(
        eval(
            "let value = '\\u{F0000}'; let iterated = Array.from(value)[0]; let decoded = decodeURIComponent('%F3%B0%80%80'); [iterated === value, iterated.length, value.toLowerCase() === value, value.toUpperCase() === value, decoded === value, decoded.length].join(':');"
        ),
        Ok(Value::String("true:2:true:true:true:2".to_owned().into()))
    );

    assert_eq!(
        eval(
            "let value = '\\u{F0000}'; [/.*/u.exec(value)[0] === value, /./u.exec(value)[0] === value, /./u.exec(value)[0].length, /./.exec(value)[0].charCodeAt(0), /^(.)$/u.exec(value)[1] === value, value.match(/./gu).length, value.match(/./g).length, new RegExp(value, 'u').test(value)].join(':');"
        ),
        Ok(Value::String(
            "true:true:2:56192:true:1:2:true".to_owned().into()
        ))
    );

    assert_eq!(
        eval(
            "let value = '\\u{F0000}'; let parts = value.split(''); [encodeURIComponent(value), String.raw({raw: [value]}) === value, value.concat(value).length, value.repeat(2).length, ''.padEnd(2, value) === value, value.replace(/(.)/u, '$1') === value, value.replaceAll(value, value) === value, parts.length, parts[0].charCodeAt(0), parts[1].charCodeAt(0)].join(':');"
        ),
        Ok(Value::String(
            "%F3%B0%80%80:true:4:4:true:true:true:2:56192:56320"
                .to_owned()
                .into()
        ))
    );

    assert_eq!(
        eval(
            "let value = '\\u{F0000}'; let lone = String.fromCharCode(0xD800); let fromEval = eval(\"'\" + value + \"'\"); let loneFromEval = eval(\"'\" + lone + \"'\"); let fromFunction = Function(\"return '\" + value + \"';\")(); [fromEval === value, fromEval.length, loneFromEval === lone, loneFromEval.length, fromFunction === value, fromFunction.length].join(':');"
        ),
        Ok(Value::String("true:2:true:1:true:2".to_owned().into()))
    );
}

#[test]
fn evaluates_string_html_builtins() {
    assert_eq!(
        eval("'x'.bold() + ':' + 'x'.italics() + ':' + 'x'.fixed();"),
        Ok(Value::String(
            "<b>x</b>:<i>x</i>:<tt>x</tt>".to_owned().into()
        ))
    );
    assert_eq!(
        eval("'x'.big() + ':' + 'x'.small() + ':' + 'x'.blink();"),
        Ok(Value::String(
            "<big>x</big>:<small>x</small>:<blink>x</blink>"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("'x'.strike() + ':' + 'x'.sub() + ':' + 'x'.sup();"),
        Ok(Value::String(
            "<strike>x</strike>:<sub>x</sub>:<sup>x</sup>"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("'x'.anchor('a') + ':' + 'x'.link('https://e.test') + ':' + 'x'.fontcolor('red') + ':' + 'x'.fontsize(3);"),
        Ok(Value::String(
            "<a name=\"a\">x</a>:<a href=\"https://e.test\">x</a>:<font color=\"red\">x</font>:<font size=\"3\">x</font>".to_owned().into()
        ))
    );
    assert_eq!(
        eval("'x'.anchor('a\"b') + ':' + String.prototype.bold.call(7);"),
        Ok(Value::String(
            "<a name=\"a&quot;b\">x</a>:<b>7</b>".to_owned().into()
        ))
    );
    assert!(eval("String.prototype.bold.call(null);").is_err());
    assert!(eval("String.prototype.link.call(undefined, 'x');").is_err());
}

#[test]
fn repeat_beyond_max_length_throws_range_error() {
    // Result length must stay within the 2^30-1 string-length limit. The
    // accepted boundary is exercised against QuickJS-NG, not here, to avoid
    // allocating a gigabyte-scale string in the unit test.
    assert!(eval("'a'.repeat(1073741824);").is_err());
    assert!(eval("'ab'.repeat(1073741824);").is_err());
}
