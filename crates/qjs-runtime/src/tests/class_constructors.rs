use crate::{Value, eval};

#[test]
fn default_constructor_creates_instance() {
    assert_eq!(
        eval("class C {} typeof new C();"),
        Ok(Value::String("object".to_owned()))
    );
}

#[test]
fn default_derived_constructor_arguments_do_not_shadow_outer_bindings() {
    assert_eq!(
        eval(
            "var args, that; \
             class Base { constructor() { that = this; args = arguments; } } \
             class Derived extends Base {} \
             new Derived(0, 1, 2); \
             args.length + ':' + (that instanceof Derived);"
        ),
        Ok(Value::String("3:true".to_owned()))
    );
}

#[test]
fn default_derived_constructor_forwards_arguments_without_spread_iteration() {
    assert_eq!(
        eval(
            "Array.prototype[Symbol.iterator] = function() { throw new Error('iterated'); }; \
             class Base { constructor(value) { this.value = value; } } \
             class Derived extends Base {} \
             new Derived(5).value;"
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn null_extending_class_uses_function_prototype_as_constructor_parent() {
    assert_eq!(
        eval(
            "var reached = 0, after = 0, superError, returnError; \
             class C extends null { \
               constructor(mode) { \
                 if (mode === 'super') { \
                   reached += 1; \
                   try { super(); } catch (error) { superError = error.name; } \
                   after += 1; \
                   return {}; \
                 } \
               } \
             } \
             try { new C('return'); } catch (error) { returnError = error.name; } \
             new C('super'); \
             [Object.getPrototypeOf(C) === Function.prototype, superError, returnError, reached, after].join(':');"
        ),
        Ok(Value::String(
            "true:TypeError:ReferenceError:1:1".to_owned()
        ))
    );
}

#[test]
fn explicit_derived_constructor_must_call_super_before_returning() {
    assert!(
        eval("class B {} class C extends B { constructor() {} } new C();").is_err(),
        "derived constructor without super() must throw"
    );
}

#[test]
fn derived_super_property_requires_initialized_this() {
    assert!(
        eval(
            "class B {} \
             class C extends B { constructor() { super.m(); } } \
             new C();"
        )
        .is_err(),
        "super property access before super() must throw"
    );
}

#[test]
fn repeated_super_call_runs_parent_before_reference_error() {
    assert_eq!(
        eval(
            "var calls = 0; \
             class B { constructor() { calls += 1; } } \
             class C extends B { \
               constructor() { \
                 super(); \
                 try { super(); } catch (e) {} \
               } \
             } \
             new C(); calls;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn derived_constructor_return_waits_for_lexical_super_in_cleanup() {
    assert_eq!(
        eval(
            "class C extends class {} { \
               constructor() { \
                 var f = () => super(); \
                 try { return; } finally { f(); } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval(
            "class C extends class {} { \
               constructor() { \
                 var f = () => super(); \
                 try { throw null; } catch (e) { return; } finally { f(); } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval(
            "var iter = { \
               [Symbol.iterator]() { return this; }, \
               next() { return { done: false }; }, \
               return() { this.f(); return { done: true }; } \
             }; \
             class C extends class {} { \
               constructor() { \
                 iter.f = () => super(); \
                 for (var k of iter) { return; } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned()))
    );
}
