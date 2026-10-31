//! Observable binding/context contracts for prepared synchronous calls.
use super::{FunctionCallEnv, function_env};
use crate::{Bytecode, CallEnv, Value, eval};

fn inspect(source: &str, check: impl FnOnce(&FunctionCallEnv<'_>, &Bytecode)) {
    let callee = eval(source).expect("function source evaluates");
    let Value::Function(function) = &callee else {
        panic!("expected function");
    };
    let code = function.bytecode.as_ref().unwrap();
    let env = CallEnv::new(function.realm().unwrap().clone());
    let arguments = [Value::Number(7.0)];
    let prepared = function_env(
        function,
        code,
        &callee,
        Value::Undefined,
        &arguments,
        &env,
        false,
    );
    check(&prepared, code);
}

#[test]
fn positional_entry_omits_parameter_names_and_static_eval_marker() {
    inspect(
        "(function ({x}, y = x + 1, ...tail) { return function () { return x + y + tail[0]; }; })",
        |prepared, code| {
            assert!(prepared.direct_call_slots.is_none());
            assert!(prepared.call_inputs.is_some());
            for name in code
                .parameter_slots()
                .iter()
                .map(|&slot| code.local_name_at(slot).unwrap())
            {
                assert!(!prepared.env.has_frame_binding(name), "{name}");
            }
            assert!(!prepared.env.has_frame_binding("tail"));
            assert_eq!(
                prepared.env.owner_kind(),
                Some(crate::context::OwnerKind::Function)
            );
        },
    );
}

#[test]
fn mapped_dynamic_and_suspended_parameters_use_positional_inputs() {
    for source in [
        "(function (value) { return arguments[0]; })",
        "(function (value) { return eval('value'); })",
        "(function (value) { return function () { return eval('value'); }; })",
        "(function (value) { class C { #field = eval('value'); } return C; })",
        "(function (value) { class C { get #field() { return eval('value'); } } return C; })",
        "(function (value) { class C { [eval('value')]() {} } return C; })",
        "(function (value) { class C { static { eval('value'); } } return C; })",
        "(function* (value) { yield value; })",
        "(async function (value) { return value; })",
    ] {
        inspect(source, |prepared, _| {
            assert!(prepared.call_inputs.is_some(), "{source}");
            if prepared.direct_call_slots.is_none() {
                assert_eq!(prepared.env.get_local("value"), None, "{source}");
            }
        });
    }
}

#[test]
fn parameter_metadata_distinguishes_binding_names_from_property_keys() {
    for (source, shadows) in [
        (
            "(function ({arguments: other}) { return arguments; })",
            false,
        ),
        ("(function ({arguments}) { return arguments; })", true),
        ("(function ([arguments]) { return arguments; })", true),
        (
            "(function (...{length: arguments}) { return arguments; })",
            true,
        ),
    ] {
        inspect(source, |_, code| {
            assert_eq!(!code.needs_arguments_object(), shadows)
        });
    }
}

#[test]
fn defaults_patterns_rest_share_cells() {
    assert_eq!(
        eval(
            r###"(function(){function make({x}, y=x+1, ...rest){let n=x+y+rest[0];return [function(v){n+=v;return n;},function(){return n;}];}let a=make({x:2},undefined,4),b=make({x:10},20,1);return [a[0](3),a[1](),b[1](),a[0](1)].join(':');})()"###
        ),
        Ok(Value::String("12:12:31:13".to_owned().into()))
    );
}

#[test]
fn duplicate_parameters_share_last_position() {
    assert_eq!(
        eval(
            r###"(function(){function make(a,a){return [()=>a,v=>a=v];}let x=make(3,8),y=make(3);x[1](5);return x[0]()+':'+(y[0]()===undefined);})()"###
        ),
        Ok(Value::String("5:true".to_owned().into()))
    );
}

#[test]
fn parameter_default_closure_sees_later_initialization() {
    assert_eq!(
        eval(r###"(function(){function make(get=()=>x,x=6){return get;}return make()();})()"###),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn mapped_arguments_and_eval_share_cells() {
    assert_eq!(
        eval(
            r###"(function(){function f(a){let read=()=>a;arguments[0]=4;eval('a+=3');return [read(),arguments[0]].join(':');}return f(1);})()"###
        ),
        Ok(Value::String("7:7".to_owned().into()))
    );
}

#[test]
fn with_shadow_preserves_parameter_cell() {
    assert_eq!(
        eval(
            r###"(function(){function f(a){var read=()=>a;with({a:9}){a++;}return read();}return f(1);})()"###
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn callback_throw_preserves_outer_cell() {
    assert_eq!(
        eval(
            r###"(function(){function outer(x){let read=()=>x;try{[2,3].map(function(a){let keep=()=>a;a+=10;throw keep();});}catch(e){x+=e;}return read();}return outer(5);})()"###
        ),
        Ok(Value::Number(17.0))
    );
}

#[test]
fn lexical_receiver_survives_native_call_boundary() {
    assert_eq!(
        eval(
            r###"(function(){function outer(x){return ()=>this.value+x;}let read=outer.call({value:7},4);return read.call({value:99});})()"###
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn derived_constructor_parameter_cell() {
    assert_eq!(
        eval(
            r###"(function(){class Base{constructor(x){this.x=x;}}class Derived extends Base{constructor(x=4){let get=()=>x;super(get());this.read=get;x+=1;}}let d=new Derived();return d.x+':'+d.read();})()"###
        ),
        Ok(Value::String("4:5".to_owned().into()))
    );
}

#[test]
fn default_derived_constructor_forwards_rest_without_positional_alias() {
    assert_eq!(
        eval(
            "class Base { constructor(...items) { this.items = items; } } class Child extends Base {} new Child(2, 3).items.join(':');"
        ),
        Ok(Value::String("2:3".to_owned().into()))
    );
}

#[test]
fn method_super_call_preserves_receiver() {
    assert_eq!(
        eval(
            "class Base { value() { return this.n; } } class Child extends Base { constructor(n) { super(); this.n = n; } probe(x) { let keep = function () { return x; }; return super.value() + keep(); } } new Child(5).probe(2);"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn named_self_and_callback_throw() {
    assert_eq!(
        eval(
            r###"(function(){let self=10;let f=function self(a){let read=()=>a;try{[1].forEach(function(){throw 2;});}catch(e){}return self===f?read():99;};return f(7)+':'+self;})()"###
        ),
        Ok(Value::String("7:10".to_owned().into()))
    );
}

#[test]
fn accessor_callback_reuses_parameter_cell() {
    assert_eq!(
        eval(
            r###"(function(){function make(value){var read=()=>value;return {get x(){value+=1;return read();},set x(n){value=n;}};}let obj=make(3);obj.x=10;return obj.x;})()"###
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn bound_proxy_forwarding_seeds_parameters() {
    assert_eq!(
        eval(
            r###"(function(){function make(a){return ()=>a;}let bound=make.bind(null);let proxy=new Proxy(bound,{apply(t,ths,args){return Reflect.apply(t,ths,args);}});return proxy(8)();})()"###
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn private_receiver_in_captured_method() {
    assert_eq!(
        eval(
            r###"(function(){class C{#n=1;make(x){return ()=>this.#n+x;}}return new C().make(4)();})()"###
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn captured_local_above_mask_range() {
    let declarations = (0..140)
        .map(|i| format!("let v{i}=seed+{i};"))
        .collect::<String>();
    let source = format!(
        "(function(){{function make(seed){{{declarations}return [()=>++v139,()=>v139];}}let pair=make(10);return pair[0]()+\":\"+pair[1]();}})()"
    );
    assert_eq!(
        eval(&source),
        Ok(Value::String("150:150".to_owned().into()))
    );
}

#[test]
fn class_methods_share_lazily_boxed_parameter() {
    assert_eq!(
        eval(
            r###"(function(){function make(value){class C{get(){return value;}set(x){value=x;}}return new C();}let c=make(2);c.set(8);return c.get();})()"###
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn constructor_lexical_new_target_survives_return() {
    assert_eq!(
        eval(
            r###"(function(){function Parent(x){let get=()=>new.target;return get;}let get=new Parent(1);return get()===Parent;})()"###
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn named_native_callback_restores_outer_parameter() {
    assert_eq!(
        eval(
            r###"(function(){function outer(self){let read=()=>self;let fn=function self(x){let inner=()=>x;return inner();};[7].map(fn);return read();}return outer(11);})()"###
        ),
        Ok(Value::Number(11.0))
    );
}

#[test]
fn rest_pattern_can_shadow_arguments() {
    assert_eq!(
        eval(
            r###"(function(){function f(...{length:arguments}){let read=()=>arguments;return read();}return f(1,2,3);})()"###
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn pattern_property_key_does_not_shadow_arguments() {
    assert_eq!(
        eval(
            r###"(function(){function f({arguments:a}){return arguments[0].arguments+a;}return f({arguments:5});})()"###
        ),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn generator_prologue_retains_parameter_context() {
    assert_eq!(
        eval(
            r###"(function(){function* g({x},y=()=>x,...rest){yield y()+rest[0];}return g({x:2},undefined,3).next().value;})()"###
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn static_native_callbacks_keep_cells_live_across_throws() {
    assert_eq!(
        eval(
            "function outer(x) { let read = function () { return x; }; try { [2,3].map(function (a) { let keep = function () { return a; }; a += 10; throw keep(); }); } catch (e) { x += e; } return read(); } outer(5);"
        ),
        Ok(Value::Number(17.0))
    );
}

#[test]
fn mapped_arguments_keep_distinct_cells_after_frame_storage_reuse() {
    assert_eq!(
        eval(
            "function make(value) { let args = arguments; let read = function () { return value; }; return {args: args, read: read}; } let a = make(1), b = make(2); a.args[0] = 9; a.read() + ':' + b.read();"
        ),
        Ok(Value::String("9:2".to_owned().into()))
    );
}

#[test]
fn escaped_thrown_closures_outlive_recycled_cold_state() {
    assert_eq!(
        eval(
            "function make(value, fail) { let read = function () { return value; }; try { if (fail) throw read; return read; } finally {} } let first; try { make(3, true); } catch (e) { first = e; } let second = make(5, false); first() + ':' + second();"
        ),
        Ok(Value::String("3:5".to_owned().into()))
    );
}

#[test]
fn static_coercion_hooks_update_the_shared_parameter_cell() {
    assert_eq!(
        eval(
            "function outer(value) { let read = function () { return value; }; let object = { valueOf() { value += 2; return 1; } }; let result = +object; return read() + ':' + result; } outer(5);"
        ),
        Ok(Value::String("7:1".to_owned().into()))
    );
}

#[test]
fn class_key_arrows_retain_one_lexical_owner_across_resume() {
    let value = eval(
        r#"let first, second; const receiver = {};
        function* g() { class C { #p;
            [(first = () => this, #p in {}, yield 'key', second = () => this, 'm')]() {}
        } }
        const it = g.call(receiver); it.next(); it.next(); [first, second, receiver];"#,
    )
    .unwrap();
    let Value::Array(values) = value else {
        panic!("array result");
    };
    let values = values.to_vec();
    let (Value::Function(first), Value::Function(second)) = (&values[0], &values[1]) else {
        panic!("key closures");
    };
    let first = first.lexical_call().unwrap();
    let second = second.lexical_call().unwrap();
    assert!(std::rc::Rc::ptr_eq(&first, &second));
    assert_eq!(first.this.read(), Ok(values[2].clone()));
    assert_eq!(first.kind, crate::context::OwnerKind::Function);
}

#[test]
fn parameter_only_arrow_does_not_retain_a_lexical_owner() {
    let value = eval("(value => value + 1);").unwrap();
    let Value::Function(function) = value else {
        panic!("arrow");
    };
    assert!(function.lexical_call().is_none());
    assert!(!function.has_cold_lexical_state());
}

#[test]
fn nested_arrow_new_target_demand_retains_the_original_owner() {
    let value = eval("function F() { return () => () => new.target; } new F();").unwrap();
    let Value::Function(function) = value else {
        panic!("outer arrow");
    };
    let owner = function.lexical_call().expect("nested new.target owner");
    assert!(matches!(owner.new_target, Some(Value::Function(_))));
}

#[test]
fn parameter_assignment_fallback_is_distinct_from_a_body_var_declaration() {
    assert_eq!(
        eval(
            "var defaultTarget = 3; function f(value = (defaultTarget = 8)) { var defaultTarget; return String(defaultTarget); } f() + ':' + defaultTarget;"
        ),
        Ok(Value::String("undefined:8".into()))
    );
}

#[test]
fn non_simple_body_arguments_copy_preserves_value_identity_and_binding_independence() {
    assert_eq!(
        eval(
            r#"
        function f(read = () => arguments) {
            var arguments; const initial = arguments; arguments = 'body';
            return (read() === initial) + ':' + initial.length + ':' + arguments;
        }
        function g(value = (arguments = 11)) { var arguments; return arguments; }
        f(undefined, 7) + '|' + g();
    "#
        ),
        Ok(Value::String("true:2:body|11".into()))
    );
}

#[test]
fn non_simple_body_eval_can_shadow_parameter_arguments() {
    assert_eq!(
        eval(
            r#"
        function f(read = () => arguments) {
            eval('var arguments = ["body"]'); arguments = ['changed'];
            return arguments[0] + ':' + read()[1];
        }
        f(undefined, 'parameter');
    "#
        ),
        Ok(Value::String("changed:parameter".into()))
    );
    assert_eq!(
        eval("function f(value = 0) { eval('var arguments = [7]'); return arguments[0]; } f(3);"),
        Ok(Value::Number(7.0))
    );
}
