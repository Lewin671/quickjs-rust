mod builtins;
mod constructor;
mod flat_map;
mod flatten;
mod indexing;
mod iteration;
mod mutation;
mod search;
mod sequence;
mod sort;
mod splice;

#[test]
fn array_index_keys_accept_only_canonical_decimal_forms() {
    use crate::{Value, eval};
    // An array index property key is the canonical decimal form of its value.
    // Non-canonical spellings are ordinary string properties, so they must not
    // participate in indexed storage or in `length`.
    assert_eq!(
        eval("var a = []; a['0'] = 1; a.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("var a = []; a['00'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['01'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a[' 1'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['1 '] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['+1'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['-1'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['1.0'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['1e1'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a[''] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['4294967294'] = 1; a.length;"),
        Ok(Value::Number(4294967295.0))
    );
    assert_eq!(
        eval("var a = []; a['4294967295'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("var a = []; a['9999999999999'] = 1; a.length;"),
        Ok(Value::Number(0.0))
    );
    // The non-canonical spellings are still readable as ordinary properties.
    assert_eq!(
        eval("var a = []; a['01'] = 7; a['01'] + ':' + Object.keys(a).join(',');"),
        Ok(Value::String("7:01".to_owned().into()))
    );
    // Typed arrays use the same canonical-form rule for integer indices.
    // `'00'` is not a canonical numeric index string, so on a typed array it
    // is an ordinary property rather than an integer-indexed element.
    assert_eq!(
        eval("var t = new Int32Array(2); t['0'] = 5; t['00'] = 9; t[0] + ':' + t['00'];"),
        Ok(Value::String("5:9".to_owned().into()))
    );
}
