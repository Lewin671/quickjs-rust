//! Class keys execute in their enclosing activation, including suspension.
use crate::{Value, eval};

#[test]
fn private_key_yield_resumes_the_enclosing_generator() {
    assert_eq!(
        eval(
            r#"function* g() {
        class C { #p; [(#p in {}, yield 'key')]() {} }
        return typeof new C().method;
    } const it = g(); it.next().value + ':' + it.next('method').value;"#
        ),
        Ok(Value::String("key:function".into()))
    );
}

#[test]
fn private_key_await_resumes_the_enclosing_async_function() {
    let value = eval(
        r#"let result = []; async function f() {
        class C { #p; [(#p in {}, await Promise.resolve('method'))]() {} }
        return typeof new C().method;
    } f().then(value => result.push(value), error => result.push(error.name)); result;"#,
    )
    .unwrap();
    let Value::Array(result) = value else {
        panic!("expected result array");
    };
    assert_eq!(result.to_vec(), vec![Value::String("function".into())]);
}

#[test]
fn class_name_is_uninitialized_during_keys_then_shared_by_members() {
    assert_eq!(
        eval(
            r#"let read, failed;
        const Outer = class Inner {
            [(read = () => Inner, (() => { try { Inner; } catch (e) {
                failed = e instanceof ReferenceError;
            } })(), 'method')]() { return Inner; }
            static value = read();
        }; [failed, read() === Outer, new Outer().method() === Outer,
            Outer.value === Outer].join(':');"#
        ),
        Ok(Value::String("true:true:true:true".into()))
    );
}

#[test]
fn anonymous_class_display_name_does_not_create_a_binding() {
    assert_eq!(
        eval(
            r#"let C = class { method() { return C; } }; const first = C;
        C = 8; first.name + ':' + new first().method();"#
        ),
        Ok(Value::String("C:8".into()))
    );
}

#[test]
fn failed_heritage_precedes_computed_key_effects() {
    assert_eq!(
        eval(
            r#"let effects = 0; try {
        class C extends 1 { [(effects++, 'method')]() {} }
    } catch (e) {} effects;"#
        ),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn failed_element_definition_precedes_later_key_effects() {
    assert_eq!(
        eval(
            r#"let effects = 0; try {
        class C { static ['prototype']() {} [(effects++, 'method')]() {} }
    } catch (e) {} effects;"#
        ),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn static_public_private_fields_and_blocks_keep_source_order() {
    assert_eq!(
        eval(
            r#"let effects = ''; class C {
        static a = (effects += 'a'); static #b = (effects += 'b');
        static { effects += 'c'; } static #d = (effects += 'd');
        static e = (effects += 'e');
    } effects;"#
        ),
        Ok(Value::String("abcde".into()))
    );
}

#[test]
fn catch_cleanup_preserves_the_enclosing_arguments_cell() {
    assert_eq!(
        eval(
            r#"let read; function f(value) {
        try { read = () => arguments[0]; class C { #p; [(#p in {}, 'm')]() {} } throw 0; }
        catch (e) {} return arguments[0];
    } f('m') + ':' + read();"#
        ),
        Ok(Value::String("m:m".into()))
    );
}

#[test]
fn field_and_static_restrictions_follow_arrows_but_stop_at_ordinary_functions() {
    assert_eq!(
        eval(
            r#"let results = []; class C {
        value = (() => { try { eval('arguments'); } catch(e) { results.push(e instanceof SyntaxError); }
            return function () { return eval('typeof arguments'); }; })();
        static { const read = () => { try { eval('arguments'); } catch(e) { return e instanceof SyntaxError; } };
            results.push(read());
            results.push((function () { return eval('typeof arguments'); })());
            results.push(eval('new.target') === undefined);
        }
    } results.push(new C().value()); results.join(':');"#
        ),
        Ok(Value::String("true:object:true:true:object".into()))
    );
}

#[test]
fn public_property_named_this_cannot_replace_the_script_receiver() {
    assert_eq!(
        eval(
            r#"globalThis.this = 8;
        const read = () => this; (0, eval)('this === globalThis') + ':' + (read() === globalThis);"#
        ),
        Ok(Value::String("true:true".into()))
    );
}
