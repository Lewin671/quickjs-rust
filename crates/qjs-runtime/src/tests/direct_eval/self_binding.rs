//! Self names have an immutable creation-time cell outside parameters/body.
use crate::{Value, eval};

#[test]
fn named_call_does_not_restore_a_caller_homonym_after_callback_writes() {
    assert_eq!(
        eval(
            r#"let f = 'before'; const run = function f(callback) {
        callback(); return typeof f;
    }; run(() => f = 'after') + ':' + f;"#
        ),
        Ok(Value::String("function:after".into()))
    );
}

#[test]
fn self_assignment_ignores_sloppy_writes_and_rejects_strict_writes() {
    assert_eq!(
        eval(
            r#"const f = function self() {
        self = 8; eval('self = 9'); let threw = false;
        try { eval('\"use strict\"; self = 10'); } catch(e) { threw = e instanceof TypeError; }
        return typeof self + ':' + threw;
    }; f();"#
        ),
        Ok(Value::String("function:true".into()))
    );
}

#[test]
fn class_key_assignment_uses_strict_policy_of_its_enclosing_region() {
    assert_eq!(
        eval(
            r#"(function self() {
        try { class C { [(self = 8)]() {} } } catch(e) { return e instanceof TypeError; }
    })();"#
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn self_cell_survives_a_shadowing_body_eval_declaration() {
    assert_eq!(
        eval(
            r#"const f = function self() {
        const read = () => self; eval('var self = 8'); return read() + ':' + self;
    }; f();"#
        ),
        Ok(Value::String("8:8".into()))
    );
}

#[test]
fn dynamic_function_display_name_does_not_bind_anonymous() {
    assert_eq!(
        eval("Function('return typeof anonymous')();"),
        Ok(Value::String("undefined".into()))
    );
}
