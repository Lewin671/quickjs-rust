//! Native argument ownership, callback re-entry and invocation metadata.

use std::collections::HashMap;

use super::*;
use crate::function::{call_function_slice, construct_function_slice, new_realm};

fn empty_root() -> crate::context::HostRoot {
    crate::context::HostRoot::new(new_realm(HashMap::new()))
}

#[test]
fn retained_native_arguments_outlive_the_host_slice() {
    let mut root = empty_root();
    let mut env = root.context();
    let object = crate::ObjectRef::new(HashMap::new(), env.realm().atoms());
    let native = Value::Function(Function::new_native(
        None,
        1,
        NativeFunction::Array,
        true,
        env.realm(),
    ));
    let result = {
        let arguments = [Value::Object(object.clone()), Value::Boolean(true)];
        call_function_slice(&native, Value::Undefined, &arguments, &mut env).unwrap()
    };
    let Value::Array(array) = result else {
        panic!("Array must return an array");
    };
    assert!(array.get(0).unwrap().same_value(&Value::Object(object)));
    assert_eq!(array.get(1), Some(Value::Boolean(true)));
}

#[test]
fn native_bound_arguments_retain_values_after_the_host_list_drops() {
    let mut root = empty_root();
    let mut env = root.context();
    let bind = Value::Function(Function::new_native(
        None,
        1,
        NativeFunction::FunctionPrototypeBind,
        false,
        env.realm(),
    ));
    let max = Value::Function(Function::new_native(
        None,
        2,
        NativeFunction::MathMax,
        false,
        env.realm(),
    ));
    let bound = {
        let arguments = [Value::Undefined, Value::Number(17.0)];
        call_function_slice(&bind, max, &arguments, &mut env).unwrap()
    };
    assert_eq!(
        call_function_slice(&bound, Value::Undefined, &[Value::Number(4.0)], &mut env).unwrap(),
        Value::Number(17.0)
    );
}

#[test]
fn nested_callbacks_leave_the_callers_operands_and_arguments_live() {
    assert_eq!(
        crate::eval(
            r#"
        function recur(n) {
            if (n === 0) return 1;
            return 10 + [n, n + 1].reduce(function(total, value) {
                return total + value + recur(n - 1);
            }, 0);
        }
        var mapped = Array.from([7, 9], Number);
        recur(3) === 107 && mapped[0] === 7 && mapped[1] === 9;
    "#
        )
        .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn callback_throw_restores_the_stack_before_catch_and_finally() {
    assert_eq!(
        crate::eval(
            r#"
        var token = {}, seen = 0, finished = false;
        function fail() {
            return 100 + [1, 2, 3].map(function(value) {
                seen += value;
                if (value === 2) throw token;
                return value;
            }).length;
        }
        try { fail(); } catch (e) { if (e !== token) throw 0; }
        finally { finished = true; }
        seen === 3 && finished && 20 + Math.max(3, 7) === 27;
    "#
        )
        .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn retained_async_reaction_arguments_survive_native_reentry() {
    let promise = crate::eval(
        r#"
        var object = { value: 13 };
        Promise.resolve(object).then(function(value) {
            return [value].map(function(item) { return item.value; })[0];
        });
    "#,
    )
    .unwrap();
    assert_eq!(
        crate::promise::promise_debug_state_result(&promise),
        Some(("fulfilled".to_owned(), Value::Number(13.0)))
    );
}

#[test]
fn native_constructor_slice_preserves_a_cross_realm_new_target() {
    let mut root = empty_root();
    let mut env = root.context();
    let mut foreign_root = empty_root();
    let mut foreign_env = foreign_root.context();
    let foreign_global = Value::Object(crate::ObjectRef::new(
        HashMap::new(),
        foreign_env.realm().atoms(),
    ));
    crate::builtins::initialize_builtins(&mut foreign_env, &foreign_global);
    let target = foreign_env.get("Boolean").unwrap();
    let new_target = foreign_env.get("Object").unwrap();
    let previous = Value::Function(Function::new_native(
        None,
        0,
        NativeFunction::Array,
        true,
        env.realm(),
    ));
    let mut owner =
        crate::context::CallOwner::root(Value::Undefined, crate::context::OwnerKind::Function);
    owner.new_target = Some(previous.clone());
    env.set_call_owner(owner);
    let result = construct_function_slice(
        target,
        new_target.clone(),
        &[Value::Boolean(true)],
        &mut env,
    )
    .unwrap();
    let prototype = crate::property_value(new_target, "prototype", &mut env).unwrap();
    let actual = crate::object::native_object_get_prototype_of(&[result], &mut env).unwrap();
    assert!(actual.same_value(&prototype));
    assert!(
        env.caller()
            .lexical_new_target()
            .unwrap()
            .same_value(&previous)
    );
}

#[test]
fn new_target_is_restored_when_prototype_lookup_throws() {
    assert_eq!(
        crate::eval(
            r#"
        var token = {}, caught = false;
        function Outer() {
            var target = new Proxy(function() {}, {
                get: function(target, key) {
                    if (key === 'prototype') throw token;
                    return target[key];
                }
            });
            try { Reflect.construct(Boolean, [true], target); }
            catch (e) { caught = e === token; }
            this.ok = new.target === Outer;
        }
        new Outer().ok && caught;
    "#
        )
        .unwrap(),
        Value::Boolean(true)
    );
}

#[test]
fn uninitialized_lexical_has_an_installed_throwing_entry() {
    let native = Value::Function(Function::uninitialized_lexical_marker());
    let error = call_function_slice(&native, Value::Undefined, &[], &mut empty_root().context())
        .unwrap_err();
    assert!(error.message.starts_with("ReferenceError:"));
}
