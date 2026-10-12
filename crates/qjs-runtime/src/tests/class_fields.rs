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
fn public_string_fields_keep_default_descriptors_and_definition_order() {
    assert_eq!(
        eval(
            "class C { first = 1; second = this.first + 1; } \
             let c = new C(); \
             let first = Object.getOwnPropertyDescriptor(c, 'first'); \
             let second = Object.getOwnPropertyDescriptor(c, 'second'); \
             [Object.keys(c).join(','), first.writable, first.enumerable, first.configurable, second.value].join(':');"
        ),
        Ok(Value::String(
            "first,second:true:true:true:2".to_owned().into()
        ))
    );
}

#[test]
fn public_field_falls_back_when_an_earlier_initializer_owns_a_later_key() {
    assert_eq!(
        eval(
            "let caught = false; \
             class C { \
               first = (Object.defineProperty(this, 'later', { value: 1, writable: true, enumerable: false, configurable: false }), 0); \
               later = 2; \
             } \
             try { new C(); } catch (error) { caught = error instanceof TypeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn public_field_falls_back_for_nonextensible_derived_receiver() {
    assert_eq!(
        eval(
            "function Base() { return Object.preventExtensions({}); } \
             class C extends Base { field = 1; } \
             let caught = false; \
             try { new C(); } catch (error) { caught = error instanceof TypeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn public_field_on_a_global_receiver_keeps_lexical_and_property_names_distinct() {
    assert_eq!(
        eval(
            "let mirrored = 0; \
             function Base() { return globalThis; } \
             class C extends Base { mirrored = 7; } \
             new C(); \
             [mirrored, globalThis.mirrored].join(':');"
        ),
        Ok(Value::String("0:7".to_owned().into()))
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

#[test]
fn a_field_reading_a_captured_binding_s_property_sees_its_current_value() {
    assert_eq!(
        eval(
            "class Palette { static base = 'red'; }
             class Swatch { color = Palette.base; tone = Palette.missing; }
             var out = [new Swatch().color];
             Palette.base = 'blue';
             out.push(new Swatch().color, String(new Swatch().tone));
             Object.defineProperty(Palette, 'base', { get() { return 'computed'; } });
             out.push(new Swatch().color);
             class Early { static seen = new Early().probe; probe = Early.name; }
             out.push(Early.seen);
             out.join(',');"
        ),
        Ok(Value::String("red,blue,undefined,computed,Early".into()))
    );
}

#[test]
fn constant_definitions_keep_duplicate_order_and_default_descriptors() {
    assert_eq!(
        eval(
            "class C { b=1; a=2; b=3; [2]=4; [1]=5; } let c=new C(); let d=Object.getOwnPropertyDescriptor(c,'b'); [Object.keys(c), c.b, d.writable, d.enumerable, d.configurable].join(':');"
        ),
        Ok(Value::String("1,2,b,a:3:true:true:true".into()))
    );
}

#[test]
fn constant_definitions_replace_configurable_properties_without_setters() {
    assert_eq!(
        eval(
            "let calls=0; let target={}; Object.defineProperty(target,'a',{set(v){calls++;},configurable:true}); function Base(){return target;} class C extends Base { a=1; b=2; } new C(); let d=Object.getOwnPropertyDescriptor(target,'a'); [calls,target.a,target.b,d.writable,d.enumerable,d.configurable].join(':');"
        ),
        Ok(Value::String("0:1:2:true:true:true".into()))
    );
}

#[test]
fn initializer_observes_completed_segment_and_no_future_fields() {
    assert_eq!(
        eval(
            "let seen; class C { a=1; b=2; observer=(seen=[Object.keys(this).join(','),this.a,this.b,'c' in this].join(':')); c=3; d=4; } let c=new C(); seen+':'+Object.keys(c).join(',');"
        ),
        Ok(Value::String("a,b:1:2:false:a,b,observer,c,d".into()))
    );
}

#[test]
fn later_initializer_exception_preserves_only_completed_fields() {
    assert_eq!(
        eval(
            "let saved; function fail(x){saved=x;throw 'boom';} class C { a=1; b=2; c=fail(this); d=3; e=4; } let error; try{new C();}catch(e){error=e;} [error,Object.keys(saved).join(','),saved.a,saved.b,'c' in saved,'d' in saved].join(':');"
        ),
        Ok(Value::String("boom:a,b:1:2:false:false".into()))
    );
}

#[test]
fn rejected_descriptor_in_constant_run_keeps_prior_definitions() {
    assert_eq!(
        eval(
            "let target={}; Object.defineProperty(target,'b',{value:9,configurable:false}); function Base(){return target;} class C extends Base { a=1;b=2;c=3; } let error;try{new C();}catch(e){error=e instanceof TypeError;} [error,target.a,target.b,'c' in target].join(':');"
        ),
        Ok(Value::String("true:1:9:false".into()))
    );
}

#[test]
fn frozen_or_nonextensible_receiver_rejects_before_new_fields() {
    assert_eq!(
        eval(
            "let targets=[Object.freeze({}),Object.preventExtensions({})]; let out=[]; for(let i=0;i<2;i++){let target=targets[i];function Base(){return target;}class C extends Base {a=1;b=2;}try{new C();}catch(e){out.push(e instanceof TypeError,Object.keys(target).length);}}out.join(':');"
        ),
        Ok(Value::String("true:0:true:0".into()))
    );
}

#[test]
fn freezing_initializer_terminates_constant_installation_before_failure() {
    assert_eq!(
        eval(
            "let saved; class C { a=1;b=2;stop=(saved=this,Object.freeze(this),0);c=3;d=4;}let error;try{new C();}catch(e){error=e instanceof TypeError;}[error,Object.keys(saved).join(','),'stop' in saved,'c' in saved].join(':');"
        ),
        Ok(Value::String("true:a,b:false:false".into()))
    );
}

#[test]
fn proxy_traps_observe_each_constant_definition_and_rejection() {
    assert_eq!(
        eval(
            "let target={},log=[];function Base(){return new Proxy(target,{defineProperty(t,k,d){log.push(k+':'+Object.keys(t).join(',')+':'+d.value+':'+d.writable+':'+d.enumerable+':'+d.configurable);if(k==='b')return false;return Reflect.defineProperty(t,k,d);}});}class C extends Base {a=1;b=2;c=3;}let error;try{new C();}catch(e){error=e instanceof TypeError;}[error,log.join('|'),Object.keys(target).join(',')].join('/');"
        ),
        Ok(Value::String(
            "true/a::1:true:true:true|b:a:2:true:true:true/a".into()
        ))
    );
}

#[test]
fn computed_keys_resolve_once_and_symbol_definitions_keep_order() {
    assert_eq!(
        eval(
            "let log=[],s=Symbol('s');function key(x){log.push(x);return x;} class C { [key('a')]=1;[key('b')]=2;[s]=3;c=4;d=5; }let first=new C();let second=new C();[log.join(','),Reflect.ownKeys(first).map(k=>typeof k==='symbol'?'symbol':k).join(','),first[s],second[s]].join(':');"
        ),
        Ok(Value::String("a,b:a,b,c,d,symbol:3:3".into()))
    );
}

#[test]
fn private_field_terminates_segment_and_observes_prior_definitions() {
    assert_eq!(
        eval(
            "class C {a=1;b=2;#x=this.a+this.b;c=4;d=5;read(){return this.#x;}}let c=new C();[c.read(),Object.keys(c).join(',')].join(':');"
        ),
        Ok(Value::String("3:a,b,c,d".into()))
    );
}

#[test]
fn exotic_array_superclass_result_keeps_length_definition_semantics() {
    assert_eq!(
        eval(
            "let saved=[];function Base(){return saved;}class C extends Base {0=1;1=2;length=3;after=4;}let error;try{new C();}catch(e){error=e instanceof TypeError;}[error,saved.length,saved[0],saved[1],'after' in saved].join(':');"
        ),
        Ok(Value::String("true:2:1:2:false".into()))
    );
}

#[test]
fn constant_global_receiver_definitions_sync_var_bindings() {
    assert_eq!(
        eval(
            "globalThis.mirroredA=0;globalThis.mirroredB=0;function Base(){return globalThis;}class C extends Base {mirroredA=1;mirroredB=2;}new C();[mirroredA,mirroredB,globalThis.mirroredA,globalThis.mirroredB].join(':');"
        ),
        Ok(Value::String("1:2:1:2".into()))
    );
}

#[test]
fn lazy_string_indices_cannot_be_replaced_by_constant_fields() {
    assert_eq!(
        eval(
            "let saved=new String('hi');function Base(){return saved;}class C extends Base {a=1;b=2;0=3;c=4;}let error;try{new C();}catch(e){error=e instanceof TypeError;}[error,saved.a,saved.b,saved[0],'c' in saved].join(':');"
        ),
        Ok(Value::String("true:1:2:h:false".into()))
    );
}

#[test]
fn typed_array_superclass_result_keeps_integer_index_definitions() {
    assert_eq!(
        eval(
            "let saved=new Uint8Array(2);function Base(){return saved;}class C extends Base {a=1;b=2;0=257;1=258;}new C();[saved.a,saved.b,saved[0],saved[1]].join(':');"
        ),
        Ok(Value::String("1:2:1:2".into()))
    );
}

#[test]
fn declined_long_constant_run_defines_every_field_sequentially() {
    let fields = (0..512).map(|i| format!("p{i}={i};")).collect::<String>();
    let script = format!(
        "let target={{}};Object.defineProperty(target,'fixed',{{value:null}});function Base(){{return target;}}class C extends Base {{{fields}}}new C();Object.keys(target).length===512 && target.p0===0 && target.p511===511;"
    );
    assert_eq!(eval(&script), Ok(Value::Boolean(true)));
}
