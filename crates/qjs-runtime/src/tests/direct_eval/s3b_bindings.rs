//! Binding metadata must survive dynamic resolution and mapped arguments.

use crate::{Value, eval};

#[test]
fn dynamic_assignments_preserve_const_and_mapped_parameter_authority() {
    assert_eq!(
        eval(include_str!(
            "../../../../../tests/fixtures/compare-qjs/s3b-scope-binding-policies.js"
        )),
        Ok(Value::String("TypeError:3|2:4|9:8:9".into())),
    );
}

#[test]
fn global_lexical_cells_are_independent_of_same_named_properties() {
    assert_eq!(
        eval(
            "let s3bLexical = 1; globalThis.s3bLexical = 7; \
         Object.defineProperty(globalThis, 's3bLexical', {get() {return 9;}, configurable: true}); \
         var first = s3bLexical; s3bLexical = 2; delete globalThis.s3bLexical; \
         first + ':' + s3bLexical + ':' + (0, eval)('s3bLexical');"
        ),
        Ok(Value::String("1:2:2".into()))
    );
}

#[test]
fn leases_preserve_live_declarations_tdz_arguments_and_parameter_boundaries() {
    assert_eq!(
        eval(include_str!("../../../../../tests/fixtures/compare-qjs/s3b-scope-live-leases.js")),
        Ok(Value::String("8|5|TypeError:3|ReferenceError:3|2:1|2:1|true:undefined|false:2|2:4|1:2|ReferenceError:3|undefined|object|2:1|2:1".into())),
    );
}

#[test]
fn missing_with_property_does_not_repeat_has_binding_on_fallback() {
    assert_eq!(
        eval(
            r#"let log = []; const p = new Proxy({}, {
        has(target, key) { log.push(String(key)); return false; },
        get() { throw new Error('unexpected Get'); }
    }); with (p) { Object; } log.join(':');"#
        ),
        Ok(Value::String("Object".into()))
    );
}
