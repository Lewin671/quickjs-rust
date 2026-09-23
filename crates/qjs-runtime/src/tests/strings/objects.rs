use crate::{Value, eval};

#[test]
fn evaluates_string_objects() {
    assert_eq!(
        eval("typeof new String('abc');"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.constructor === String;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.valueOf();"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.toString();"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(eval("new String('abc').length;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let s = new String('abc'); s[1];"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); try { s.length = 1; } catch (error) {} s.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let s = new String('abc'); s == 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s !== 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new String('abc'));"),
        Ok(Value::String("[object String]".to_owned().into()))
    );
    assert_eq!(
        eval("new String('abc').charAt(2);"),
        Ok(Value::String("c".to_owned().into()))
    );
}

#[test]
fn string_prototype_is_empty_string_object() {
    assert_eq!(
        eval(
            "[
                String.prototype == '',
                String.prototype.valueOf(),
                String.prototype.length,
                Object.prototype.isPrototypeOf(String.prototype),
                (delete String.prototype.toString,
                 Object.prototype.toString.call(String.prototype))
             ].join('|');"
        ),
        Ok(Value::String(
            "true||0|true|[object String]".to_owned().into()
        ))
    );
}

#[test]
fn boxed_character_values_preserve_all_latin1_and_wide_code_units() {
    assert_eq!(
        eval(
            "function sloppyBox() { return this; }
             let units = [];
             for (let n = 0; n < 256; n++) units.push(n);
             units.push(256, 0xd800, 0xdfff, 0xffff);
             for (let n of units) {
                 let text = String.fromCharCode(n);
                 let boxes = [new String(text), Object(text), sloppyBox.call(text)];
                 for (let box of boxes) {
                     let descriptor = Object.getOwnPropertyDescriptor(box, '0');
                     if (box.length !== 1 || box[0] !== text ||
                         descriptor.value !== text || !descriptor.enumerable ||
                         descriptor.writable || descriptor.configurable) throw n;
                 }
             }
             true;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn boxed_character_sharing_preserves_copy_on_write_subclasses_and_proxy_invariants() {
    assert_eq!(
        eval(
            "class Text extends String {}
             let original = 'aaé\u{1f680}';
             let first = new Text(original);
             let second = Object(original);
             let changed = first[0];
             changed += 'x';
             let proxy = new Proxy(first, {});
             let same = Object.defineProperty(first, '0', {value: 'a'}) === first;
             let rejected = false;
             try { Object.defineProperty(first, '0', {value: 'z'}); }
             catch (error) { rejected = error instanceof TypeError; }
             [first instanceof Text, first[0], second[0], changed,
              first[3].charCodeAt(0), first[4].charCodeAt(0),
              Object.keys(first).join(','), Reflect.set(proxy, '0', 'z'),
              Reflect.deleteProperty(proxy, '0'),
              Reflect.getOwnPropertyDescriptor(proxy, '0').writable,
              same, rejected, first.valueOf() === original].join('|');"
        ),
        Ok(Value::String(
            "true|a|a|ax|55357|56960|0,1,2,3,4|false|false|false|true|true|true"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn string_objects_install_index_properties_with_shared_and_fresh_keys() {
    // Short wrappers take realm-shared keys; indices past the shared range
    // get their own. Attributes, order and values must not differ.
    assert_eq!(
        eval(
            "var long = '';
             for (var i = 0; i < 70; i++) long += String.fromCharCode(97 + i % 26);
             var wrapped = new String(long), boxed = Object('\\uD83D\\uDE00x');
             var d = Object.getOwnPropertyDescriptor(wrapped, '65');
             [Object.keys(new String(7)).join(), wrapped[3] + wrapped[65], wrapped.length,
              d.value + d.writable + d.enumerable + d.configurable,
              Object.getOwnPropertyNames(boxed).join(), boxed[0].length, boxed.length,
              Object.getOwnPropertyDescriptor(new String('ab'), 'length').writable,
              Object.keys(new String('')).length].join('|');"
        ),
        Ok(Value::String(
            "0|dn|70|nfalsetruefalse|0,1,2,length|1|3|false|0"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn typeof_name_literals_compare_by_identity_and_by_text() {
    // `typeof` results and literals naming them share one instance; a string
    // built with the same text, or an appended copy, still compares equal.
    assert_eq!(
        eval(
            "function f(x) { return typeof x === 'number'; }
             var s = 'num'; s += 'ber';
             var t = 'number'; t += '';
             [f(1), f('1'), s === 'number', t === 'number', typeof s === 'string',
              'number' === 'object', ('obj' + 'ect') == typeof null,
              ['number', 'object'].indexOf(typeof {})].join(',');"
        ),
        Ok(crate::Value::String(
            "true,false,true,true,true,false,true,1".to_owned().into()
        ))
    );
}

#[test]
fn string_wrapper_index_properties_are_defined_when_observed() {
    // A wrapper defines its index properties only when its property table is
    // read by index or as a whole; every observation below must match a
    // wrapper that defined them eagerly.
    assert_eq!(
        eval(
            "var out = []; \
             var s = new String(\"ab\"); \
             s.foo = 1; \
             out.push(Object.getOwnPropertyNames(s).join()); \
             out.push(JSON.stringify(Object.getOwnPropertyDescriptor(s, \"1\"))); \
             out.push(s[1], \"0\" in s, s.hasOwnProperty(\"1\"), s.hasOwnProperty(\"2\"), delete s[0], s[0]); \
             try { Object.defineProperty(s, \"0\", { value: \"z\" }); out.push(\"no throw\"); } catch (e) { out.push(e instanceof TypeError); } \
             out.push(Object.keys(s).join()); \
             var keys = []; for (var k in s) keys.push(k); out.push(keys.join()); \
             s[1] = \"q\"; out.push(s[1]); \
             out.push(Object.isFrozen(Object.freeze(new String(\"xy\")))); \
             String.prototype.probe = function () { return this.length + \":\" + this[1] + \":\" + typeof this + \":\" + Object.keys(this).join(); }; \
             out.push(\"cd\".probe()); \
             String.prototype.plain = function () { return this.replace(\"c\", \"C\") + this.toUpperCase(); }; \
             out.push(\"cd\".plain()); \
             var t = new String(\"hi\"); t[5] = \"x\"; out.push(Object.getOwnPropertyNames(t).join(), t.length); \
             out.push(Object.entries(new String(\"ok\")).join(\"|\")); \
             out.push(Reflect.ownKeys(new String(\"12\")).join()); \
             out.join(\" \");"
        ),
        Ok(Value::String(
            "0,1,length,foo {\"value\":\"b\",\"writable\":false,\"enumerable\":true,\"configurable\":false} b true true false false a true 0,1,foo 0,1,foo b true 2:d:object:0,1 CdCD 0,1,5,length 2 0,o|1,k 0,1,length"
                .to_owned()
                .into()
        ))
    );
}
