use crate::{Value, eval};

#[test]
fn evaluates_json_builtins() {
    assert_eq!(
        eval("typeof JSON;"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(JSON);"),
        Ok(Value::String("[object JSON]".to_owned().into()))
    );
    assert_eq!(eval("JSON.parse.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("JSON.rawJSON.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("JSON.isRawJSON.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("JSON.stringify.length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("JSON.parse('null');"), Ok(Value::Null));
    assert_eq!(eval("JSON.parse('true');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("JSON.parse('-12.5e2');"), Ok(Value::Number(-1250.0)));
    assert_eq!(
        eval("JSON.parse('\"text\"');"),
        Ok(Value::String("text".to_owned().into()))
    );
    assert_eq!(
        eval("JSON.parse('\"line\\\\nfeed\"');"),
        Ok(Value::String("line\nfeed".to_owned().into()))
    );
    assert_eq!(
        eval("JSON.parse('[1, true, null]')[1];"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let value = JSON.parse('{\"a\":1,\"b\":[2]}'); value.b[0];"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("JSON.stringify({a: 1, b: [true, null], c: undefined});"),
        Ok(Value::String(
            "{\"a\":1,\"b\":[true,null]}".to_owned().into()
        ))
    );
    assert_eq!(
        eval("JSON.stringify(['x', undefined, NaN, Infinity]);"),
        Ok(Value::String("[\"x\",null,null,null]".to_owned().into()))
    );
    assert_eq!(eval("JSON.stringify(undefined);"), Ok(Value::Undefined));
    assert_eq!(
        eval("JSON.stringify(JSON.rawJSON(1.1));"),
        Ok(Value::String("1.1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let parsed = JSON.parse(JSON.stringify({x: JSON.rawJSON('true'), y: JSON.rawJSON('\"text\"')})); parsed.x + ':' + parsed.y;"
        ),
        Ok(Value::String("true:text".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let raw = JSON.rawJSON(null); Object.getPrototypeOf(raw) === null && Object.hasOwn(raw, 'rawJSON') && Object.getOwnPropertyNames(raw).join() === 'rawJSON' && Object.isFrozen(raw) && JSON.isRawJSON(raw) && !JSON.isRawJSON({ rawJSON: 'null' });"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { JSON.parse('{bad'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { JSON.rawJSON([]); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { JSON.rawJSON('{\"x\":1}'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let rejected = 0; for (let text of ['', ' 1', '1 ', '\\t1', '1\\n', '\\r1', '1\\r']) { try { JSON.rawJSON(text); } catch (error) { if (error instanceof SyntaxError) rejected++; } } rejected;"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn json_stringify_observes_replacer_and_wrapper_semantics() {
    assert_eq!(
        eval(
            "let n = new Number(10); n.toString = function() { return 'toString'; }; n.valueOf = function() { throw new Error('bad'); }; JSON.stringify({10: 1, toString: 2}, [n]);"
        ),
        Ok(Value::String("{\"toString\":2}".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let s = new String('str'); s.toString = function() { return 'toString'; }; s.valueOf = function() { throw new Error('bad'); }; JSON.stringify({str: 1, toString: 2}, [s]);"
        ),
        Ok(Value::String("{\"toString\":2}".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let value = {}; let wrapper; JSON.stringify(value, function() { wrapper = this; }); Object.getPrototypeOf(wrapper) === Object.prototype && Object.getOwnPropertyNames(wrapper).join() === '' && Object.getOwnPropertyDescriptor(wrapper, '').value === value;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn json_stringify_space_and_primitive_wrappers_use_conversion() {
    assert_eq!(
        eval("JSON.stringify({first: undefined, keep: 1, last: function() {}}, null, 2);"),
        Ok(Value::String("{\n  \"keep\": 1\n}".to_owned().into()))
    );
    assert_eq!(
        eval("JSON.stringify({a:{b:1}}, null, new Number(2));"),
        Ok(Value::String(
            "{\n  \"a\": {\n    \"b\": 1\n  }\n}".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let n = new Number(1); n.valueOf = function() { return 3; }; JSON.stringify({a:1}, null, n);"
        ),
        Ok(Value::String("{\n   \"a\": 1\n}".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let s = new String('x'); s.toString = function() { return '--'; }; JSON.stringify({a:1}, null, s);"
        ),
        Ok(Value::String("{\n--\"a\": 1\n}".to_owned().into()))
    );
    assert_eq!(
        eval("let n = new Number(1); n.valueOf = function() { return 2; }; JSON.stringify([n]);"),
        Ok(Value::String("[2]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let s = new String('x'); s.toString = function() { return 'ok'; }; JSON.stringify([s]);"
        ),
        Ok(Value::String("[\"ok\"]".to_owned().into()))
    );
}

#[test]
fn json_stringify_to_json_proxy_and_cycle_semantics() {
    assert_eq!(
        eval("let a = [true]; a.toJSON = function() {}; JSON.stringify(a);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("JSON.stringify({ toJSON: function() { return [false]; } });"),
        Ok(Value::String("[false]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let objectProxy = new Proxy({}, { getOwnPropertyDescriptor() { return { value: 1, writable: true, enumerable: true, configurable: true }; }, get() { return 1; }, ownKeys() { return ['a', 'b']; } }); JSON.stringify(new Proxy(objectProxy, {}));"
        ),
        Ok(Value::String("{\"a\":1,\"b\":1}".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let arr = []; let circular = [arr]; arr.toJSON = function() { return circular; }; let caught = false; try { JSON.stringify(circular); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn json_stringify_escapes_strings_and_unpaired_surrogates() {
    assert_eq!(
        eval(r#"JSON.stringify("😀\"\\\\\b\f\n\r\t\u0001");"#),
        Ok(Value::String(
            r#""😀\"\\\\\b\f\n\r\t\u0001""#.to_owned().into()
        ))
    );
    assert_eq!(
        eval("JSON.stringify(String.fromCharCode(0xD83D, 0xDE00));"),
        Ok(Value::String("\"😀\"".to_owned().into()))
    );
    assert_eq!(
        eval("JSON.stringify(String.fromCharCode(0xD834));"),
        Ok(Value::String("\"\\ud834\"".to_owned().into()))
    );
}

#[test]
fn json_preserves_wtf16_at_the_surrogate_sentinel_boundary() {
    let direct_source = format!(
        "let direct = '{}'; let parsed = JSON.parse('\\\"{}\\\"'); let escaped = JSON.parse('\\\"\\\\udb80\\\\udc00\\\"'); let lone = JSON.parse('\\\"\\\\ud800\\\"'); [parsed === direct, parsed.length, escaped === direct, escaped.length, lone.length, lone.charCodeAt(0), JSON.parse(JSON.stringify(direct)) === direct].join(':');",
        '\u{F0000}', '\u{F0000}'
    );
    assert_eq!(
        eval(&direct_source),
        Ok(Value::String(
            "true:2:true:2:1:55296:true".to_owned().into()
        ))
    );
}

#[test]
fn json_parse_reviver_observes_context_and_forward_modifications() {
    assert_eq!(
        eval(
            "let log = []; JSON.parse('{\"a\":1,\"b\":[2]}', function(k, v, c) { log.push(k + ':' + String(c.source)); return v; }); log.join('|');"
        ),
        Ok(Value::String(
            "a:1|0:2|b:undefined|:undefined".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let log = []; let out = JSON.parse('[1,2]', function(k, v, c) { log.push(k + ':' + String(v) + ':' + String(c.source)); if (k === '0') this[1] = 3; return this[k]; }); out.join(',') + '|' + log.join('|');"
        ),
        Ok(Value::String(
            "1,3|0:1:1|1:3:undefined|:1,3:undefined".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let wrapper; JSON.parse('2', function() { wrapper = this; }); Object.getPrototypeOf(wrapper) === Object.prototype && Object.getOwnPropertyNames(wrapper).join() === '' && Object.getOwnPropertyDescriptor(wrapper, '').value === 2;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn json_parse_reviver_uses_property_internal_methods() {
    assert_eq!(
        eval(
            "let object = JSON.parse('{\"a\":1,\"b\":2}', function(k, v) { if (k === 'a') Object.defineProperty(this, 'b', { configurable: false }); if (k === 'b') return 22; return v; }); object.a + ':' + object.b;"
        ),
        Ok(Value::String("1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let array = JSON.parse('[1,2]', function(k, v) { if (k === '0') Object.defineProperty(this, '1', { configurable: false }); if (k === '1') return; return v; }); array[0] + ':' + array.hasOwnProperty('1') + ':' + array[1];"
        ),
        Ok(Value::String("1:true:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let marker = {}; let bad = new Proxy([0], { deleteProperty() { throw marker; } }); let caught = false; try { JSON.parse('[0,0]', function(k, v) { if (k === '0') this[1] = bad; }); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = {}; let bad = new Proxy({}, { ownKeys() { throw marker; } }); let caught = false; try { JSON.parse('[0,0]', function(k, v) { if (k === '0') this[1] = bad; return v; }); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = { a: 1 }; let proxy = new Proxy(target, { ownKeys() { return ['a']; }, getOwnPropertyDescriptor() { return { value: 1, enumerable: true, configurable: true }; }, get(t, k) { return t[k]; } }); let log = []; JSON.parse('[0,0]', function(k, v) { if (k === '0') this[1] = proxy; log.push(k); return v; }); log.join(',');"
        ),
        Ok(Value::String("0,a,1,".to_owned().into()))
    );
}
