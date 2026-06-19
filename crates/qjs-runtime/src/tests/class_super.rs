use crate::{Value, eval};

#[test]
fn super_property_read_on_null_base_throws_type_error() {
    // GetSuperBase requires the home object's [[Prototype]] be object-coercible,
    // so reading `super.x` when it is null throws a TypeError (matching the
    // existing assignment path).
    assert_eq!(
        eval(
            "var o = { __proto__: null, m() { return super.x; } }; \
             var caught = false; \
             try { o.m(); } catch (error) { caught = error instanceof TypeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // A non-null super base still reads normally.
    assert_eq!(
        eval(
            "class A { foo() { return 7; } } class B extends A { bar() { return super.foo(); } } new B().bar();"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn super_property_assignment_to_null_base_evaluates_rhs_before_type_error() {
    assert_eq!(
        eval(
            "var count = 0; \
             class C { static m() { super.x = count += 1; } } \
             Object.setPrototypeOf(C, null); \
             var caught = false; \
             try { C.m(); } catch (error) { caught = error instanceof TypeError; } \
             caught + ':' + count;"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
}

#[test]
fn computed_super_property_assignment_to_null_base_evaluates_rhs_before_type_error() {
    assert_eq!(
        eval(
            "var count = 0; \
             class C { static m() { super[0] = count += 1; } } \
             Object.setPrototypeOf(C, null); \
             var caught = false; \
             try { C.m(); } catch (error) { caught = error instanceof TypeError; } \
             caught + ':' + count;"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
}

#[test]
fn computed_super_property_checks_this_before_key() {
    assert_eq!(
        eval(
            "var keyEvaluated = false; \
             class Derived extends Object { \
               constructor() { \
                 super[keyEvaluated = true]; \
               } \
             } \
             try { new Derived(); } catch (error) { error.name + ':' + keyEvaluated; }"
        ),
        Ok(Value::String("ReferenceError:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var keyEvaluated = false; \
             class Derived extends Object { \
               constructor() { \
                 super[keyEvaluated = true] = 1; \
               } \
             } \
             try { new Derived(); } catch (error) { error.name + ':' + keyEvaluated; }"
        ),
        Ok(Value::String("ReferenceError:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var keyEvaluated = false; \
             class Base { method() {} } \
             class Derived extends Base { \
               constructor() { \
                 super[keyEvaluated = 'method'](); \
               } \
             } \
             try { new Derived(); } catch (error) { error.name + ':' + keyEvaluated; }"
        ),
        Ok(Value::String("ReferenceError:false".to_owned().into()))
    );
}

#[test]
fn computed_super_property_evaluates_key_after_super_call() {
    assert_eq!(
        eval(
            "var keyEvaluated = 0; \
             var result; \
             class Base { get value() { return 7; } } \
             class Derived extends Base { \
               constructor() { \
                 super(); \
                 result = super[keyEvaluated += 1, 'value'] + ':' + keyEvaluated; \
               } \
             } \
             new Derived(); \
             result;"
        ),
        Ok(Value::String("7:1".to_owned().into()))
    );
}
