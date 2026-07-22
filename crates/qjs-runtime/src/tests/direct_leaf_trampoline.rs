use crate::{Value, eval};

#[test]
fn direct_leaf_trampoline_handles_ten_thousand_recursive_frames() {
    assert_eq!(
        eval(
            "function depth(value) { \
               if (value === 0) return 0; \
               return depth(value - 1) + 1; \
             } \
             depth(10000);"
        ),
        Ok(Value::Number(10_000.0))
    );
}

#[test]
fn direct_leaf_trampoline_handles_ten_thousand_recursive_getter_frames() {
    assert_eq!(
        eval(
            "var depth = 0; \
             var key = 'value'; \
             var holder = {}; \
             Object.defineProperty(holder, 'value', { \
               get: function() { \
                 if (this !== holder) return -1; \
                 if (depth === 10000) return depth; \
                 depth = depth + 1; \
                 return this[key]; \
               } \
             }); \
             holder.value;"
        ),
        Ok(Value::Number(10_000.0))
    );
}

#[test]
fn direct_leaf_trampoline_handles_ten_thousand_indexed_getter_frames() {
    assert_eq!(
        eval(
            "var depth = 0; \
             var holder = {}; \
             Object.defineProperty(holder, '0', { \
               get: function() { \
                 if (this !== holder) return -1; \
                 if (depth === 10000) return depth; \
                 depth = depth + 1; \
                 return this[0]; \
               } \
             }); \
             holder[0];"
        ),
        Ok(Value::Number(10_000.0))
    );
}

#[test]
fn direct_leaf_getter_errors_reach_the_parent_catch_frame() {
    assert_eq!(
        eval(
            "var marker = {}; \
             var depth = 0; \
             var holder = {}; \
             Object.defineProperty(holder, 'value', { \
               get: function() { \
                 if (depth === 64) throw marker; \
                 depth = depth + 1; \
                 return this.value; \
               } \
             }); \
             var preserved = 10; \
             var same = false; \
             try { preserved = preserved + holder.value * 2; } \
             catch (error) { same = error === marker; } \
             same + ':' + preserved + ':' + depth;"
        ),
        Ok(Value::String("true:10:64".to_owned().into()))
    );
}

#[test]
fn direct_leaf_trampoline_preserves_arguments_receiver_and_parent_operands() {
    assert_eq!(
        eval(
            "function zero() { return 'z'; } \
             function one(first) { return first; } \
             function two(first, second) { return first + second; } \
             function three(first, second, third) { return first + second + third; } \
             function numberLeaf(value) { \
               if (value < 0) return 0; \
               return value; \
             } \
             var holder = { \
               prefix: 'm', \
               leaf: function(first, second, third) { \
                 return this.prefix + first + second + third; \
               } \
             }; \
             zero() + ':' + one('1') + ':' + two('2', '3') + ':' + \
               three('4', '5', '6') + ':' + holder.leaf('7', '8', '9') + ':' + \
               (10 + numberLeaf(3) * 2);"
        ),
        Ok(Value::String("z:1:23:456:m789:16".to_owned().into()))
    );
}

#[test]
fn direct_leaf_trampoline_unwinds_thrown_identity_through_finally_frames() {
    assert_eq!(
        eval(
            "var marker = {}; \
             var log = ''; \
             function inner() { \
               try { throw marker; } finally { log = log + 'i'; } \
             } \
             function middle() { \
               try { inner(); } finally { log = log + 'm'; } \
             } \
             function outer() { \
               try { middle(); } finally { log = log + 'o'; } \
             } \
             var same = false; \
             try { outer(); } catch (error) { \
               same = error === marker; \
               log = log + 'c'; \
             } \
             same + ':' + log;"
        ),
        Ok(Value::String("true:imoc".to_owned().into()))
    );
}

#[test]
fn direct_leaf_trampoline_reuses_cleared_frame_storage() {
    assert_eq!(
        eval(
            "function maybeStore(store, value) { \
               var local; \
               if (store) local = value; \
               return local; \
             } \
             var last; \
             for (var index = 0; index < 1024; index = index + 1) { \
               last = maybeStore(true, { index: index }); \
             } \
             last.index === 1023 && maybeStore(false) === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn eval_with_and_closure_calls_stay_on_semantic_fallbacks() {
    assert_eq!(
        eval(
            "function evalFallback(value) { return eval('value + 1'); } \
             function withFallback(object) { with (object) { return value; } } \
             function closureFallback(value) { \
               return function() { return value; }; \
             } \
             evalFallback(1) + ':' + withFallback({ value: 3 }) + ':' + \
               closureFallback(4)();"
        ),
        Ok(Value::String("2:3:4".to_owned().into()))
    );
}

#[test]
fn generator_async_class_proxy_and_bound_calls_stay_on_fallbacks() {
    assert_eq!(
        eval(
            "function* generatorFallback() { yield 5; } \
             async function asyncFallback() { return 6; } \
             class Box { constructor(value) { this.value = value; } } \
             function add(first, second) { return first + second; } \
             var proxy = new Proxy(add, { \
               apply: function(target, receiver, argumentsList) { \
                 return argumentsList[0] * 2; \
               } \
             }); \
             var bound = add.bind(null, 9); \
             generatorFallback().next().value + ':' + \
               (asyncFallback() instanceof Promise) + ':' + \
               (new Box(7)).value + ':' + proxy(4) + ':' + bound(1);"
        ),
        Ok(Value::String("5:true:7:8:10".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_hit_then_miss_preserves_arguments_and_coercion_side_effects() {
    assert_eq!(
        eval(
            "function one(value) { return value + 1; } \
             function two(first, second) { return first + second; } \
             function three(first, second, third) { return first + second + third; } \
             var argumentCalls = 0; \
             var coercions = 0; \
             var boxed = { valueOf: function() { coercions = coercions + 1; return 4; } }; \
             function getBoxed() { argumentCalls = argumentCalls + 1; return boxed; } \
             var holder = { one: one }; \
             var values = [ \
               one(1), one(getBoxed()), \
               holder.one(2), holder.one(getBoxed()), \
               two(1, 2), two(getBoxed(), 2), \
               three(1, 2, 3), three(getBoxed(), 2, 3), \
               two(...[1, 2]), two(...[getBoxed(), 2]) \
             ]; \
             values.join(':') + ':' + argumentCalls + ':' + coercions;"
        ),
        Ok(Value::String("2:5:3:5:3:6:6:9:3:6:5:5".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_getter_hit_then_miss_runs_coercion_once() {
    assert_eq!(
        eval(
            "var coercions = 0; \
             var boxed = { valueOf: function() { coercions = coercions + 1; return 4; } }; \
             function makeState() { \
               var captured = 1; \
               var holder = {}; \
               Object.defineProperty(holder, 'value', { \
                 get: function() { return captured + 1; } \
               }); \
               return { \
                 holder: holder, \
                 set: function(value) { captured = value; } \
               }; \
             } \
             var state = makeState(); \
             var first = state.holder.value; \
             state.set(boxed); \
             var second = state.holder.value; \
             first + ':' + second + ':' + coercions;"
        ),
        Ok(Value::String("2:5:1".to_owned().into()))
    );
}
