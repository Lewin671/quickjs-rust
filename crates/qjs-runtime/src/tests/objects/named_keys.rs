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

#[test]
fn surrogate_pairs_reach_literal_constructor_array_and_function_stores() {
    assert_eq!(
        eval(
            "function C() { this['\\u{1F600}'] = 1; }
         var receivers = [{ '\\u{1F600}': 1, y: 2 }, new C(), [], function() {}];
         var pair = '\\uD83D' + '\\uDE00';
         var out = [];
         for (var i = 0; i < receivers.length; i++) {
             var o = receivers[i]; o['\\u{1F600}'] = 3;
             out.push(o[pair]); o[pair] = 4; out.push(o['\\u{1F600}']);
             delete o[pair]; out.push('\\u{1F600}' in o);
         }
         out.join();"
        ),
        Ok(string("3,4,false,3,4,false,3,4,false,3,4,false"))
    );
}
#[test]
fn layouts_preserve_reentry_descriptors_and_exotic_named_parts() {
    // The same source is also a QuickJS-NG fixture. Its assertions cover
    // construction, lazy properties, getters/Proxy re-entry and exotics.
    let source =
        include_str!("../../../../../tests/fixtures/compare-qjs/property-layout-mutations.js")
            .replace("print('layout mutations ok');", "'layout mutations ok';");
    assert_eq!(eval(&source), Ok(string("layout mutations ok")));
}

#[test]
fn array_named_properties_keep_string_insertion_order() {
    assert_eq!(
        eval(
            "var a = []; a.b = 1; a.a = 2;
        var first = Object.keys(a).join(); delete a.b; a.b = 3;
        first + '|' + Object.keys(a).join() + '|' + Object.getOwnPropertyNames(a).join();"
        ),
        Ok(string("b,a|a,b|length,a,b"))
    );
}
