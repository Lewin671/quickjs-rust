//! Inline entry and VM continuation must observe the same canonical call owner.
use super::*;

fn assert_owner_survives_exit(construct: bool, source: &str) {
    let mut callee = crate::eval(source).unwrap();
    let Value::Function(function) = callee.clone() else {
        panic!("function")
    };
    let bytecode = function.bytecode.as_ref().unwrap().clone();
    let mut root = crate::context::HostRoot::new(function.creation_realm().clone());
    let mut env = root.context();
    let caller_this = env.caller().this_value().unwrap();
    let receiver = Value::Object(crate::ObjectRef::new(
        std::collections::HashMap::new(),
        function.creation_realm().atoms(),
    ));
    let mut supplied = Some(receiver.clone());
    let mut registers = Vec::new();
    let mut frames = Vec::new();
    let mut current = Value::Undefined;
    let (mut base, mut len, mut pc) = (0, 0, 0);
    if construct {
        assert!(
            enter_constructor(
                callee.clone(),
                &bytecode,
                &mut env,
                &mut registers,
                &mut frames,
                &mut current,
                &mut base,
                &mut len,
                0,
                0,
                0,
                0,
            )
            .unwrap()
            .is_none()
        );
        assert!(
            env.caller()
                .lexical_new_target()
                .unwrap()
                .same_value(&callee)
        );
    } else {
        assert!(
            entry::try_enter_call(
                &mut callee,
                &mut supplied,
                &bytecode,
                &mut env,
                &mut registers,
                &mut frames,
                &mut current,
                &mut base,
                &mut len,
                &mut pc,
                0,
                0,
                0,
                0,
            )
            .unwrap()
        );
        assert!(env.caller().lexical_new_target().is_none());
        assert!(env.caller().this_value().unwrap().same_value(&receiver));
    }
    let actual_receiver = env.caller().this_value().unwrap();
    assert!(matches!(actual_receiver, Value::Object(_)));
    match (function.home_object(), env.caller().home_object()) {
        (None, None) => {}
        (Some(expected), Some(actual)) => assert!(actual.same_value(&expected)),
        _ => panic!("home object must come from the callee"),
    }
    assert!(
        env.caller()
            .this_value()
            .unwrap()
            .same_value(&actual_receiver)
    );
    assert!(
        frames[0]
            .activation
            .this_value()
            .unwrap()
            .same_value(&caller_this)
    );
    let ip = bytecode
        .code
        .iter()
        .position(|op| matches!(op, crate::bytecode::ir::Op::LoadGlobal(name) if name == "this"))
        .unwrap();
    let result = exit_to_interpreter(
        &current,
        &bytecode,
        RootExit {
            upvalues: crate::bytecode::DirectCallUpvalues::Function(&function),
            realm_upvalue_slots: 0,
        },
        ip as u32,
        0,
        0,
        &mut registers[base..base + len],
        &mut env,
    );
    let ExitOutcome::Finished(Ok(value)) = result else {
        panic!("continuation must finish")
    };
    assert!(value.same_value(&actual_receiver));
}

#[test]
fn inline_call_owner_and_vm_exit_keep_the_same_receiver() {
    for source in [
        "(function () { return this; })",
        "({ m() { return this; } }).m",
    ] {
        assert_owner_survives_exit(false, source);
    }
}

#[test]
fn inline_constructor_owner_keeps_receiver_and_new_target() {
    assert_owner_survives_exit(true, "(function () { return this; })");
}

#[test]
fn implicit_bindings_keep_their_general_entry_when_wide_cannot_represent_them() {
    for source in [
        "(function () { return arguments[0]; })",
        "(function () { return new.target; })",
        "({ __proto__: { value: 7 }, m() { return super.value; } }).m",
        "(function self() { return self; })",
        "(() => this)",
    ] {
        let callee = crate::eval(source).unwrap();
        assert!(inline_entry(&callee).is_none(), "{source}");
    }
}

#[test]
fn equality_deoptimization_preserves_receivers_across_loop_and_call_forms() {
    for comparison in ["== 0", "=== 0", "!= 1", "!== 1"] {
        for (before, after) in [
            ("for (var i = 0; i < 3; i++) {", "}"),
            ("var i = 0; while (i++ < 3) {", "}"),
            ("var i = 0; do {", "} while (++i < 3);"),
        ] {
            let body = format!(
                "var r=0; {before} if(this.w {comparison}) r+=this['v']; {after} return r;"
            );
            for definition in [
                format!("function p(){{{body}}} var o={{w:0,v:2,p:p}};"),
                format!("var o={{w:0,v:2,p(){{{body}}}}};"),
                format!("class C{{p(){{{body}}}}} var o=new C();o.w=0;o.v=2;"),
            ] {
                for caller in [
                    "function calc(){return o.p();} calc();",
                    "var calc=()=>o.p();calc();",
                    "[0].map(function(){return o.p();})[0];",
                ] {
                    let source = format!("{definition}{caller}");
                    assert_eq!(crate::eval(&source), Ok(Value::Number(6.0)), "{source}");
                }
            }
        }
    }
}

#[test]
fn native_callback_standalone_entry_keeps_the_canonical_receiver() {
    let source = "function p(){var r=0;for(var i=0;i<3;i++){if(this.w==0)r+=this.v;}return r;} \
        var o={w:0,v:2}; \
        [p.call(o),p.apply(o,[]),Reflect.apply(p,o,[]),p.bind(o)(), \
         [0].map(p,o)[0],Array.from([0],p,o)[0]].join(':');";
    assert_eq!(crate::eval(source), Ok(Value::String("6:6:6:6:6:6".into())));
}

#[test]
fn compound_member_assignments_after_deoptimization_keep_this() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/compare-qjs/s3b-inline-receiver-deoptimization.js"
    ));
    assert_eq!(
        crate::eval(source),
        Ok(Value::String("[[1,1,0],[1,1,0]]".into()))
    );
}
