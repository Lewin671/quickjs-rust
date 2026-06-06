use crate::{Value, eval};

#[test]
fn evaluates_string_search_builtins() {
    assert_eq!(eval("'abc'.startsWith('ab');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.startsWith('bc', 1);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("'abc'.startsWith('bc', 2);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let caught = false; try { ''.startsWith(/./); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let re = /a/; re[Symbol.match] = false; '/a/'.startsWith(re);"),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval("let search = { get [Symbol.match]() { throw new Error('match'); } }; ''.startsWith(search);")
            .is_err()
    );
    assert_eq!(eval("'abc'.endsWith('bc');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('ab', 2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('bc', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval(
            "let caught = false; try { ''.endsWith(/./); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let re = /c/; re[Symbol.match] = false; '/c/'.endsWith(re);"),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval("let search = { get [Symbol.match]() { throw new Error('match'); } }; ''.endsWith(search);")
            .is_err()
    );
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
        eval(
            "let caught = false; try { ''.includes(/./); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let re = /b/; re[Symbol.match] = false; 'a /b/ c'.includes(re);"),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval("let search = { get [Symbol.match]() { throw new Error('match'); } }; ''.includes(search);")
            .is_err()
    );
    assert_eq!(
        eval("String.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("String.prototype.search.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.search(/b/);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.search(/z/);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.search('b');"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let calls = 0; let searcher = { [Symbol.search]: function(input) { calls = calls + 1; return this === searcher && input === 'abc' ? 42 : -1; } }; 'abc'.search(searcher) + ':' + calls;"
        ),
        Ok(Value::String("42:1".to_owned()))
    );
    assert!(eval("let searcher = { get [Symbol.search]() { throw new Error('search'); } }; ''.search(searcher);").is_err());
    assert_eq!(
        eval(
            "let searcher = { [Symbol.search]: null, toString: function() { return '\\\\d'; } }; 'ab3'.search(searcher);"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let original = RegExp.prototype[Symbol.search]; RegExp.prototype[Symbol.search] = function(input) { return this.source + ':' + input; }; let result = 'abc'.search('b'); RegExp.prototype[Symbol.search] = original; result;"
        ),
        Ok(Value::String("b:abc".to_owned()))
    );
    assert_eq!(
        eval("new String('test string').search(/String/i);"),
        Ok(Value::Number(5.0))
    );
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
    assert_eq!(
        eval("String.prototype.replaceAll.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("String.prototype.replace.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'foo foo'.replace('foo', 'bar');"),
        Ok(Value::String("bar foo".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let search = { [Symbol.replace]: function(input, replacement) { calls = calls + 1; return this === search && input === 'abc' && replacement === 7 ? 42 : -1; } }; 'abc'.replace(search, 7) + ':' + calls;"
        ),
        Ok(Value::String("42:1".to_owned()))
    );
    assert!(
        eval("let search = { get [Symbol.replace]() { throw new Error('replace'); } }; ''.replace(search, 'x');")
            .is_err()
    );
    assert_eq!(
        eval(
            "let search = { [Symbol.replace]: null, toString: function() { return '3'; } }; 'ab3c'.replace(search, '<foo>');"
        ),
        Ok(Value::String("ab<foo>c".to_owned()))
    );
    assert_eq!(
        eval(
            "let search = { toString: function() { throw 'search'; } }; let replacement = { toString: function() { throw 'replacement'; } }; try { 'abc'.replace(search, replacement); } catch (error) { error; }"
        ),
        Ok(Value::String("search".to_owned()))
    );
    assert_eq!(
        eval(
            "let search = { toString: function() { return 'b'; } }; let replacement = { toString: function() { throw 'replacement'; } }; try { 'abc'.replace(search, replacement); } catch (error) { error; }"
        ),
        Ok(Value::String("replacement".to_owned()))
    );
    assert_eq!(
        eval("'abc'.replace('', '-');"),
        Ok(Value::String("-abc".to_owned()))
    );
    assert_eq!(
        eval("'aba'.replace('a', '[$&:$`:$\\']');"),
        Ok(Value::String("[a::ba]ba".to_owned()))
    );
    assert_eq!(
        eval("'foo-x-bar'.replace(/(x)($^)?/, '|$01:$02:$10:$20|');"),
        Ok(Value::String("foo-|x::x0:0|-bar".to_owned()))
    );
    assert_eq!(
        eval("'uid=31'.replace(/(uid=)(\\d+)/, '$11' + 15);"),
        Ok(Value::String("uid=115".to_owned()))
    );
    assert_eq!(
        eval(
            "'a-b-a'.replace('a', function(match, position, input) { return match + position + input.length; });"
        ),
        Ok(Value::String("a05-b-a".to_owned()))
    );
    assert_eq!(
        eval(
            "'abc12 def34'.replace(/([a-z]+)([0-9]+)/, function() { return arguments[2] + arguments[1]; });"
        ),
        Ok(Value::String("12abc def34".to_owned()))
    );
    assert_eq!(
        eval("'a1b2'.replace(/(\\d)/g, '[$1:$&]');"),
        Ok(Value::String("a[1:1]b[2:2]".to_owned()))
    );
    assert_eq!(
        eval("'asdf'.replace(new RegExp(undefined, 'g'), '1');"),
        Ok(Value::String("1a1s1d1f1".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(String.prototype, 'replace'); (d.value === String.prototype.replace) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval("'foo foo'.replaceAll('foo', 'bar');"),
        Ok(Value::String("bar bar".to_owned()))
    );
    assert_eq!(
        eval("'abc'.replaceAll('', '-');"),
        Ok(Value::String("-a-b-c-".to_owned()))
    );
    assert_eq!(
        eval("'aba'.replaceAll('a', '[$&:$`:$\\']');"),
        Ok(Value::String("[a::ba]b[a:ab:]".to_owned()))
    );
    assert_eq!(
        eval(
            "'a-b-a'.replaceAll('a', function(match, position, input) { return match + position + input.length; });"
        ),
        Ok(Value::String("a05-b-a45".to_owned()))
    );
    assert_eq!(
        eval("'a1b2'.replaceAll(/(\\d)/g, '[$1:$&]');"),
        Ok(Value::String("a[1:1]b[2:2]".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { 'abc'.replaceAll(/a/, 'x'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; let search = { [Symbol.match]: true, get flags() { calls = calls + 1; return 'g'; }, toString: function() { return 'a'; } }; 'aba'.replaceAll(search, 'x') + ':' + calls;"
        ),
        Ok(Value::String("xbx:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let search = { [Symbol.match]: false, get flags() { throw new Error('flags'); }, toString: function() { return 'a'; } }; 'aba'.replaceAll(search, 'x');"
        ),
        Ok(Value::String("xbx".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; let search = { [Symbol.match]: true, flags: undefined }; try { 'abc'.replaceAll(search, 'x'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval(
            "let search = { get [Symbol.match]() { throw new Error('match'); }, toString: function() { throw new Error('toString'); } }; ''.replaceAll(search, 'x');"
        )
        .is_err()
    );
    assert_eq!(
        eval(
            "let calls = 0; let search = /./g; Object.defineProperty(search, Symbol.replace, { value: function(input, replacement) { calls = calls + 1; return this === search && input == 'abc' && replacement === 7 ? 42 : -1; } }); new String('abc').replaceAll(search, 7) + ':' + calls;"
        ),
        Ok(Value::String("42:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let poisoned = 0; let poison = { toString: function() { poisoned = poisoned + 1; throw new Error('poison'); } }; let search = /./g; Object.defineProperty(search, Symbol.replace, { value: function(input, replacement) { return input === poison && replacement === poison ? 'ok' : 'bad'; } }); ''.replaceAll.call(poison, search, poison) + ':' + poisoned;"
        ),
        Ok(Value::String("ok:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let search = { [Symbol.replace]: null, toString: function() { return 'a'; } }; 'aba'.replaceAll(search, 'x');"
        ),
        Ok(Value::String("xbx".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; let search = { [Symbol.match]: false, [Symbol.replace]: 1, toString: function() { throw new Error('toString'); } }; try { ''.replaceAll(search, 'x'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
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
fn evaluates_string_match_global_regexp() {
    assert_eq!(
        eval("'343443444'.match(/34/g).length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("'343443444'.match(/34/g)[1];"),
        Ok(Value::String("34".to_owned()))
    );
    assert_eq!(
        eval("'123456abcde7890'.match(/\\d{2}/g).join(',');"),
        Ok(Value::String("12,34,56,78,90".to_owned()))
    );
    assert_eq!(eval("'abc'.match(/\\d/g);"), Ok(Value::Null));
}

#[test]
fn evaluates_string_match_coercions() {
    assert_eq!(
        eval(
            "let calls = 0; let matcher = { [Symbol.match]: function(input) { calls = calls + 1; return this === matcher && input === 'abc' ? 42 : -1; } }; 'abc'.match(matcher) + ':' + calls;"
        ),
        Ok(Value::String("42:1".to_owned()))
    );
    assert!(
        eval(
            "let matcher = { get [Symbol.match]() { throw new Error('match'); } }; ''.match(matcher);"
        )
        .is_err()
    );
    assert_eq!(
        eval(
            "let matcher = { [Symbol.match]: null, toString: function() { return '\\\\d'; } }; 'ab3'.match(matcher)[0];"
        ),
        Ok(Value::String("3".to_owned()))
    );
    assert_eq!(
        eval(
            "let original = RegExp.prototype[Symbol.match]; let calls = 0; RegExp.prototype[Symbol.match] = function(input) { calls = calls + 1; return this.source + ':' + input; }; let result = 'abc'.match('b'); RegExp.prototype[Symbol.match] = original; result + ':' + calls;"
        ),
        Ok(Value::String("b:abc:1".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.match.call(12345, /34/)[0];"),
        Ok(Value::String("34".to_owned()))
    );
    assert_eq!(
        eval("'12345'.match(34)[0];"),
        Ok(Value::String("34".to_owned()))
    );
    assert_eq!(eval("'12345'.match(34).index;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("'undefined'.match().length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'undefined'.match().index;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("'ABBABAB'.match({ toString: function() { return 'AB'; } })[0];"),
        Ok(Value::String("AB".to_owned()))
    );
    assert_eq!(
        eval(
            "'ABBAB1ABAB1BBAA'.match({ toString: function() { return {}; }, valueOf: function() { return 1; } })[0];"
        ),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught; try { 'ABBABAB'.match({ toString: function() { throw 'intostr'; } }); } catch (error) { caught = error; } caught;"
        ),
        Ok(Value::String("intostr".to_owned()))
    );
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
