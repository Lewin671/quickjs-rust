use crate::{Value, eval};

#[test]
fn uint8_array_set_from_hex_decodes_and_reports_progress() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([0, 0, 0, 0]); \
             let r = a.setFromHex('0aFf10'); \
             a.join(',') + '|' + r.read + ':' + r.written;"
        ),
        Ok(Value::String("10,255,16,0|6:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let base = new Uint8Array([1, 2, 3, 4]); \
             let a = base.subarray(1, 3); \
             let r = a.setFromHex('aabbcc'); \
             a.join(',') + '|' + base.join(',') + '|' + r.read + ':' + r.written;"
        ),
        Ok(Value::String("170,187|1,170,187,4|4:2".to_owned()))
    );
}

#[test]
fn uint8_array_from_hex_decodes_and_checks_surface() {
    assert_eq!(
        eval(
            "let a = Uint8Array.fromHex('666F6f626172'); \
             (Object.getPrototypeOf(a) === Uint8Array.prototype) + ':' + \
             a.length + ':' + a.buffer.byteLength + ':' + a.join(',');"
        ),
        Ok(Value::String("true:6:6:102,111,111,98,97,114".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array, 'fromHex'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.fromHex.name + ':' + Uint8Array.fromHex.length;"
        ),
        Ok(Value::String("true:false:true:fromHex:1".to_owned()))
    );
    assert_eq!(
        eval(
            "class Subclass extends Uint8Array { constructor() { throw 'bad'; } }; \
             let fromSubclass = Subclass.fromHex('aa'); \
             let fromBare = (0, Uint8Array.fromHex)('bb'); \
             (Object.getPrototypeOf(fromSubclass) === Uint8Array.prototype) + ':' + \
             fromSubclass[0] + ':' + fromBare[0];"
        ),
        Ok(Value::String("true:170:187".to_owned()))
    );
}

#[test]
fn uint8_array_from_hex_rejects_invalid_inputs() {
    assert!(eval("new Uint8Array.fromHex('');").is_err());
    assert!(eval("Uint8Array.fromHex('a');").is_err());
    assert!(eval("Uint8Array.fromHex('a a');").is_err());
    assert!(eval("Uint8Array.fromHex('a\\ta');").is_err());
    assert!(eval("Uint8Array.fromHex('aa^');").is_err());
    assert!(eval("Uint8Array.fromHex({ toString() { throw 'no'; } });").is_err());
}

#[test]
fn uint8_array_to_hex_encodes_and_checks_surface() {
    assert_eq!(
        eval(
            "[
               new Uint8Array([]).toHex(),
               new Uint8Array([0]).toHex(),
               new Uint8Array([10, 15, 16, 255]).toHex(),
               Uint8Array.fromHex('666f6f').subarray(1).toHex()
             ].join('|');"
        ),
        Ok(Value::String("|00|0a0f10ff|6f6f".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'toHex'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.prototype.toHex.name + ':' \
             + Uint8Array.prototype.toHex.length;"
        ),
        Ok(Value::String("true:false:true:toHex:0".to_owned()))
    );
    assert!(eval("new Uint8Array.prototype.toHex();").is_err());
    assert!(eval("Uint8Array.prototype.toHex.call(new Int8Array([1]));").is_err());
    assert!(eval("Uint8Array.prototype.toHex.call({});").is_err());
}

#[test]
fn uint8_array_set_from_hex_surface_and_receiver_checks() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'setFromHex'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.prototype.setFromHex.name + ':' \
             + Uint8Array.prototype.setFromHex.length;"
        ),
        Ok(Value::String("true:false:true:setFromHex:1".to_owned()))
    );
    assert!(eval("new Uint8Array.prototype.setFromHex();").is_err());
    assert!(eval("Uint8Array.prototype.setFromHex.call(new Int8Array(1), '00');").is_err());
    assert!(eval("Uint8Array.prototype.setFromHex.call({}, '00');").is_err());
}

#[test]
fn uint8_array_set_from_hex_errors_preserve_specified_writes() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             try { a.setFromHex('aaa'); } catch (e) { (e instanceof SyntaxError) + ':' + a.join(','); }"
        ),
        Ok(Value::String("true:1,2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             try { a.setFromHex('aaa '); } catch (e) { (e instanceof SyntaxError) + ':' + a.join(','); }"
        ),
        Ok(Value::String("true:170,2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); \
             let arg = { toString() { a[0] = 99; return '00'; } }; \
             try { a.setFromHex(arg); } catch (e) { (e instanceof TypeError) + ':' + a[0]; }"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
}
