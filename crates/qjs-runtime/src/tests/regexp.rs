use crate::{EvalErrorKind, Value, eval, eval_classified};

#[test]
fn rejects_invalid_regexp_literal_at_parse_phase() {
    // A regexp literal with an invalid pattern/flags must fail before the
    // script body runs, mirroring Test262's `negative: phase: parse` cases.
    for source in [
        "throw 'unreached'; /]/u;",
        "throw 'unreached'; /}/u;",
        "throw 'unreached'; /(/;",
        "throw 'unreached'; /a/gg;",
        "throw 'unreached'; /\\2(a)/u;",
        "throw 'unreached'; /.(?<!.){2,3}/;",
        "throw 'unreached'; /a/biu;",
        "throw 'unreached'; /[\\d-a]/u;",
        "throw 'unreached'; /[%-\\d]/u;",
        "throw 'unreached'; /[\\s-\\d]/u;",
        "throw 'unreached'; /[\\uFFFF-\\p{Hex}]/u;",
        "throw 'unreached'; /(?<a>\\a)/u;",
        "throw 'unreached'; /\\x/u;",
        "throw 'unreached'; /\\u123/u;",
        "throw 'unreached'; /[\\B]/u;",
        "throw 'unreached'; /[\\c0]/u;",
    ] {
        let error = eval_classified(source).expect_err("invalid regexp literal must fail");
        // Invalid regexp literals are parse-phase errors (kind=parse), so the
        // harness accepts them for Test262 `negative: phase: parse` cases.
        assert_eq!(error.kind, EvalErrorKind::Parse, "source: {source}");
        assert!(
            error.message.contains("SyntaxError"),
            "expected SyntaxError, got {} for {source}",
            error.message
        );
    }
}

#[test]
fn rejects_malformed_named_group_specifiers_at_parse_phase() {
    // `(?<name>` must be a well-formed RegExpIdentifierName; a malformed name
    // is a parse-phase SyntaxError for regexp literals (Test262
    // `negative: phase: parse`).
    for source in [
        "throw 'unreached'; /(?<>a)/;",            // empty name
        "throw 'unreached'; /(?<42a>a)/;",         // non-identifier-start digit
        "throw 'unreached'; /(?<a.b>a)/;",         // punctuator in name
        "throw 'unreached'; /(?<a)/;",             // unterminated name
        "throw 'unreached'; /(?<\\u2764>a)/;",     // a non-ID code point
        "throw 'unreached'; /(?<\\uD800>a)/u;",    // lone surrogate
        "throw 'unreached'; /(?<\\u{1f98a}>a)/u;", // astral non-ID (emoji)
    ] {
        let error = eval_classified(source).expect_err("invalid group name must fail");
        assert_eq!(error.kind, EvalErrorKind::Parse, "source: {source}");
        assert!(
            error.message.contains("SyntaxError"),
            "expected SyntaxError, got {} for {source}",
            error.message
        );
    }
}

#[test]
fn accepts_well_formed_named_group_specifiers() {
    // Plain, `$`/`_`, `\u` escape, raw astral ID_Start, CJK, and a ZWJ part are
    // all valid RegExpIdentifierName forms regardless of the `u` flag.
    for source in [
        "/(?<name>a)/.source",
        "/(?<$_a>a)/.source",
        "/(?<\\u0041>a)/.source",
        "/(?<\\u{1d453}o>a)/u.source",
        "/(?<\u{1d453}\u{1d45c}\u{1d465}>a)/u.source",
        "/(?<\u{72f8}>a)/u.source",
    ] {
        assert!(
            eval(source).is_ok(),
            "expected a valid group name to compile: {source}"
        );
    }
    // The group is usable: backreference and `.groups` access still work.
    assert_eq!(
        eval("let m = 'ab'.match(/(?<g>a)(?<h>b)/); m.groups.g + m.groups.h"),
        Ok(Value::String("ab".to_owned().into()))
    );
}

#[test]
fn accepts_valid_regexp_literal_during_compilation() {
    assert_eq!(
        eval("/[0-9]+/g.source;"),
        Ok(Value::String("[0-9]+".to_owned().into()))
    );
    // A genuine `new RegExp(...)` with a runtime-built invalid pattern still
    // fails at runtime, not at the parse/early stage.
    let error = eval_classified("new RegExp('(');").expect_err("invalid pattern must fail");
    assert_eq!(error.kind, EvalErrorKind::Runtime);
}

