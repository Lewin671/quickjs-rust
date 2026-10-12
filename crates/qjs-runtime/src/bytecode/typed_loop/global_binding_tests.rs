//! Global sink admission and exact publication at interpreter continuations.

use crate::{Value, eval};

fn programs(source: &str) -> Vec<super::TypedLoopProgram> {
    let script = qjs_parser::parse_script(source).unwrap();
    let bytecode = crate::bytecode::compile_script(&script).unwrap();
    super::compile_all(&bytecode)
}

#[test]
fn global_scalar_stores_share_a_register_and_defer_without_observers() {
    let source = "var i = 0, sum = 0; for (; i < 20; i++) { sum += i; } sum * 100 + i;";
    let plans = programs(source);
    let plan = plans.first().expect("global loop compiles");
    assert!(plan.deferred_global_publication);
    assert_eq!(plan.sloppy_global_writes.len(), 2);
    assert!(
        !plan
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::StoreSloppyGlobal { .. }))
    );
    assert_eq!(eval(source), Ok(Value::Number(19020.0)));
    // A name-addressed read inside a function uses the same fallback register.
    assert_eq!(
        eval(
            "x = 0; function f() { var sum = 0; for (var j=0;j<20;j++) { x++; sum += x; } return sum; } f()*100 + globalThis.x;"
        ),
        Ok(Value::Number(21020.0))
    );
}

#[test]
fn global_descriptors_preserve_setters_and_failed_writes() {
    assert_eq!(
        eval(
            "x=0; var seen=0, backing=0; Object.defineProperty(globalThis,'x',{get:function(){return backing;},set:function(v){seen++;backing=v;},configurable:true}); for(var i=0;i<20;i++){x++;} [x,seen,globalThis.x].join();"
        ),
        Ok(Value::String("20,20,20".into()))
    );
    assert_eq!(
        eval(
            "var x=7; Object.defineProperty(globalThis,'x',{writable:false}); for(var i=0;i<20;i++){x=i;} [x,globalThis.x,i].join();"
        ),
        Ok(Value::String("7,7,20".into()))
    );
}

#[test]
fn global_shadowing_and_deletion_keep_live_binding_identity() {
    assert_eq!(
        eval(
            "x=3; function f(x){ for(var i=0;i<20;i++){x++;} return x; } [f(7),globalThis.x].join();"
        ),
        Ok(Value::String("27,3".into()))
    );
    assert_eq!(
        eval(
            "x=0; function f(){ for(var i=0;i<20;i++){x++;} return x; } var a=f(); delete globalThis.x; x=100; [a,f(),globalThis.x].join();"
        ),
        Ok(Value::String("20,120,120".into()))
    );
}

#[test]
fn global_calls_and_property_reads_observe_every_completed_store() {
    let source = "var x=0, sum=0; function read(){return x;} for(var i=0;i<20;i++){x++;sum+=read();} [x,sum].join();";
    assert!(
        programs(source)
            .iter()
            .all(|p| !p.deferred_global_publication)
    );
    assert_eq!(eval(source), Ok(Value::String("20,210".into())));
    // A numeric leaf that writes a captured realm cell must also decline.
    assert_eq!(
        eval(
            "var x=0,sum=0; function bump(){return x++;} for(var i=0;i<20;i++){x++;sum+=bump();} [x,sum].join();"
        ),
        Ok(Value::String("40,400".into()))
    );
    assert_eq!(
        eval(
            "var x=0,sum=0; var probe={get value(){return x;}}; for(var i=0;i<20;i++){x++;sum+=probe.value;} [x,sum].join();"
        ),
        Ok(Value::String("20,210".into()))
    );
    assert_eq!(
        eval("var x=0,sum=0; for(var i=0;i<20;i++){x++;sum+=globalThis.x+this.x;} [x,sum].join();"),
        Ok(Value::String("20,420".into()))
    );
    assert_eq!(
        eval(
            "var x=0; var alias=globalThis; for(var i=0;i<20;i++){x++;alias.x+=2;} [x,globalThis.x].join();"
        ),
        Ok(Value::String("60,60".into()))
    );
}

#[test]
fn global_pure_parameter_calls_publish_per_write() {
    let source = "var x=0; function add(n){return n+1;} for(var i=0;i<20;i++){x+=add(i);} [x,globalThis.x,i].join();";
    let plans = programs(source);
    let plan = plans.first().expect("numeric call loop compiles");
    assert!(!plan.deferred_global_publication);
    assert!(
        plan.ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::StoreSloppyGlobal { .. }))
    );
    assert_eq!(eval(source), Ok(Value::String("210,210,20".into())));
}

#[test]
fn global_break_and_residency_exits_publish_completed_stores() {
    assert_eq!(
        eval("var x=0; for(var i=0;i<20;i++){x+=i;if(i===7)break;} [x,globalThis.x,i].join();"),
        Ok(Value::String("28,28,7".into()))
    );
    let bound = 265;
    super::test_residency::with_limit(256, || {
        assert_eq!(
            eval(&format!(
                "var x=0; for(var i=0;i<{bound};i++){{x++;}} x+globalThis.x+i;"
            )),
            Ok(Value::Number((bound * 3) as f64))
        );
    });
}

