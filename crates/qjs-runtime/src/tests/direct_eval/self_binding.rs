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

#[test]
fn self_scope_keeps_positional_free_bindings_ahead_of_global_homonyms() {
    for prefix in ["", "'use strict';"] {
        for declaration in ["var", "let"] {
            let source = format!(
                "var x='global',count=0;function outer(x){{{declaration} count=0; \
                var inc=function inc(){{{prefix}inc;return ++count;}}; \
                var read=function nf(){{{prefix}nf;return x;}}; \
                var gen=function* gn(){{gn;yield x;yield ++count;}}; \
                inc();inc();var it=gen();return [read(),[0].map(function cb(){{cb;return x;}})[0], \
                it.next().value,it.next().value,count].join(':');}}outer('param')+':'+x+':'+count;"
            );
            assert_eq!(
                eval(&source),
                Ok(Value::String("param:param:param:3:3:global:0".into())),
                "{source}"
            );
        }
    }
}

#[test]
fn invalidated_global_capture_resolves_the_current_object_record() {
    assert_eq!(
        eval("y=1;var f=function nf(){nf;return y;};delete y;var r;try{f();}catch(e){r=e.name;}r;"),
        Ok(Value::String("ReferenceError".into()))
    );
    assert_eq!(
        eval(
            "y=1;var f=function nf(){nf;return y;};delete y;y=3;var a=f();delete y;Object.defineProperty(globalThis,'y',{configurable:true,get(){return 7;}});a+':'+f();"
        ),
        Ok(Value::String("3:7".into()))
    );
    for (mutation, expected) in [
        ("clean();", "ReferenceError"),
        ("clean();y=9;", "ok:9"),
        (
            "Object.defineProperty(globalThis,'y',{configurable:true,get(){return 7;}});",
            "ok:7",
        ),
    ] {
        let source = format!(
            "y=undefined;var clean=()=>eval('delete y');var p=(async function(){{await 0;return y=c?y:1;}})().then(v=>'ok:'+v,e=>e.name);c=1;{mutation}p;"
        );
        let value = eval(&source).unwrap();
        assert_eq!(
            crate::promise::promise_debug_state_result(&value),
            Some(("fulfilled".into(), Value::String(expected.into()))),
            "{source}"
        );
    }
}
