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

#[test]
fn computed_super_property_captures_base_before_key_coercion() {
    assert_eq!(
        eval(
            "var proto = { p: 'ok' }; \
             var proto2 = { p: 'bad' }; \
             var obj = { \
               __proto__: proto, \
               get() { return super[key]; }, \
               set() { super[key] = 10; } \
             }; \
             var result; \
             var key = { toString() { Object.setPrototypeOf(obj, proto2); return 'p'; } }; \
             Object.defineProperty(proto, 'p', { set(v) { result = 'ok'; }, get() { return 'ok'; } }); \
             Object.defineProperty(proto2, 'p', { set(v) { result = 'bad'; }, get() { return 'bad'; } }); \
             var read = obj.get(); \
             Object.setPrototypeOf(obj, proto); \
             read + ':' + (obj.set(), result);"
        ),
        Ok(Value::String("ok:ok".to_owned().into()))
    );
}

#[test]
fn super_call_uses_current_constructor_prototype_after_argument_evaluation() {
    assert_eq!(
        eval(
            "var evaluatedArg = false; \
             var caught; \
             class C extends Object { \
               constructor() { \
                 try { super(evaluatedArg = true); } catch (error) { caught = error; } \
               } \
             } \
             Object.setPrototypeOf(C, parseInt); \
             try { new C(); } catch (_) {} \
             (caught instanceof TypeError) + ':' + evaluatedArg;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn super_property_compound_assignment_reads_and_writes_through_super() {
    // `super.x <op>= v` reads the prototype accessor, combines, and writes back
    // through the home object's prototype setter.
    assert_eq!(
        eval(
            "class A { get x() { return this._x; } set x(v) { this._x = v; } } \
             class B extends A { \
               constructor() { super(); this._x = 10; } \
               run() { super.x += 5; return super.x; } \
             } \
             new B().run();"
        ),
        Ok(Value::Number(15.0))
    );
    // Computed key is evaluated exactly once.
    assert_eq!(
        eval(
            "var keyEvals = 0; \
             class A { get x() { return this._x; } set x(v) { this._x = v; } } \
             class B extends A { \
               constructor() { super(); this._x = 1; } \
               run() { super[(keyEvals++, 'x')] += 100; return super.x + ':' + keyEvals; } \
             } \
             new B().run();"
        ),
        Ok(Value::String("101:1".to_owned().into()))
    );
}

#[test]
fn super_property_update_returns_old_or_new_value() {
    assert_eq!(
        eval(
            "class A { get x() { return this._x; } set x(v) { this._x = v; } } \
             class B extends A { \
               constructor() { super(); this._x = 4; } \
               post() { return super.x++; } \
               peek() { return super.x; } \
             } \
             var b = new B(); var old = b.post(); old + ':' + b.peek();"
        ),
        Ok(Value::String("4:5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "class A { get x() { return this._x; } set x(v) { this._x = v; } } \
             class B extends A { \
               constructor() { super(); this._x = 4; } \
               pre() { return ++super.x; } \
             } \
             new B().pre();"
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn super_property_logical_assignment_short_circuits() {
    assert_eq!(
        eval(
            "class A { get x() { return this._x; } set x(v) { this._x = v; } } \
             class B extends A { \
               constructor() { super(); this._x = 0; } \
               run() { super.x ||= 9; super.x &&= 7; return super.x; } \
             } \
             new B().run();"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn delete_super_property_throws_reference_error_without_evaluating_key() {
    // `delete super.x` is a runtime ReferenceError, thrown before the property
    // key expression is evaluated, so a computed key with a side effect never
    // runs and a throwing ToString is never reached.
    assert_eq!(
        eval(
            "var obj = { m() { delete super.x; return 'no throw'; } }; \
             try { obj.m(); } catch (e) { e.name; }"
        ),
        Ok(Value::String("ReferenceError".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var keyEvaluated = false; \
             var obj = { m() { delete super[keyEvaluated = true]; } }; \
             try { obj.m(); } catch (e) { e.name + ':' + keyEvaluated; }"
        ),
        Ok(Value::String("ReferenceError:false".to_owned().into()))
    );
}

#[test]
fn computed_super_assignment_key_coercion_error_is_catchable() {
    // A throwing ToPropertyKey on a computed super target must surface as a
    // catchable JS exception, not an uncatchable VM fault.
    assert_eq!(
        eval(
            "class A {} \
             class B extends A { \
               run() { \
                 var key = { toString() { throw new TypeError('boom'); } }; \
                 try { super[key]; return 'no throw'; } \
                 catch (e) { return e.name; } \
               } \
             } \
             new B().run();"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
}
