//! Cached plans retain code/parse facts; bindings and object identities stay live.
use crate::{Value, eval};
use std::collections::HashMap;

#[test]
fn repeated_declaration_free_direct_eval_uses_fresh_cached_blueprints() {
    let mut root = crate::context::HostRoot::new(crate::context::new_realm(HashMap::new()));
    let mut caller = root.context();
    let eval_function =
        crate::Function::new_native(None, 1, crate::NativeFunction::Eval, false, caller.realm());
    let receiver = Value::Undefined;
    let info = crate::native::CallInfo {
        callee: &eval_function,
        this_value: &receiver,
        is_construct: false,
    };
    let site = crate::context::DirectEvalSite {
        strict: false,
        in_parameters: false,
        scope: None,
        private: None,
    };
    let mut env = caller.with_native(&info, None, Some(&site));
    env.insert_realm("value".to_owned(), Value::Number(1.0));

    assert_eq!(
        crate::global::native_global_eval(
            &[Value::String("value + 1".to_owned().into())],
            &mut env,
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(env.direct_eval_cache_len(), 1);

    env.insert_realm("value".to_owned(), Value::Number(41.0));
    assert_eq!(
        crate::global::native_global_eval(
            &[Value::String("value + 1".to_owned().into())],
            &mut env,
        ),
        Ok(Value::Number(42.0))
    );
    assert_eq!(env.direct_eval_cache_len(), 1);

    let first_object =
        crate::global::native_global_eval(&[Value::String("({})".to_owned().into())], &mut env)
            .expect("first direct eval object literal should succeed");
    let second_object =
        crate::global::native_global_eval(&[Value::String("({})".to_owned().into())], &mut env)
            .expect("second direct eval object literal should succeed");
    match (&first_object, &second_object) {
        (Value::Object(first), Value::Object(second)) => assert!(!first.ptr_eq(second)),
        _ => panic!("direct eval object literals should produce objects"),
    }
    assert_eq!(env.direct_eval_cache_len(), 2);

    assert_eq!(
        eval(
            "function cachedWriteback() { \
               let local = 1; \
               eval('local += 1'); \
               return eval('local += 1'); \
             } \
             cachedWriteback();"
        ),
        Ok(Value::Number(3.0))
    );

    assert_eq!(
        crate::global::native_global_eval(
            &[Value::String(
                "var uncached = 1; uncached".to_owned().into()
            )],
            &mut env,
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(env.direct_eval_cache_len(), 2);
}

#[test]
fn cached_direct_eval_keeps_only_referenced_outer_bindings() {
    assert_eq!(
        eval(
            "var globalSelectedBinding = 100; \
             function selectedBindings(seed) { \
               var untouched = 23; \
               var selected = seed; \
               eval('selected += 1'); \
               var twice = eval('selected += 1'); \
               var firstThis = eval('this.offset + arguments[0]'); \
               var secondThis = eval('this.offset + arguments[0]'); \
               return twice + ':' + firstThis + ':' + secondThis + ':' + untouched + ':' + globalSelectedBinding; \
             } \
             selectedBindings.call({ offset: 4 }, 5);"
        ),
        Ok(Value::String("7:9:9:23:100".to_owned().into()))
    );

    // The declaration-bearing first eval must remain on the conservative
    // route, but its function-scope var is still a selected dynamic binding
    // when the repeated declaration-free source reaches the cache-hit route.
    assert_eq!(
        eval(
            "function readsPriorEvalBinding() { \
               eval('var introducedByPriorEval = 11'); \
               return eval('introducedByPriorEval') + eval('introducedByPriorEval'); \
             } \
             readsPriorEvalBinding();"
        ),
        Ok(Value::Number(22.0))
    );
}
