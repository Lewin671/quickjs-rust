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