#[test]
fn global_deoptimization_publishes_before_throw_and_coercion() {
    // A boxed BigInt fails the numeric guard after x++ completed. The
    // interpreter then throws at that same addition, seeing published x and
    // the exact operands. The separate JS counter also bounds replay bugs.
    let source = "var x=0,a=0,guard=0; try{for(var i=0;i<10 && guard++<40;i++){x++;a+=i===5?1n:1;}}catch(e){} [x,globalThis.x,i,a,guard].join();";
    assert!(
        programs(source)
            .iter()
            .any(|p| p.deferred_global_publication)
    );
    assert_eq!(eval(source), Ok(Value::String("6,6,5,5,6".into())));
    let source = "var x=0,a=0; for(var i=0;i<10;i++){x++;a+=i===5?'s':1;} [x,a].join();";
    assert!(
        programs(source)
            .iter()
            .any(|p| p.deferred_global_publication)
    );
    assert_eq!(eval(source), Ok(Value::String("10,5s1111".into())));
    assert_eq!(
        eval(
            "var x=0, a=0; try{for(var i=0;i<20;i++){x++;a+=i===5?'s':i===8?Symbol():1;}}catch(e){} [x,globalThis.x,i,a].join();"
        ),
        Ok(Value::String("9,9,8,5s11".into()))
    );
    assert_eq!(
        eval(
            "var x=0, a=0; var o={valueOf:function(){if(x!==6)throw 'stale';return 7;}}; for(var i=0;i<10;i++){x++;a+=i===5?o:1;} [x,a].join();"
        ),
        Ok(Value::String("10,16".into()))
    );
}

#[test]
fn global_direct_eval_and_try_loops_keep_interpreter_control_flow() {
    assert_eq!(
        eval(
            "var x=0; for(var i=0;i<20;i++){x++;eval('if(x!==i+1)throw 1');} [x,globalThis.x].join();"
        ),
        Ok(Value::String("20,20".into()))
    );
    assert_eq!(
        eval(
            "var x=0,guard=0; for(var i=0;i<20;i++){if(++guard>40)throw 'hang';try{x++;if(i===4)throw i;}catch(e){x+=10;}finally{x++;}} [x,guard,i].join();"
        ),
        Ok(Value::String("50,20,20".into()))
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn global_entry_proofs_take_the_fast_path_and_decline_observers() {
    use crate::diagnostics::{counters, reset_counters};
    for source in [
        "var x=0; for(var i=0;i<20;i++){x+=i;} x;",
        "var x=0; function add(n){return n+1;} for(var i=0;i<20;i++){x+=add(i);} x;",
    ] {
        reset_counters();
        eval(source).unwrap();
        assert_eq!(counters().unwrap().loop_plan_entries, 1, "{source}");
        assert_eq!(counters().unwrap().declined_loop_plan_edges, 0, "{source}");
    }
    for source in [
        "x=0; var b=0; Object.defineProperty(globalThis,'x',{get:function(){return b;},set:function(v){b=v;},configurable:true}); for(var i=0;i<20;i++){x++;} x;",
        "var x=0; Object.defineProperty(globalThis,'x',{writable:false}); for(var i=0;i<20;i++){x++;} x;",
        "var x=0; function read(){return x;} for(var i=0;i<20;i++){x+=read()+1;} x;",
        "var x=0; for(var i=0;i<20;i++){x++;eval('x');} x;",
    ] {
        reset_counters();
        eval(source).unwrap();
        assert_eq!(counters().unwrap().loop_plan_entries, 0, "{source}");
    }
}

#[test]
fn global_sink_calls_recheck_resolved_and_reassigned_callees() {
    assert_eq!(
        eval(
            "var x=0; var alias=globalThis,key='x'; for(var i=0;i<20;i++){x++;alias[key]+=2;} [x,globalThis.x].join();"
        ),
        Ok(Value::String("60,60".into()))
    );
    assert_eq!(
        eval(
            "var x=0,sum=0; var holder={bump:function(n){return ++x+n;}}; for(var i=0;i<20;i++){x++;sum+=holder.bump(i);} [x,sum].join();"
        ),
        Ok(Value::String("40,610".into()))
    );
    assert_eq!(
        eval(
            "x=0; function add(n){return n+1;} function bump(n){x++;return n+1;} function run(){var f=add,sum=0;for(var i=0;i<10;i++){x++;f=i===5?bump:add;sum+=f(i);}return sum;} [run(),x,globalThis.x].join();"
        ),
        Ok(Value::String("55,11,11".into()))
    );
}

#[test]
fn global_calls_preserve_four_packed_scalar_arguments() {
    assert_eq!(
        eval(
            "var s=0; function mix(a,b,c,d){return a+2*b+3*c+4*d;} for(var i=0;i<20;i++){s+=mix(i,i+1,i+2,i+3);} [s,globalThis.s,i].join();"
        ),
        Ok(Value::String("2300,2300,20".into()))
    );
}
