//! Statically named property operands resolve to the same keys as their
//! computed spellings, and to nothing else, whatever form a key is stored in.

use crate::{Value, eval};

fn string(text: &str) -> Value {
    Value::string_from_utf8(text)
}

#[test]
fn named_and_computed_spellings_reach_one_property() {
    assert_eq!(
        eval(
            "var o = {}; o.k = 1; o['k'] += 1; o['' + 'k'] *= 5;
             var r = o.k; delete o.k; r + ':' + ('k' in o) + ':' + Object.keys(o).length;"
        ),
        Ok(string("10:false:0"))
    );
}

#[test]
fn numeric_spellings_stay_distinct() {
    assert_eq!(
        eval(
            "var o = { '01': 'padded', 1: 'index' };
             [o['01'], o[1], o['1'], o[01], Object.keys(o).join()].join('|');"
        ),
        Ok(string("padded|index|index|index|1,01"))
    );
    // A named operand that spells a canonical index still reaches the
    // indexed element of an array, and a non-canonical one does not.
    assert_eq!(
        eval("var a = ['x', 'y']; var r = { l: a.length }; r.l + ':' + a['1'] + ':' + a['01'];"),
        Ok(string("2:y:undefined"))
    );
}

#[test]
fn surrogate_keys_match_by_code_units() {
    // Lone halves are exact keys, distinct from each other and from U+FFFD.
    // (A key built by joining two halves still misses the literal pair's key
    // on the computed path: stores compare stored bytes until T056 M1.2.)
    assert_eq!(
        eval(
            "var o = {}; o['\\u{1F600}'] = 'pair'; o['\\uD83D'] = 'lone';
             [o['\\u{1F600}'], o['\\uD83D'], o['\\uDE00'], o['\\uFFFD'], Object.keys(o).length].join('|');"
        ),
        Ok(string("pair|lone|||2"))
    );
    // An astral identifier name is a named operand.
    assert_eq!(
        eval("var o = {}; o.\u{102A7} = 3; o['\\u{102A7}'] + o.\u{102A7};"),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn symbols_and_private_names_never_alias_strings() {
    assert_eq!(
        eval(
            "var s = Symbol('k'); var o = { k: 'string' }; o[s] = 'symbol';
             class C { #k = 'private'; read(x) { return x.#k + ':' + x.k; } }
             var c = new C(); c.k = 'public';
             [o.k, o[s], o['Symbol(k)'], c.read(c)].join('|');"
        ),
        Ok(string("string|symbol||private:public"))
    );
}

#[test]
fn named_access_preserves_accessors_and_proxies() {
    assert_eq!(
        eval(
            "var log = [];
             var target = { get g() { log.push('get:' + (this === p)); return 1; } };
             var p = new Proxy(target, {
                 get(t, key, receiver) { log.push('trap:' + String(key)); return Reflect.get(t, key, receiver); },
                 set(t, key, value, receiver) { log.push('set:' + String(key)); return Reflect.set(t, key, value, receiver); }
             });
             p.g; p.v = 2;
             log.join();"
        ),
        Ok(string("trap:g,get:true,set:v"))
    );
}

#[test]
fn enumeration_order_survives_named_insertion_and_reinsertion() {
    assert_eq!(
        eval(
            "var o = {}; o.b = 1; o[2] = 1; o.a = 1; o[1] = 1; delete o.b; o.b = 2;
             Object.keys(o).join();"
        ),
        Ok(string("1,2,a,b"))
    );
}

#[test]
fn objects_cross_eval_function_and_module_code_by_key() {
    assert_eq!(
        eval(
            "var shared = { a: 1 };
             var viaFunction = new Function('o', 'o.b = o.a + 1; return o.b;');
             var viaEval = eval('shared.c = viaFunction(shared) + 1; shared.c');
             [shared.a, shared.b, shared.c, viaEval].join();"
        ),
        Ok(string("1,2,3,3"))
    );
}
