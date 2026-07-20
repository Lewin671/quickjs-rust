use crate::{Value, eval};

#[test]
fn instance_field_with_initializer() {
    assert_eq!(
        eval("class C { x = 41; } new C().x;"),
        Ok(Value::Number(41.0))
    );
}

#[test]
fn instance_field_without_initializer_is_undefined() {
    assert_eq!(eval("class C { x; } new C().x;"), Ok(Value::Undefined));
}

#[test]
fn fields_initialize_in_definition_order_seeing_earlier_fields() {
    // A later field's initializer observes earlier fields through `this`.
    assert_eq!(
        eval("class C { a = 1; b = this.a + 1; c = this.b + 1; } new C().c;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn simple_field_initializers_keep_receiver_and_class_capture() {
    assert_eq!(
        eval(
            "class C { a = 1; b = this.a + 1; self = C; } \
             let c = new C(); [c.b, c.self === C].join(':');"
        ),
        Ok(Value::String("2:true".to_owned().into()))
    );
}

#[test]
fn shared_instance_metadata_allows_initializer_reentrancy() {
    assert_eq!(
        eval(
            "class C { \
               first = Object.defineProperty(C, 'probe', { value: 7 }); \
               second = C.probe + 1; \
             } \
             let value = new C(); \
             value.first === C && value.second === 8;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn field_shadows_prototype_method_and_is_own_enumerable() {
    assert_eq!(
        eval(
            "class C { m() { return 'method'; } m = 7; } let c = new C(); [typeof c.m, c.m, Object.keys(c).join(',')].join('|');"
        ),
        Ok(Value::String("number|7|m".to_owned().into()))
    );
}

#[test]
fn prototype_methods_stay_non_enumerable_with_fields() {
    assert_eq!(
        eval(
            "class C { x = 1; m() {} } Object.keys(C.prototype).length === 0 && Object.keys(new C()).join(',');"
        ),
        Ok(Value::String("x".to_owned().into()))
    );
}

#[test]
fn static_field_runs_with_this_as_constructor() {
    assert_eq!(
        eval("class C { static self = this; } C.self === C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn static_field_value_installed_on_constructor() {
    assert_eq!(
        eval("class C { static n = 9; } C.n;"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn computed_field_key_evaluated_once_at_definition() {
    assert_eq!(
        eval(
            "var calls = 0; function k() { calls++; return 'f'; } class C { [k()] = 1; } new C(); new C(); calls;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn computed_field_key_installs_field() {
    assert_eq!(
        eval("let key = 'dyn'; class C { [key] = 5; } new C().dyn;"),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn computed_static_and_instance_fields_preserve_evaluation_order() {
    assert_eq!(
        eval(
            "function scanDescriptor(desc) { \
               let names = Object.getOwnPropertyNames(desc); \
               for (var i = 0; i < names.length; i++) {} \
             } \
             let i = 0; \
             class C { [i++] = i++; static [i++] = i++; [i++] = i++; } \
             let c = new C(); \
             scanDescriptor(Object.getOwnPropertyDescriptor(c, '0')); \
             [i, c[0], c[2], C[1], c.hasOwnProperty('1'), C.hasOwnProperty('0'), C.hasOwnProperty('2')].join(',');"
        ),
        Ok(Value::String("6,4,5,3,false,false,false".to_owned().into()))
    );
}

#[test]
fn derived_instance_fields_run_after_super() {
    // The field initializer sees state established by the base constructor.
    assert_eq!(
        eval(
            "class A { constructor() { this.tag = 'A'; } } class B extends A { x = this.tag + 'B'; } new B().x;"
        ),
        Ok(Value::String("AB".to_owned().into()))
    );
}

#[test]
fn derived_instance_field_sees_this_bound_for_pre_super_arrow() {
    assert_eq!(
        eval(
            "let probe, before, duringBase, fieldThis, probeThis, ctorThis; \
             class Base { constructor() { try { probe(); } catch (e) { duringBase = e.name; } } } \
             class C extends Base { \
               field = (fieldThis = this, probeThis = probe()); \
               constructor() { \
                 probe = () => this; \
                 try { probe(); } catch (e) { before = e.name; } \
                 super(); \
                 ctorThis = this; \
               } \
             } \
             let c = new C(); \
             [fieldThis === c, probeThis === c, ctorThis === c, before, duringBase].join(':');"
        ),
        Ok(Value::String(
            "true:true:true:ReferenceError:ReferenceError"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn default_derived_constructor_runs_field_init() {
    assert_eq!(
        eval(
            "class A { constructor() { this.a = 1; } } class B extends A { x = 5; } let b = new B(); b.a + b.x;"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn default_base_constructor_runs_field_init() {
    assert_eq!(
        eval("class C { x = 3; } new C().x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn field_initializer_closes_over_class_scope() {
    assert_eq!(
        eval("let v = 10; class C { x = v; } new C().x;"),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn super_property_in_field_initializer() {
    assert_eq!(
        eval("class A { get y() { return 8; } } class B extends A { x = super.y; } new B().x;"),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn super_property_assignment_in_field_initializer_arrow() {
    assert_eq!(
        eval(
            "class C {
               func = () => { super.prop = 'test262'; };
               static staticFunc = () => { super.staticProp = 'static test262'; };
             }
             let c = new C();
             c.func();
             C.staticFunc();
             c.prop + ':' + C.staticProp;"
        ),
        Ok(Value::String("test262:static test262".to_owned().into()))
    );
}

#[test]
fn instance_fields_are_writable_and_configurable() {
    assert_eq!(
        eval(
            "class C { x = 1; } let c = new C(); let d = Object.getOwnPropertyDescriptor(c, 'x'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,true,true".to_owned().into()))
    );
}

#[test]
fn derived_public_fields_define_through_proxy_receiver() {
    assert_eq!(
        eval(
            "let log = []; \
             let expectedTarget = null; \
             function Base() { \
               expectedTarget = this; \
               return new Proxy(this, { \
                 defineProperty(target, key, descriptor) { \
                   log.push(key); \
                   log.push(descriptor.value); \
                   log.push(target); \
                   log.push(descriptor.enumerable); \
                   log.push(descriptor.configurable); \
                   log.push(descriptor.writable); \
                   return Reflect.defineProperty(target, key, descriptor); \
                 } \
               }); \
             } \
             class C extends Base { f = 3; g = 'Test262'; } \
             let c = new C(); \
             c.f + ':' + c.g + ':' + \
               [log[0], log[1], log[2] === expectedTarget, log[3], log[4], log[5], \
                log[6], log[7], log[8] === expectedTarget, log[9], log[10], log[11]].join('|') + \
               ':' + (expectedTarget === null);"
        ),
        Ok(Value::String(
            "3:Test262:f|3|true|true|true|true|g|Test262|true|true|true|true:false"
                .to_owned()
                .into()
        ))
    );
}
