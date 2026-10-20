//! Ordinary property semantics across layout transitions: edges, attribute
//! edits, dictionaries, payload re-encodings, literal sites and realms.

use crate::{Value, eval};

#[test]
fn layout_semantics_survive_edges_dictionaries_and_reencoding() {
    let source = r#"
        var out = [];
        function keys(o) { return Object.keys(o).join(','); }
        // Delete and re-add moves the key to the end.
        var a = { x: 1, y: 2, z: 3 }; delete a.y; a.y = 4; out.push(keys(a));
        // Integer keys enumerate first, ascending.
        var b = { b: 1, 2: 1, a: 1, 1: 1 }; out.push(keys(b));
        // Equal keys with different attributes take different edges.
        var c1 = {}; c1.k = 1;
        var c2 = {}; Object.defineProperty(c2, 'k', { value: 1, enumerable: false, writable: true, configurable: true });
        var c3 = {}; c3.k = 2;
        out.push(keys(c1) + '|' + keys(c2) + '|' + keys(c3) + '|' + c2.k);
        // More than 32 edits and more than 128 properties.
        var d = {};
        for (var i = 0; i < 40; i++) { d['p' + i] = i; delete d['p' + i]; d['p' + i] = i * 2; }
        out.push(Object.keys(d).length + ':' + d.p39);
        var e = {};
        for (var i = 0; i < 150; i++) e['q' + i] = i;
        delete e.q0; e.q0 = 'last';
        var ek = Object.keys(e);
        out.push(ek.length + ':' + ek[0] + ':' + ek[149] + ':' + e.q75);
        // Freezing one literal instance leaves its siblings writable.
        function lit() { return { m: 1, n: 2 }; }
        var f1 = Object.freeze(lit()); var f2 = lit(); f2.m = 9; f1.m = 9;
        out.push(f1.m + ':' + f2.m + ':' + Object.isFrozen(f1) + ':' + Object.isFrozen(f2));
        // Class-field initializers that delete and observe the instance.
        class G { a = 1; b = (delete this.a, Object.keys(this).join('+') + '!'); c = 3; }
        var g = new G(); out.push(keys(g) + '=' + g.b);
        // A base constructor returning another object.
        class H { constructor() { return { other: true }; } }
        class K extends H { f = 1; }
        var k = new K(); out.push(keys(k));
        // Null-prototype objects and accessors.
        var n = Object.create(null); n.a = 1;
        Object.defineProperty(n, 'g', { get: function () { return this.a + 1; }, enumerable: true, configurable: true });
        out.push(keys(n) + ':' + n.g);
        // Lone-surrogate and halves-built keys name the same property.
        var s = {}; s['\uD800'] = 1; s['\uD83D' + '\uDE00'] = 2;
        out.push(s['\uD800'] + ':' + s['😀'] + ':' + Object.keys(s).length);
        // Literals across eval keep independent instances.
        var ev = eval('({ p: 1, q: 2 })'); ev.r = 3; out.push(keys(ev) + ':' + keys(eval('({ p: 1, q: 2 })')));
        out.join(' ; ');
    "#;
    let value = eval(source).expect("semantics script");
    let expected = "x,z,y ; 1,2,b,a ; k||k|1 ; 40:78 ; 150:q1:q0:75 ; 1:9:true:false ; \
                    b,c=! ; other,f ; a,g:2 ; 1:2:2 ; p,q,r:p,q";
    assert_eq!(value, Value::String(expected.into()));
}