#[test]
fn evaluates_regexp_constructor_identity() {
    assert_eq!(
        eval("typeof RegExp;"),
        Ok(Value::String("function".to_owned().into()))
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
        Ok(Value::String("[object RegExp]".to_owned().into()))
    );
    assert_eq!(
        eval("new RegExp('test').toString();"),
        Ok(Value::String("/test/".to_owned().into()))
    );
    assert_eq!(
        eval("let obj = { constructor: RegExp }; obj[Symbol.match] = true; RegExp(obj) === obj;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let obj = { source: 'source text', flags: 'i' }; obj[Symbol.match] = []; let result = new RegExp(obj); Object.getPrototypeOf(result) === RegExp.prototype && result.source + ':' + result.flags;"
        ),
        Ok(Value::String("source text:i".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let realmPrototype = {}; \
             function C() {} \
             Object.defineProperty(C, '__quickjsRustRealmRegExpPrototype', { value: realmPrototype }); \
             C.prototype = null; \
             Object.getPrototypeOf(Reflect.construct(RegExp, [], C)) === realmPrototype;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let obj = { source: 'source text' }; Object.defineProperty(obj, 'flags', { get: function() { throw 'flags'; } }); obj[Symbol.match] = true; let result = new RegExp(obj, 'g'); result.source + ':' + result.flags;"
        ),
        Ok(Value::String("source text:g".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let thrown = {}; let obj = {}; Object.defineProperty(obj, 'source', { get: function() { throw thrown; } }); obj[Symbol.match] = true; let caught = false; try { new RegExp(obj); } catch (error) { caught = error === thrown; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/test/.toString();"),
        Ok(Value::String("/test/".to_owned().into()))
    );
    assert_eq!(
        eval("/\\n/iyg.toString();"),
        Ok(Value::String("/\\n/giy".to_owned().into()))
    );
    assert_eq!(
        eval("/test/.test('a test value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/missing/.test('a test value');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let re = new RegExp(''); let d = Object.getOwnPropertyDescriptor(re, 'lastIndex'); re.lastIndex + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("0:true:false:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /./; let d = Object.getOwnPropertyDescriptor(re, 'lastIndex'); d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:false:false".to_owned().into()))
    );
}

#[test]
fn exposes_regexp_species_accessor() {
    assert_eq!(
        eval("RegExp[Symbol.species] === RegExp;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let desc = Object.getOwnPropertyDescriptor(RegExp, Symbol.species); let receiver = {}; [desc.get.call(receiver) === receiver, desc.set === undefined, desc.enumerable, desc.configurable, desc.get.name, desc.get.length].join(':');"
        ),
        Ok(Value::String(
            "true:true:false:true:get [Symbol.species]:0"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn evaluates_regexp_escape() {
    assert_eq!(
        eval("typeof RegExp.escape;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("RegExp.escape.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("RegExp.escape('abc123');"),
        Ok(Value::String("\\x61bc123".to_owned().into()))
    );
    assert_eq!(
        eval(r#"RegExp.escape('^$\\.*+?()[]{}|/');"#),
        Ok(Value::String(
            "\\^\\$\\\\\\.\\*\\+\\?\\(\\)\\[\\]\\{\\}\\|\\/"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval(r#"RegExp.escape(",-=<>#&!%:;@~'`\"");"#),
        Ok(Value::String(
            "\\x2c\\x2d\\x3d\\x3c\\x3e\\x23\\x26\\x21\\x25\\x3a\\x3b\\x40\\x7e\\x27\\x60\\x22"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("RegExp.escape('\\t\\n\\v\\f\\r ');"),
        Ok(Value::String("\\t\\n\\v\\f\\r\\x20".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.escape(String.fromCharCode(0x00a0, 0x2028, 0xfeff));"),
        Ok(Value::String("\\xa0\\u2028\\ufeff".to_owned().into()))
    );
    assert_eq!(
        eval(r#"RegExp.escape("\ud800\udc00");"#),
        Ok(Value::String("\\ud800\\udc00".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.escape(String.fromCharCode(0x100));"),
        Ok(Value::String(String::from_utf16(&[0x100]).unwrap().into()))
    );
    assert!(eval("RegExp.escape(123);").is_err());
    assert!(eval("RegExp.escape(null);").is_err());
}

#[test]
fn evaluates_regexp_prototype_compile() {
    assert_eq!(
        eval("typeof RegExp.prototype.compile;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype.compile.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let re = /abc/gi; let same = re.compile('def'); (same === re) + ':' + re.source + ':' + re.flags + ':' + re.test('def') + ':' + re.test('DEF') + ':' + re.lastIndex;"
        ),
        Ok(Value::String("true:def::true:false:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /abc/g; let source = /def/i; source.lastIndex = 4; re.lastIndex = 9; let same = re.compile(source); (same === re) + ':' + (source.lastIndex === 4) + ':' + re.source + ':' + re.flags + ':' + re.test('DEF') + ':' + re.lastIndex;"
        ),
        Ok(Value::String("true:true:def:i:true:0".to_owned().into()))
    );
    assert_eq!(
        eval("let re = /abc/; re.compile(); re.source + ':' + re.test('');"),
        Ok(Value::String("(?:):true".to_owned().into()))
    );
    assert!(eval("RegExp.prototype.compile.call({}, 'abc');").is_err());
    assert!(eval("RegExp.prototype.compile.call(null, 'abc');").is_err());
    assert!(eval("/abc/.compile(/def/, 'g');").is_err());
    assert_eq!(
        eval(
            "let re = /abc/; Object.defineProperty(re, 'lastIndex', { value: 45, writable: false }); let caught = false; try { re.compile(/def/g); } catch (error) { caught = error instanceof TypeError; } caught + ':' + re.toString() + ':' + re.lastIndex;"
        ),
        Ok(Value::String("true:/def/g:45".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /test262/gi; let caught = false; try { re.compile('', 'igi'); } catch (error) { caught = error instanceof SyntaxError; } caught + ':' + re.toString() + ':' + re.test('TEsT262');"
        ),
        Ok(Value::String("true:/test262/gi:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /test262/gi; let caught = false; try { re.compile('.{2,1}'); } catch (error) { caught = error instanceof SyntaxError; } caught + ':' + re.toString() + ':' + re.test('TEsT262');"
        ),
        Ok(Value::String("true:/test262/gi:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { new RegExp('^[z-a]$'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = /test262/gi; let caught = false; try { re.compile('\\\\2', 'u'); } catch (error) { caught = error instanceof SyntaxError; } caught + ':' + re.toString() + ':' + re.test('TEsT262');"
        ),
        Ok(Value::String("true:/test262/gi:true".to_owned().into()))
    );
}

#[test]
fn evaluates_regexp_prototype_accessors() {
    assert_eq!(
        eval("/test/g.source;"),
        Ok(Value::String("test".to_owned().into()))
    );
    assert_eq!(eval("/test/g.global;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/s.dotAll;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/.dotAll;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("/./s.test('\\n');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/./.test('\\n');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("/^.$/.test('\\u{10300}');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("/^.$/u.test('\\u{10300}');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/i.ignoreCase;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/m.multiline;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/.unicodeSets;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("/test/v.unicodeSets;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("RegExp.prototype.unicodeSets;"), Ok(Value::Undefined));
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(RegExp.prototype, 'unicodeSets').get.name;"),
        Ok(Value::String("get unicodeSets".to_owned().into()))
    );
    assert!(eval("Object.create(RegExp.prototype).unicodeSets;").is_err());
    assert_eq!(
        eval("new RegExp('.', 'v').unicodeSets;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/test/v.flags;"),
        Ok(Value::String("v".to_owned().into()))
    );
    assert_eq!(
        eval("/test/vdgy.flags;"),
        Ok(Value::String("dgvy".to_owned().into()))
    );
    assert!(eval("new RegExp('.', 'uv');").is_err());
    assert!(eval("new RegExp('.', 'vu');").is_err());
    assert!(eval("new RegExp('[(]', 'v');").is_err());
    assert_eq!(eval("/test/.global;"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("/test/iyg.flags;"),
        Ok(Value::String("giy".to_owned().into()))
    );
    assert_eq!(
        eval("new RegExp('').source;"),
        Ok(Value::String("(?:)".to_owned().into()))
    );
    assert_eq!(
        eval("new RegExp('/').source;"),
        Ok(Value::String("\\/".to_owned().into()))
    );
    assert_eq!(
        eval("eval('/' + new RegExp('/').source + '/').test('/');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/\u{1d306}/u.source;"#),
        Ok(Value::String("\\u{1d306}".to_owned().into()))
    );
    assert_eq!(
        eval(r#"/\ud834\udf06/u.test("𝌆");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/\ud834\udf06/u.test("\ud834\udf06");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval(r#"/\u{1d306}/u.test("𝌆");"#), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(r#"/\u{1d306}/u.test("\ud834\udf06");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/\u{1d306}/u.test("x𝌆y");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/\u{1d306}/u.exec("x𝌆y").index;"#),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(r#"/[\u{1d306}]/u.test("𝌆");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = eval('/' + new RegExp('\\n').source + '/'); re.test('\\n') && re.test('_\\n_') && !re.test('n');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = eval('/' + new RegExp('\\r').source + '/'); re.test('\\r') && !re.test('r');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new RegExp(String.fromCharCode(0x2028)).source;"),
        Ok(Value::String("\\u2028".to_owned().into()))
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
        Ok(Value::String("(?:)".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype.flags;"),
        Ok(Value::String(String::new().into()))
    );
    assert_eq!(eval("RegExp.prototype.global;"), Ok(Value::Undefined));
    assert_eq!(eval("RegExp.prototype.dotAll;"), Ok(Value::Undefined));
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source').get; let caught = false; try { get.call({}); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source').get; let caught = false; try { get.call(Symbol()); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(RegExp.prototype, 'flags').get; function throwsTypeError(value) { try { get.call(value); return false; } catch (error) { return error instanceof TypeError; } } throwsTypeError(undefined) + ':' + throwsTypeError(null) + ':' + throwsTypeError(4) + ':' + throwsTypeError('string') + ':' + throwsTypeError(false) + ':' + throwsTypeError(Symbol()) + ':' + throwsTypeError(4n);"
        ),
        Ok(Value::String(
            "true:true:true:true:true:true:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let get = Object.getOwnPropertyDescriptor(RegExp.prototype, 'global').get; \
             let caught = false; \
             try { get.call({ global: true }); } catch (error) { caught = error instanceof TypeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function Test262Error() {} \
             Test262Error.prototype.toString = function() { return 'Test262Error'; }; \
             /a\\n/.source;"
        ),
        Ok(Value::String("a\\n".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /a/; \
             Object.defineProperty(re, 'source', { get: function() { return 'own'; } }); \
             re.source;"
        ),
        Ok(Value::String("own".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let original = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source'); \
             Object.defineProperty(RegExp.prototype, 'source', { get: function() { return 'prototype'; }, configurable: true }); \
             let result = /a/.source; \
             Object.defineProperty(RegExp.prototype, 'source', original); \
             result;"
        ),
        Ok(Value::String("prototype".to_owned().into()))
    );
}

#[test]
fn evaluates_regexp_exec_literal_match() {
    assert_eq!(
        eval("/test/.exec('a test value')[0];"),
        Ok(Value::String("test".to_owned().into()))
    );
    assert_eq!(eval("/missing/.exec('a test value');"), Ok(Value::Null));
    assert_eq!(
        eval("/test/.exec('a test value').index;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("/test/.exec('a test value').input;"),
        Ok(Value::String("a test value".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp('\\\\u0037+').exec('a777b')[0];"),
        Ok(Value::String("777".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp('\\\\s+').exec('a \\t b')[0].length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("new RegExp('\\\\cA').test('\\x01');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = new RegExp('\\\\c' + String.fromCharCode(0x0410)); re.test('\\\\c' + String.fromCharCode(0x0410)) + ':' + re.test('c' + String.fromCharCode(0x0410));"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "new RegExp('[\\\\c!]').test('\\\\') + ':' + new RegExp('[\\\\c!]').test('c') + ':' + new RegExp('[\\\\c!]').test('!') + ':' + new RegExp('[\\\\c!]').test('\\x01');"
        ),
        Ok(Value::String("true:true:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            r#"/\k<a>/.test("k<a>") + ":" + /\k<a>\1/.test("k<a>\x01") + ":" + /\1(b)\k<a>/.test("bk<a>");"#
        ),
        Ok(Value::String("true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "/\\s/.test('\\u0085') + ':' + /\\S/.test('\\u0085') + ':' + /[\\s]/.test('\\u202f') + ':' + /[\\S]/.test('\\u180e');"
        ),
        Ok(Value::String("false:true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval("/String/i.exec('test string')[0];"),
        Ok(Value::String("string".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let r = /[\\d][\\12-\\14]{1,}[^\\d]/.exec('line1\\n\\n\\n\\n\\nline2'); r.length + ':' + r.index + ':' + r[0];"
        ),
        Ok(Value::String("1:4:1\n\n\n\n\nl".to_owned().into()))
    );
    assert_eq!(
        eval(
            "/]/.test(']') + ':' + /{/.test('{') + ':' + /}/.test('}') + ':' + /x{o}x/.test('x{o}x');"
        ),
        Ok(Value::String("true:true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "/\\00/.exec('\\x00')[0].charCodeAt(0) + ':' + /\\07/.exec('\\x07')[0].charCodeAt(0) + ':' + /\\0111/.exec('\\x091')[0].length + ':' + /\\0003/.exec('\\x003')[0].length;"
        ),
        Ok(Value::String("0:7:2:2".to_owned().into()))
    );
}

#[test]
fn evaluates_regexp_exec_global_last_index() {
    assert_eq!(
        eval(
            "let re = /34/g; let first = re.exec('343443444'); first[0] + ':' + first.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("34:0:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /34/g; re.exec('343443444'); let second = re.exec('343443444'); second[0] + ':' + second.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("34:2:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /34/g; re.lastIndex = 8; re.exec('343443444') === null && re.lastIndex === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = /./ug; let match = re.exec('\\uD834\\uDF06'); match.index + ':' + match[0].length + ':' + re.lastIndex;"
        ),
        Ok(Value::String("0:2:2".to_owned().into()))
    );
    assert_eq!(
        eval("/a/u.exec('\\uD834\\uDF06a').index;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let re = /(?:)/ug; re.lastIndex = 3; re.exec('\\uD834\\uDF06') === null && re.lastIndex === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let gets = 0; let counter = { valueOf: function() { gets = gets + 1; return 0; } }; let re = /a/; re.lastIndex = counter; let result = re.exec('nbc'); (result === null) + ':' + (re.lastIndex === counter) + ':' + gets;"
        ),
        Ok(Value::String("true:true:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let gets = 0; let counter = { valueOf: function() { gets = gets + 1; return 0; } }; let re = /./; re.lastIndex = counter; let result = re.exec('abc'); result[0] + ':' + (re.lastIndex === counter) + ':' + gets;"
        ),
        Ok(Value::String("a:true:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /./g; Object.defineProperty(re, 'lastIndex', { writable: false }); let caught = false; try { re.exec('abc'); } catch (error) { caught = error instanceof TypeError; } caught + ':' + re.lastIndex;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
}

#[test]
fn evaluates_regexp_symbol_search() {
    assert_eq!(
        eval("RegExp.prototype[Symbol.search].name;"),
        Ok(Value::String("[Symbol.search]".to_owned().into()))
    );
    assert_eq!(eval("/b/[Symbol.search]('abc');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("/z/[Symbol.search]('abc');"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval(
            "let value = 86; let re = { get lastIndex() { return value; }, set lastIndex(next) { value = next; }, exec() { value = null; return null; } }; RegExp.prototype[Symbol.search].call(re); value;"
        ),
        Ok(Value::Number(86.0))
    );
    assert_eq!(
        eval(
            "let re = { exec() { return Symbol(); } }; let caught = false; try { RegExp.prototype[Symbol.search].call(re, 'a'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/\\udf06/u[Symbol.search]('\\ud834\\udf06');"),
        Ok(Value::Number(-1.0))
    );
}

#[test]
fn evaluates_regexp_symbol_match_all() {
    assert_eq!(
        eval("typeof RegExp.prototype[Symbol.matchAll];"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.matchAll].length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.matchAll].name;"),
        Ok(Value::String("[Symbol.matchAll]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.from(/a./g[Symbol.matchAll]('a1 a2')).map(function(match) { return match[0] + ':' + match.index + ':' + match.input; }).join('|');"
        ),
        Ok(Value::String("a1:0:a1 a2|a2:3:a1 a2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /a/g; re.lastIndex = 1; let result = Array.from(re[Symbol.matchAll]('aba')).map(function(match) { return match.index; }).join(','); re.lastIndex + ':' + result;"
        ),
        Ok(Value::String("1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let it = RegExp.prototype[Symbol.matchAll].call(/a/, 'aba'); let first = it.next(); let second = it.next(); first.value.index + ':' + first.value[0] + ':' + second.done;"
        ),
        Ok(Value::String("0:a:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.from(/(?:)/g[Symbol.matchAll]('a')).map(function(match) { return match.index; }).join(',');"
        ),
        Ok(Value::String("0,1".to_owned().into()))
    );
}

#[test]
fn regexp_match_all_uses_species_constructor() {
    // The matcher is built through SpeciesConstructor(R, %RegExp%), invoked once
    // with (R, flags) before R's lastIndex is read.
    assert_eq!(
        eval(
            "let re = /\\d/u; let args; \
             re.constructor = { [Symbol.species]: function() { args = arguments; return /\\w/g; } }; \
             let iter = re[Symbol.matchAll]('a*b'); \
             args.length + ':' + (args[0] === re) + ':' + args[1] + ':' + Array.from(iter).map(m => m[0]).join(',');"
        ),
        Ok(Value::String("2:true:u:a".to_owned().into()))
    );
    // A non-object, non-undefined constructor (including a Symbol) is a TypeError.
    assert!(eval("let re = /./; re.constructor = null; re[Symbol.matchAll]('');").is_err());
    assert!(eval("let re = /./; re.constructor = Symbol(); re[Symbol.matchAll]('');").is_err());
    // ToLength(lastIndex) runs and its coercion is observable.
    assert!(
        eval("let re = /./; re.lastIndex = { valueOf() { throw new TypeError('x'); } }; re[Symbol.matchAll]('');")
            .is_err()
    );
}

#[test]
fn evaluates_regexp_symbol_match() {
    assert_eq!(
        eval(
            "typeof RegExp.prototype[Symbol.match] + ':' + RegExp.prototype[Symbol.match].length + ':' + RegExp.prototype[Symbol.match].name;"
        ),
        Ok(Value::String("function:1:[Symbol.match]".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.match].call(/a./, 'a1 a2')[0];"),
        Ok(Value::String("a1".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.match].call(/a./g, 'a1 a2').join('|');"),
        Ok(Value::String("a1|a2".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.match].call(/z/g, 'a1 a2');"),
        Ok(Value::Null)
    );
    assert_eq!(
        eval("let re = /(?:)/g; re[Symbol.match]('a').join('|') + ':' + re.lastIndex;"),
        Ok(Value::String("|:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let re = { global: false, exec(input) { calls = calls + 1; return { 0: input + ':' + calls, index: 0, length: 1 }; } }; RegExp.prototype[Symbol.match].call(re, 123)[0];"
        ),
        Ok(Value::String("123:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = { flags: 'g', lastIndex: 0, exec() { if (this.lastIndex === 0) { this.lastIndex = 1; return { 0: 'a', index: 0, length: 1 }; } return null; } }; RegExp.prototype[Symbol.match].call(re, 'abc').join('|') + ':' + re.lastIndex;"
        ),
        Ok(Value::String("a:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = { get flags() { throw 'flags'; }, get global() { throw 'global'; }, exec() { return null; } }; let caught = false; try { RegExp.prototype[Symbol.match].call(re, 'abc'); } catch (error) { caught = error === 'flags'; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_regexp_symbol_replace() {
    assert_eq!(
        eval(
            "typeof RegExp.prototype[Symbol.replace] + ':' + RegExp.prototype[Symbol.replace].length + ':' + RegExp.prototype[Symbol.replace].name;"
        ),
        Ok(Value::String(
            "function:2:[Symbol.replace]".to_owned().into()
        ))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.replace].call(/a./, 'a1 a2', 'x');"),
        Ok(Value::String("x a2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let obj = { get flags() { throw new Error('flags'); }, get global() { throw new Error('global'); } }; \
             try { RegExp.prototype[Symbol.replace].call(obj, '', ''); 'none'; } catch (error) { error.message; }"
        ),
        Ok(Value::String("flags".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /./g; \
             Object.defineProperty(re, 'flags', { get: function() { return { [Symbol.toPrimitive]: function(hint) { if (hint === 'string') { throw new Error('coerce'); } } }; } }); \
             Object.defineProperty(re, 'global', { get: function() { throw new Error('global'); } }); \
             try { re[Symbol.replace]('', ''); 'none'; } catch (error) { error.message; }"
        ),
        Ok(Value::String("coerce".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /./; \
             Object.defineProperty(re, 'unicode', { get: function() { throw new Error('unicode'); } }); \
             try { re[Symbol.replace]('', ''); 'none'; } catch (error) { error.message; }"
        ),
        Ok(Value::String("unicode".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.replace].call(/a(.)/g, 'a1 a2', '[$1:$&]');"),
        Ok(Value::String("[1:a1] [2:a2]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "RegExp.prototype[Symbol.replace].call(/(\\d)/g, 'a1b2', function(match, digit, position, input) { return digit + ':' + position + ':' + input.length; });"
        ),
        Ok(Value::String("a1:1:4b2:3:4".to_owned().into()))
    );
    assert_eq!(
        eval("let re = /(?:)/g; 'a'.replace(re, '-');"),
        Ok(Value::String("-a-".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /^|\\udf06/g; \
             Object.defineProperty(re, 'unicode', { writable: true }); \
             re.unicode = false; \
             let falsy = re[Symbol.replace]('\\ud834\\udf06', 'XXX'); \
             re.unicode = true; \
             let truthy = re[Symbol.replace]('\\ud834\\udf06', 'XXX'); \
             [falsy.length, falsy.charCodeAt(3), falsy.slice(4), truthy.length, truthy.charCodeAt(3), truthy.charCodeAt(4)].join(':');"
        ),
        Ok(Value::String("7:55348:XXX:5:55348:57094".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /a/g; re.lastIndex = 1; let result = re[Symbol.replace]('aba', 'x'); result + ':' + re.lastIndex;"
        ),
        Ok(Value::String("xbx:0".to_owned().into()))
    );
    // A non-functional replace performs ToObject(namedCaptures) eagerly when
    // `exec` reports groups, so `groups: null` throws a TypeError even when the
    // replacement string has no `$<name>` reference.
    assert_eq!(
        eval(
            "var re = /./; re.exec = function () { return { length: 1, 0: '', index: 0, groups: null }; }; \
             try { re[Symbol.replace]('bar', ''); 'no throw'; } catch (error) { error instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    // `undefined` groups (the common case) are left untouched.
    assert_eq!(
        eval(
            "var re = /./; re.exec = function () { return { length: 1, 0: 'b', index: 0, groups: undefined }; }; \
             re[Symbol.replace]('bar', 'X');"
        ),
        Ok(Value::String("Xar".to_owned().into()))
    );
}

#[test]
fn evaluates_regexp_exec_and_test_sticky_last_index() {
    assert_eq!(
        eval("let re = /abc/y; re.test('abc') + ':' + re.lastIndex;"),
        Ok(Value::String("true:3".to_owned().into()))
    );
    assert_eq!(
        eval("let re = /b/y; re.test('ab') + ':' + re.lastIndex;"),
        Ok(Value::String("false:0".to_owned().into()))
    );
    assert_eq!(
        eval("let re = /./y; re.lastIndex = 1; re.test('a') + ':' + re.lastIndex;"),
        Ok(Value::String("false:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /b/y; re.lastIndex = 1; let result = re.exec('abc'); result[0] + ':' + result.index + ':' + re.lastIndex;"
        ),
        Ok(Value::String("b:1:2".to_owned().into()))
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
        Ok(Value::String("02134".to_owned().into()))
    );
    assert_eq!(
        eval(r#"'Boston, MA 02134'.match(/([\d]{5})([-\ ]?[\d]{4})?$/)[2];"#),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(r#"/(uid=)(\d+)/.exec('uid=31')[1] + '|' + /(uid=)(\d+)/.exec('uid=31')[2];"#),
        Ok(Value::String("uid=|31".to_owned().into()))
    );
    assert_eq!(
        eval(r#"/((x))/.exec('foo-x-bar')[1] + '|' + /((x))/.exec('foo-x-bar')[2];"#),
        Ok(Value::String("x|x".to_owned().into()))
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
fn evaluates_regexp_symbol_split() {
    assert_eq!(
        eval("RegExp.prototype[Symbol.split].name;"),
        Ok(Value::String("[Symbol.split]".to_owned().into()))
    );
    assert_eq!(
        eval("RegExp.prototype[Symbol.split].length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("/d/[Symbol.split]('abcdefg').join('|');"),
        Ok(Value::String("abc|efg".to_owned().into()))
    );
    assert_eq!(
        eval("/x/[Symbol.split]('axbxcxdxe', 3).join('|');"),
        Ok(Value::String("a|b|c".to_owned().into()))
    );
    assert_eq!(
        eval("/c(d)(e)/[Symbol.split]('abcdefg', 2).join('|');"),
        Ok(Value::String("ab|d".to_owned().into()))
    );
    assert_eq!(
        eval("/(?:)/[Symbol.split]('').length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("/./[Symbol.split]('').join('|');"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("let result = /\\uDF06/u[Symbol.split]('\\uD834\\uDF06'); result.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let result = /./u[Symbol.split]('\\uD834\\uDF06'); result.length + ':' + result.join('|');"
        ),
        Ok(Value::String("2:|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let thrown = {}; let obj = { flags: '', get constructor() { throw thrown; } }; let caught = false; try { RegExp.prototype[Symbol.split].call(obj, 'abc'); } catch (error) { caught = error === thrown; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let obj = { flags: '', constructor: false }; let caught = false; try { RegExp.prototype[Symbol.split].call(obj, 'abc'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let flagsArg; let re = {}; re.flags = 'i'; re.constructor = function() {}; re.constructor[Symbol.species] = function(_, flags) { flagsArg = flags; return /b/y; }; RegExp.prototype[Symbol.split].call(re, 'abc').join('|') + ':' + flagsArg;"
        ),
        Ok(Value::String("a|c:iy".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let re = /x/; re.constructor = function() {}; re.constructor[Symbol.species] = 1; let caught = false; try { re[Symbol.split]('abc'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let re = /a/; Object.defineProperty(re, Symbol.match, { get: function() { re.compile('b'); } }); let result = re[Symbol.split]('abba'); result.length + ':' + result.join('|');"
        ),
        Ok(Value::String("3:a||a".to_owned().into()))
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

#[test]
fn evaluates_regexp_named_capture_groups() {
    // The `groups` object exposes named captures and is null-prototyped.
    assert_eq!(
        eval(
            r#"let m = /(?<year>\d{4})-(?<month>\d{2})/.exec("2024-06"); m.groups.year + "/" + m.groups.month;"#
        ),
        Ok(Value::String("2024/06".to_owned().into()))
    );
    assert_eq!(
        eval(r#"Object.getPrototypeOf(/(?<a>.)/.exec("x").groups);"#),
        Ok(Value::Null)
    );
    // `groups` is undefined when there are no named groups.
    assert_eq!(eval(r#"/(\d+)/.exec("42").groups;"#), Ok(Value::Undefined));
    // Unmatched named groups are present with value undefined.
    assert_eq!(
        eval(
            r#"let m = /(?<a>x)|(?<b>y)/.exec("y"); m.groups.a === undefined && m.groups.b === "y";"#
        ),
        Ok(Value::Boolean(true))
    );
    // `\k<name>` backreferences.
    assert_eq!(
        eval(r#"/(?<c>.)\k<c>/.test("aa");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/^(?<c>.)\k<c>$/.test("ab");"#),
        Ok(Value::Boolean(false))
    );
    // `$<name>` substitution in String.prototype.replace.
    assert_eq!(
        eval(r#""2024-06".replace(/(?<y>\d{4})-(?<m>\d{2})/, "$<m>/$<y>");"#),
        Ok(Value::String("06/2024".to_owned().into()))
    );
    // Lookahead and lookbehind assertions.
    assert_eq!(
        eval(r#"/(?<=\$)\d+/.exec("$100")[0];"#),
        Ok(Value::String("100".to_owned().into()))
    );
    assert_eq!(eval(r#"/q(?=u)/.test("queue");"#), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(r#"/q(?!u)/.test("queue");"#),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_regexp_match_indices_d_flag() {
    // No `indices` property without the `d` flag.
    assert_eq!(eval(r#"/a/.exec("a").indices;"#), Ok(Value::Undefined));
    // Whole-match and capture index pairs are code-unit positions.
    assert_eq!(
        eval(
            r#"let m = /(a)(b)/d.exec("xab"); m.indices[0].join(",") + ":" + m.indices[1].join(",") + ":" + m.indices[2].join(",");"#
        ),
        Ok(Value::String("1,3:1,2:2,3".to_owned().into()))
    );
    // Unmatched optional groups produce undefined entries.
    assert_eq!(
        eval(r#"let m = /(a)(b)?/d.exec("a"); m.indices[2];"#),
        Ok(Value::Undefined)
    );
    // The indices array carries a `groups` object for named captures, or
    // undefined when there are none.
    assert_eq!(
        eval(r#"/a/d.exec("a").indices.groups;"#),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(r#"let m = /(?<g>b)/d.exec("ab"); m.indices.groups.g.join(",");"#),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert_eq!(
        eval(r#"Object.getPrototypeOf(/(?<g>b)/d.exec("ab").indices.groups);"#),
        Ok(Value::Null)
    );
    // Astral matches report code-unit (UTF-16) positions under the `u` flag.
    assert_eq!(
        eval(
            r#"let m = /(?<emoji>\u{1F600})/du.exec("x\u{1F600}y"); m.indices.groups.emoji.join(",");"#
        ),
        Ok(Value::String("1,3".to_owned().into()))
    );
}

#[test]
fn word_boundary_assertions_match_zero_width() {
    // `\b` / `\B` are zero-width word-boundary assertions, not the literal `b`.
    assert_eq!(
        eval(r#"/\bp/.exec("pilot soviet")[0];"#),
        Ok(Value::String("p".to_owned().into()))
    );
    assert_eq!(
        eval(r#"/\bword\b/.test("a word here");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"/\Bevil\B/.test("devils");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#""the cat sat".match(/\bcat\b/)[0];"#),
        Ok(Value::String("cat".to_owned().into()))
    );
    assert_eq!(
        eval(r#"/\bcat\b/.test("category");"#),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn null_character_escape_in_unicode_mode_matches_nul() {
    // In unicode mode `\0` (not followed by a decimal digit) is the NUL
    // character escape, not the literal `0`.
    assert_eq!(
        eval(r#"/\0/u.test(String.fromCharCode(0));"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval(r#"/\0/u.test("0");"#), Ok(Value::Boolean(false)));
    // Non-unicode `\0` is the legacy octal NUL escape and is unchanged.
    assert_eq!(
        eval(r#"/\0/.test(String.fromCharCode(0));"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval(r#"/\0/.test("0");"#), Ok(Value::Boolean(false)));
}

#[test]
fn null_character_escape_in_unicode_character_class_matches_nul() {
    // Character classes use the same `\0` CharacterEscape semantics under `u`.
    assert_eq!(
        eval(r#"/[\0]/u.test(String.fromCharCode(0));"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval(r#"/[\0]/u.test("0");"#), Ok(Value::Boolean(false)));
    assert_eq!(
        eval(r#"/[\0-\u0001]/u.test(String.fromCharCode(1));"#),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn unicode_mode_rejects_legacy_octal_and_invalid_decimal_escapes() {
    assert_eq!(eval(r#"/(a)\1/u.test("aa");"#), Ok(Value::Boolean(true)));
    assert!(eval(r#"new RegExp("\\00", "u");"#).is_err());
    assert!(eval(r#"new RegExp("\\01", "u");"#).is_err());
    assert!(eval(r#"new RegExp("\\1", "u");"#).is_err());
    assert!(eval(r#"new RegExp("[\\00]", "u");"#).is_err());
    assert!(eval(r#"new RegExp("[\\1]", "u");"#).is_err());
    assert!(eval(r#"/\00/u;"#).is_err());
    assert!(eval(r#"/[\1]/u;"#).is_err());
}

#[test]
fn unicode_mode_rejects_invalid_identity_escapes() {
    for source in [
        r#"new RegExp("\\a", "u");"#,
        r#"new RegExp("\\x", "u");"#,
        r#"new RegExp("\\x1", "u");"#,
        r#"new RegExp("\\u", "u");"#,
        r#"new RegExp("\\u123", "u");"#,
        r#"new RegExp("\\u{}", "u");"#,
        r#"new RegExp("[\\a]", "u");"#,
        r#"new RegExp("[\\B]", "u");"#,
        r#"new RegExp("[\\x1]", "u");"#,
        r#"new RegExp("[\\u123]", "u");"#,
        r#"new RegExp("[\\c0]", "u");"#,
    ] {
        assert!(eval(source).is_err(), "source: {source}");
    }
    assert_eq!(
        eval(r#"new RegExp("\\x41", "u").test("A");"#),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(r#"new RegExp("[\\-\\]\\b\\x41\\u0042\\u{43}]", "u").test("-");"#),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn named_group_unicode_escapes_decode_to_property_key() {
    // A `(?<A>...)` name must be decoded to `A` so the match's `groups`
    // object and `\k<...>` backreference use the decoded key.
    assert_eq!(
        eval(r#"new RegExp("(?<\\u0041>x)").exec("x").groups.A;"#),
        Ok(Value::String("x".to_owned().into()))
    );
    assert_eq!(
        eval(r#"/(?<\u{03C0}>a)/u.exec("bab").groups.\u{03C0};"#),
        Ok(Value::String("a".to_owned().into()))
    );
    // A `\k<\u...>` backreference resolves against the decoded name.
    assert_eq!(
        eval(r#"/(?<A>.)\k<A>/.test("aa");"#),
        Ok(Value::Boolean(true))
    );
}
