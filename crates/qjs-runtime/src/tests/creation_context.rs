//! Functions retain one creation authority; globals remain live property routes.
use crate::{Value, eval};

#[test]
fn dynamic_functions_retain_live_selected_globals_without_overwriting_the_origin() {
    assert_eq!(
        eval(
            r#"var value = 1; const origin = () => value; const other = {value: 2};
        globalThis.__quickjsRustDynamicFunctionRealm = other;
        const read = Function('return value'); const write = Function('v', 'value = v; return value'); const remove = Function('return delete value');
        delete globalThis.__quickjsRustDynamicFunctionRealm;
        let got = read(); other.value = 3; got += ':' + read();
        let calls = 0, seen;
        Object.defineProperty(other, 'value', {get() { calls++; return 7; }, set(v) { seen = v; }, configurable: true});
        got += ':' + write(4) + ':' + calls + ':' + seen + ':' + globalThis.value;
        got += ':' + remove() + ':' + origin();
        got;"#
        ),
        Ok(Value::String("2:3:7:1:4:1:true:1".into()))
    );
}

#[test]
fn primitive_this_boxing_uses_intrinsics_after_public_constructor_rebinding() {
    assert_eq!(
        eval(
            r#"const p = [Boolean.prototype, Number.prototype, String.prototype, BigInt.prototype, Symbol.prototype];
        const values = [true, 1, 'x', 2n, Symbol()];
        function f() { return Object.getPrototypeOf(this); }
        Boolean = Number = String = BigInt = Symbol = function Replacement() {};
        values.map((value, i) => f.call(value) === p[i]).join(':');"#
        ),
        Ok(Value::String("true:true:true:true:true".into()))
    );
}

#[test]
fn nested_heritage_and_keys_retain_distinct_private_creation_contexts() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/s3b-scope-class-heritage-private-parent.js"
        )),
        Ok(Value::String("true:false:false:true".into()))
    );
}

#[test]
fn dynamic_function_nested_source_text_retains_the_parsed_source() {
    assert_eq!(
        eval("Function('return function inner(value) { return value + 1; };')().toString();"),
        Ok(Value::String(
            "function inner(value) { return value + 1; }".into()
        ))
    );
}