/// Reserve the original default slots even when an installer replaces a
/// lazy prototype after installing statics. Later statics retain their order.
#[test]
fn builtin_constructor_keys_keep_prototype_before_statics() {
    let expected = [
        ("Array", "length,name,prototype,from,fromAsync,isArray,of"),
        (
            "Object",
            "length,name,prototype,assign,create,defineProperty,defineProperties,getPrototypeOf,getOwnPropertyDescriptor,getOwnPropertyDescriptors,getOwnPropertyNames,getOwnPropertySymbols,fromEntries,groupBy,freeze,hasOwn,is,isExtensible,isFrozen,isSealed,preventExtensions,seal,setPrototypeOf,entries,keys,values",
        ),
        (
            "Promise",
            "length,name,prototype,all,any,allSettled,race,try,withResolvers,resolve,reject",
        ),
        (
            "Number",
            "length,name,prototype,EPSILON,MAX_SAFE_INTEGER,MAX_VALUE,MIN_SAFE_INTEGER,MIN_VALUE,NaN,NEGATIVE_INFINITY,POSITIVE_INFINITY,isFinite,isInteger,isNaN,isSafeInteger,parseFloat,parseInt",
        ),
        (
            "String",
            "length,name,prototype,fromCharCode,fromCodePoint,raw",
        ),
        (
            "Symbol",
            "length,name,prototype,asyncDispose,asyncIterator,dispose,hasInstance,isConcatSpreadable,iterator,match,matchAll,replace,search,species,split,toPrimitive,toStringTag,unscopables,for,keyFor",
        ),
        ("Date", "length,name,prototype,now,parse,UTC"),
        ("RegExp", "length,name,prototype,escape"),
        ("Map", "length,name,prototype,groupBy"),
        ("Set", "length,name,prototype"),
        ("BigInt", "length,name,prototype,asIntN,asUintN"),
        ("Error", "length,name,prototype,isError"),
        ("ArrayBuffer", "length,name,prototype,isView"),
        (
            "Uint8Array",
            "length,name,prototype,fromBase64,fromHex,BYTES_PER_ELEMENT",
        ),
        ("Proxy", "length,name,revocable"),
    ];
    for (name, keys) in expected {
        let source = format!("Object.getOwnPropertyNames({name}).join(',')");
        assert_eq!(
            eval(&source),
            Ok(Value::String(keys.to_owned().into())),
            "{name}"
        );
    }
}

#[test]
fn builtin_constructor_default_materialization_preserves_key_order() {
    let constructors = [
        "Error",
        "TypeError",
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "URIError",
        "AggregateError",
        "SuppressedError",
        "Array",
        "Object",
        "RegExp",
        "Map",
        "BigInt",
        "Symbol",
        "Uint8Array",
    ];
    for constructor in constructors {
        for operation in [
            "Object.freeze(C)",
            "Object.seal(C)",
            "Object.defineProperty(C, 'name', {value: 'renamed'})",
            "Object.defineProperty(C, 'length', {value: 7})",
        ] {
            let source = format!(
                "var C = {constructor}; var before = Object.getOwnPropertyNames(C).join(',');
                 {operation}; before === Object.getOwnPropertyNames(C).join(',');"
            );
            assert_eq!(
                eval(&source),
                Ok(Value::boolean(true)),
                "{constructor}: {operation}"
            );
        }
    }
}

#[test]
fn directly_installed_native_defaults_keep_descriptors_and_leading_slots() {
    use crate::{Function, NativeFunction, Property, property::atom::AtomDomain};

    let domain = AtomDomain::new();
    let _roots = domain.roots();
    for operation in ["freeze", "seal", "define"] {
        let function =
            Function::new_native(&domain, Some("original"), 0, NativeFunction::Error, true);
        // Model installers that replace defaults through the named table,
        // including defaults whose value differs from the generic builder.
        for (key, value) in [
            ("static", Value::Number(1.0)),
            ("prototype", Value::Null),
            ("name", Value::String("installed".into())),
            ("length", Value::Number(9.0)),
        ] {
            function.properties.borrow_mut().insert_in(
                &domain,
                key,
                Property::data(value, false, false, true),
            );
        }
        let before = function.own_property_names();
        assert_eq!(before, ["length", "name", "prototype", "static"]);
        match operation {
            "freeze" => function.freeze(),
            "seal" => function.seal(),
            _ => {
                function.define_property("name".to_owned(), function.own_property("name").unwrap())
            }
        }
        assert_eq!(function.own_property_names(), before, "{operation}");
        assert_eq!(
            function.own_property("length").unwrap().value,
            Value::Number(9.0)
        );
        assert_eq!(
            function.own_property("name").unwrap().value,
            Value::String("installed".into())
        );
        assert!(!function.own_property("length").unwrap().enumerable);
        assert!(!function.own_property("name").unwrap().writable);
    }
}
