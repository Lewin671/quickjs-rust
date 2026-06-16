use crate::{Value, eval};

#[test]
fn uint8_array_set_from_base64_decodes_with_options() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255,255,255,255]); \
             let r = a.setFromBase64('Zm9vYmE='); \
             r.read + ':' + r.written + ':' + a.join(',');"
        ),
        Ok(Value::String("8:5:102,111,111,98,97,255".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255,255]); \
             let r = a.setFromBase64('x-_y', { alphabet: 'base64url' }); \
             r.read + ':' + r.written + ':' + a.join(',');"
        ),
        Ok(Value::String("4:3:199,239,242,255".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255,255,255,255]); \
             let loose = a.setFromBase64('ZXhhZg'); \
             let b = new Uint8Array([255,255,255,255,255,255]); \
             let stop = b.setFromBase64('ZXhhZg', { lastChunkHandling: 'stop-before-partial' }); \
             loose.read + ':' + loose.written + ':' + a.join(',') + '|' + \
             stop.read + ':' + stop.written + ':' + b.join(',');"
        ),
        Ok(Value::String(
            "6:4:101,120,97,102,255,255|4:3:101,120,97,255,255,255".to_owned()
        ))
    );
}

#[test]
fn uint8_array_set_from_base64_surface_and_errors() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'setFromBase64'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.prototype.setFromBase64.name + ':' \
             + Uint8Array.prototype.setFromBase64.length;"
        ),
        Ok(Value::String("true:false:true:setFromBase64:1".to_owned()))
    );
    assert!(eval("new Uint8Array.prototype.setFromBase64('');").is_err());
    assert!(eval("Uint8Array.prototype.setFromBase64.call(new Int8Array(1), 'AA==');").is_err());
    assert!(eval("new Uint8Array(1).setFromBase64({ toString() { throw 'no'; } });").is_err());
    assert!(
        eval("new Uint8Array(1).setFromBase64('AA==', { alphabet: Object('base64') });").is_err()
    );
    assert!(
        eval("new Uint8Array(1).setFromBase64('AA==', { lastChunkHandling: Object('strict') });")
            .is_err()
    );
    assert!(eval("new Uint8Array(1).setFromBase64('A');").is_err());
    assert!(
        eval("new Uint8Array(1).setFromBase64('AA=', { lastChunkHandling: 'loose' });").is_err()
    );
}

#[test]
fn uint8_array_set_from_base64_target_size_and_error_writes() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255,255,255]); \
             let r = a.setFromBase64('Zm9vYmFy'); \
             r.read + ':' + r.written + ':' + a.join(',');"
        ),
        Ok(Value::String("4:3:102,111,111,255,255".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255,255,255]); \
             try { a.setFromBase64('MjYyZm.9v'); } catch (e) { (e instanceof SyntaxError) + ':' + a.join(','); }"
        ),
        Ok(Value::String("true:50,54,50,255,255".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255]); \
             let r = a.setFromBase64('aaaa#', { lastChunkHandling: 'strict' }); \
             r.read + ':' + r.written + ':' + a.join(',');"
        ),
        Ok(Value::String("4:3:105,166,154".to_owned()))
    );
}

#[test]
fn uint8_array_set_from_base64_validation_order() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); \
             let calls = 0; \
             let options = { get alphabet() { calls++; return 'base64'; } }; \
             try { a.setFromBase64('AA==', options); } catch (e) { (e instanceof TypeError) + ':' + calls + ':' + a[0]; }"
        ),
        Ok(Value::String("true:0:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([255,255,255]); \
             let calls = 0; \
             let options = {}; \
             Object.defineProperty(options, 'alphabet', { get() { calls++; __quickjsRustDetachArrayBuffer(a.buffer); return 'base64'; } }); \
             try { a.setFromBase64('Zg==', options); } catch (e) { (e instanceof TypeError) + ':' + calls; }"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let optionCalls = 0; \
             let options = { get alphabet() { optionCalls++; throw 'no'; } }; \
             let arg = { toString() { throw 'no'; } }; \
             try { new Uint8Array(1).setFromBase64(arg, options); } catch (e) { (e instanceof TypeError) + ':' + optionCalls; }"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
}
