//! Identity representation demand is a compiler property, separate from output correctness.

use crate::{Value, eval};

const PICK: &str = "function pick(a,b,n){var s=0,r,g=0;for(var k=0;k<n&&g<9;k++,g++){r=k<2?a:b;s+=r.value;}return s;}";
const COPY: &str = "function pick(a,b,n){var s=0,r,c,d,g=0;for(var k=0;k<n&&g<9;k++,g++){c=k<2?a:b;d=c;r=d;s+=r.value;}return s;}";

fn bytecode(source: &str) -> super::super::ir::Bytecode {
    let script = qjs_parser::parse_script(source).unwrap();
    let script = super::super::compile_script(&script).unwrap();
    script
        .code
        .iter()
        .find_map(|op| match op {
            super::super::ir::Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("source contains an ordinary function")
}

#[test]
fn identity_joins_and_copies_box_each_parameter_source() {
    for source in [PICK, COPY] {
        let code = bytecode(source);
        let plans = super::compile_all(&code);
        let plan = plans.first().expect("finite loop should compile");
        for name in ["a", "b"] {
            let slot = code
                .locals
                .iter()
                .position(|local| local.name == name)
                .unwrap() as u32;
            assert!(
                plan.boxed_locals
                    .iter()
                    .any(|&(_, candidate)| candidate == slot),
                "{name}: {source}"
            );
            assert!(
                !plan
                    .local_slots
                    .iter()
                    .any(|&(_, candidate)| candidate == slot),
                "{name} must not seed an object into scalars"
            );
        }
    }
}

#[cfg(feature = "perf-counters")]
#[test]
fn parameter_identity_joins_enter_the_tier() {
    for source in [PICK, COPY] {
        crate::diagnostics::reset_counters();
        assert_eq!(
            eval(&format!("{source} pick({{value:2}},{{value:7}},6);")),
            Ok(Value::Number(32.0))
        );
        assert!(
            crate::diagnostics::counters().unwrap().loop_plan_entries > 0,
            "checksum alone does not establish tier entry: {source}"
        );
    }
}

#[cfg(feature = "perf-counters")]
fn entered(source: &str, expected: Value) {
    crate::diagnostics::reset_counters();
    assert_eq!(eval(source), Ok(expected), "{source}");
    assert!(
        crate::diagnostics::counters().unwrap().loop_plan_entries > 0,
        "{source}"
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn nested_joins_element_sources_and_loop_carried_copies_enter() {
    for (source, expected) in [
        (
            "function run(a,b,c){var s=0,r,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?(i<1?a:b):c;s+=r.other;}return s;} run({other:2},{other:3},{other:7});",
            33.0,
        ),
        (
            "function run(a,b){var s=0,r,tmp,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?a:b;tmp=a;a=b;b=tmp;s+=r.other;}return s;} run({other:2},{other:7});",
            27.0,
        ),
        (
            "function run(a,b){var s=0,r,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?a[i]:b[i];s+=r.other;}return s;} run([{other:2},{other:2}], [0,0,{other:7},{other:7},{other:7},{other:7}]);",
            32.0,
        ),
        (
            "function run(a,b){var s=0,r,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?a:b;if(i===4)break;s+=r.other;}return s;} run({other:2},{other:7});",
            18.0,
        ),
    ] {
        entered(source, Value::Number(expected));
    }
}

#[test]
fn numeric_results_do_not_propagate_identity_demand_to_operands() {
    for source in [
        "function run(a,b){var s=0,r,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?a+1:b+1;s+=r;}return s;}",
        "function run(a,b){var s=0,r,g=0;for(var i=0;i<6&&g<9;i++,g++){r=i<2?a+1:b+1;s+=r.value;}return s;}",
    ] {
        let code = bytecode(source);
        let plans = super::compile_all(&code);
        let plan = plans
            .first()
            .expect("computed scalar branch should compile");
        for name in ["a", "b"] {
            let slot = code
                .locals
                .iter()
                .position(|local| local.name == name)
                .unwrap() as u32;
            assert!(
                plan.local_slots
                    .iter()
                    .any(|&(_, candidate)| candidate == slot)
            );
            assert!(
                !plan
                    .boxed_locals
                    .iter()
                    .any(|&(_, candidate)| candidate == slot)
            );
        }
    }
}

#[test]
fn mixed_computed_and_identity_joins_recompile_or_decline() {
    let boxed_then_scalar = bytecode(
        "function run(a){var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){r=i<2?a:1;s+=r.value;}return s;}",
    );
    assert!(!super::compile_all(&boxed_then_scalar).is_empty());
    // The scalar predecessor is already emitted. There is no new identity
    // source to promote there, so this direction keeps the old conservative
    // rejection rather than interpreting predecessor code as boxed.
    let scalar_then_boxed = bytecode(
        "function run(a){var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){r=i<2?1:a;s+=r.value;}return s;}",
    );
    assert!(super::compile_all(&scalar_then_boxed).is_empty());
    for branch in ["i<2?a:1", "i<2?1:a"] {
        assert_eq!(
            eval(&format!(
                "function run(a){{var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){{r={branch};s+=r.value;}}return Number.isNaN(s);}}run({{value:3}});"
            )),
            Ok(Value::Boolean(true))
        );
    }
}

#[cfg(feature = "perf-counters")]
#[test]
fn zero_trip_tdz_and_residency_preserve_interpreter_boundaries() {
    crate::diagnostics::reset_counters();
    assert_eq!(
        eval(
            "function run(a,n){var s=0,g=0;for(var i=0;i<n&&g<9;i++,g++){var r=i<2?a:uninitialized;s+=r.value;}let uninitialized={value:7};return s;}run({value:2},0);"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(crate::diagnostics::counters().unwrap().loop_plan_entries, 0);
    assert_eq!(
        eval(
            "function run(a,n){var s=0,g=0;for(var i=0;i<n&&g<9;i++,g++){var r=i<2?a:uninitialized;s+=r.value;}let uninitialized={value:7};return s;}try{run({value:2},4);}catch(e){e.name;}"
        ),
        Ok(Value::String("ReferenceError".into()))
    );
    super::test_residency::with_limit(2, || {
        entered(
            &format!("{COPY} pick({{value:2}},{{value:7}},6);"),
            Value::Number(32.0),
        );
    });
}

#[cfg(feature = "perf-counters")]
#[test]
fn effectful_method_resolution_and_null_fallback_run_once() {
    let body = "function run(a,b){var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){r=i<2?a:b;s+=r.verb(i);}return s;}";
    entered(
        &format!(
            "{body}var events=[];var a={{verb:function(n){{return n+10;}}}};var b={{get verb(){{events.push('get');return function(n){{events.push(this===b);return n+20;}};}}}};[run(a,b),events.join(',')].join('|');"
        ),
        Value::String("66|get,true,get,true".into()),
    );
    entered(
        &format!(
            "{body}var events=[];var a={{verb:function(n){{return n+10;}}}};var b=new Proxy({{verb:function(n){{return n+20;}}}},{{get(t,k,r){{events.push(k);return Reflect.get(t,k,r);}}}});[run(a,b),events.join(',')].join('|');"
        ),
        Value::String("66|verb,verb".into()),
    );
    entered(
        "function run(a,b){var s=0,r,g=0,i=0;try{for(;i<4&&g<9;i++,g++){r=i<2?a:b;s+=r.verb(i);}}catch(e){return [e.name,s,i].join('|');}}run({verb:function(n){return n+10;}},null);",
        Value::String("TypeError|21|2".into()),
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn coercion_and_callback_mutation_keep_completed_effects() {
    entered(
        "var events=[];function run(a,b){var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){r=i<2?a:b;s+=r.value;}return s;}var b={value:{valueOf:function(){events.push('value');return 7;}}};[run({value:2},b),events.join(',')].join('|');",
        Value::String("18|value,value".into()),
    );
    entered(
        "var events=[];function run(a,b){var s=0,r,g=0;for(var i=0;i<4&&g<9;i++,g++){r=(i&1)===0?a:b;s+=r.verb(i);}return s;}var a={verb:function(n){return n+10;}};var b={verb:function(n){events.push(this===b);a.verb=function(n){return n+100;};return n+20;}};[run(a,b),events.join(',')].join('|');",
        Value::String("156|true,true".into()),
    );
}
