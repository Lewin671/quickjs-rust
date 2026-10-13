use crate::{Value, eval};

fn eval_with_prepared_calls(source: &str) -> (Result<Value, crate::RuntimeError>, usize) {
    crate::bytecode::reset_prepared_readonly_eval_calls();
    let result = eval(source);
    (result, crate::bytecode::prepared_readonly_eval_calls())
}

#[test]
fn cached_reads_use_current_cells_and_keep_callers_independent() {
    let (result, calls) = eval_with_prepared_calls(
        "function read(value) { eval('value'); value += 1; return eval('value'); }
         [read(3), read(8)].join(',');",
    );
    assert_eq!(result, Ok(Value::String("4,9".into())));
    assert_eq!(
        calls, 3,
        "first source/context call is cold, subsequent calls are prepared"
    );
}

#[test]
fn cached_reads_preserve_callback_effects_and_exception_refresh() {
    let (result, calls) = eval_with_prepared_calls(
        "var realmValue = 0;
         function read() {
           var local = 0;
           function update() { local++; realmValue++; return local; }
           eval('update()'); eval('update()');
           function fail() { local++; realmValue++; throw 7; }
           try { eval('fail()'); } catch (e) { if (e !== 7) throw e; }
           try { eval('fail()'); } catch (e) { if (e !== 7) throw e; }
           return local + ':' + realmValue;
         }
         read();",
    );
    assert_eq!(result, Ok(Value::String("4:4".into())));
    assert_eq!(
        calls, 2,
        "both warmed sources enter, including the throwing call"
    );
}

#[test]
fn cached_reads_keep_tdz_and_strict_writeback_on_the_original_paths() {
    let (result, calls) = eval_with_prepared_calls(
        "function read() {
           'use strict';
           var failures = 0;
           try { eval('value'); } catch (e) { if (!(e instanceof ReferenceError)) throw e; failures++; }
           try { eval('value'); } catch (e) { if (!(e instanceof ReferenceError)) throw e; failures++; }
           let value = 3;
           var before = eval('value');
           eval('value += 1'); eval('value += 1');
           return failures + ':' + before + ':' + value;
         }
         read();",
    );
    assert_eq!(result, Ok(Value::String("2:3:5".into())));
    assert_eq!(
        calls, 2,
        "read-only TDZ throw and initialized read enter; writes never do"
    );
}

#[test]
fn cached_reads_preserve_prior_eval_names_and_extra_argument_order() {
    let (result, calls) = eval_with_prepared_calls(
        "function read() {
           eval('var introduced = 11');
           var effects = 0;
           var first = eval('introduced', effects++);
           var second = eval('introduced', effects++);
           return first + ':' + second + ':' + effects;
         }
         read();",
    );
    assert_eq!(result, Ok(Value::String("11:11:2".into())));
    assert_eq!(calls, 1);
}

#[test]
fn cached_eval_fallbacks_are_observed_for_writes_and_dynamic_scopes() {
    for source in [
        "function f() { eval('var x = 1'); return eval('var x = 1'); } f();",
        "function f() { var x = 0; eval('x += 1'); return eval('x += 1'); } f();",
        "function f() { var x = 0; eval('delete x'); return eval('delete x'); } f();",
        "function f() { eval('eval(\"var x = 1\")'); return eval('eval(\"var x = 1\")'); } f();",
        "function f() { with ({x: 3}) { eval('x'); return eval('x'); } } f();",
        "function f(x = eval('this.k')) { return x; } f.call({k: 3}); f.call({k: 4});",
        "var object = { toString() { throw 1; } }; eval(object); eval(object) === object;",
        "(0, eval)('1 + 2'); (0, eval)('1 + 2');",
        "var original = eval; function f() { var eval = original.bind(null); eval('1 + 2'); return eval('1 + 2'); } f();",
        "var original = eval; function f() { var eval = new Proxy(original, {}); eval('1 + 2'); return eval('1 + 2'); } f();",
        "function f() { var eval = function (source) { return source; }; eval('1 + 2'); return eval('1 + 2'); } f();",
    ] {
        let (result, calls) = eval_with_prepared_calls(source);
        assert!(
            result.is_ok(),
            "fallback source failed: {source}: {result:?}"
        );
        assert_eq!(
            calls, 0,
            "unsupported call entered prepared execution: {source}"
        );
    }
}

#[test]
fn cached_reads_keep_the_intrinsic_after_global_eval_is_rebound() {
    let (result, calls) = eval_with_prepared_calls(
        "var saved = eval;
         function read() { var eval = saved; eval('3 + 4'); return eval('3 + 4'); }
         var first = read();
         globalThis.eval = function () { return 99; };
         [first, read(), globalThis.eval()].join(',');",
    );
    assert_eq!(result, Ok(Value::String("7,7,99".into())));
    assert_eq!(calls, 3);
}

#[test]
fn cached_reads_decline_a_dynamic_realm_override() {
    let (result, calls) = eval_with_prepared_calls(
        "var __quickjsRustDynamicFunctionRealm;
         function read() { return eval('3 + 4'); }
         var first = read();
         __quickjsRustDynamicFunctionRealm = globalThis;
         var second = read();
         __quickjsRustDynamicFunctionRealm = undefined;
         first + second;",
    );
    assert_eq!(result, Ok(Value::Number(14.0)));
    assert_eq!(
        calls, 0,
        "the cache hit under a realm override must fall back"
    );
}

#[test]
fn cached_reads_keep_private_super_new_target_and_arguments_context() {
    let (result, calls) = eval_with_prepared_calls(
        "class Parent { method() { return 4; } }
         class Child extends Parent {
           #value = 7;
           read(arg) {
             eval('this.#value + super.method() + arguments[0]');
             return eval('this.#value + super.method() + arguments[0]');
           }
         }
         function Target() { eval('new.target'); this.target = eval('new.target'); }
         var made = new Target();
         new Child().read(3) + ':' + (made.target === Target);",
    );
    assert_eq!(result, Ok(Value::String("14:true".into())));
    assert_eq!(
        calls, 2,
        "each context has its own cold call followed by a prepared call"
    );
}

#[test]
fn cached_reads_keep_live_module_imports() {
    crate::bytecode::reset_prepared_readonly_eval_calls();
    let resolver = crate::MapResolver::new().with(
        "dep",
        "export let imported = 3; export function update() { imported = 8; }",
    );
    let namespace = crate::eval_module(
        "import { imported, update } from 'dep';
         function read() { eval('imported'); return eval('imported'); }
         var first = read(); update();
         export const answer = first + ':' + read();",
        "main",
        Box::new(resolver),
    )
    .expect("module eval should preserve imported cells");
    let Value::Object(namespace) = namespace else {
        panic!("module namespace must be an object");
    };
    assert_eq!(
        namespace
            .own_property("answer")
            .map(|property| property.value),
        Some(Value::String("3:8".into()))
    );
    assert_eq!(crate::bytecode::prepared_readonly_eval_calls(), 3);
}

#[test]
fn cached_callbacks_can_initialize_derived_this_and_preserve_abrupt_effects() {
    let (result, calls) = eval_with_prepared_calls(
        "var constructions = 0;
         class Parent { constructor() { constructions++; this.value = 4; } }
         class Child extends Parent {
           constructor() {
             var construct = () => super();
             eval('construct()');
             var repeated = false;
             try { eval('construct()'); } catch (e) { repeated = e instanceof ReferenceError; }
             this.repeated = repeated;
           }
         }
         var first = new Child(), second = new Child();
         [first !== second, first instanceof Child, second instanceof Child,
          first === globalThis, first.value, first.repeated,
          second.value, second.repeated, constructions].join(',');",
    );
    assert_eq!(
        result,
        Ok(Value::String("true,true,true,false,4,true,4,true,4".into()))
    );
    assert_eq!(
        calls, 2,
        "after native callbacks bind this, warmed repeated callbacks enter prepared execution"
    );

    let (result, calls) = eval_with_prepared_calls(
        "var constructions = 0;
         class Parent { constructor() { constructions++; throw 7; } }
         class Child extends Parent {
           constructor() {
             var construct = () => super();
             try { eval('construct()'); } catch (e) { if (e !== 7) throw e; }
             try { eval('construct()'); } catch (e) { if (e !== 7) throw e; }
             var uninitialized = false;
             try { this; } catch (e) { uninitialized = e instanceof ReferenceError; }
             return { constructions: constructions, uninitialized: uninitialized };
           }
         }
         var first = new Child(), second = new Child();
         [first.constructions, first.uninitialized,
          second.constructions, second.uninitialized].join(',');",
    );
    assert_eq!(result, Ok(Value::String("2,true,4,true".into())));
    assert_eq!(
        calls, 0,
        "failed construction leaves this uninitialized, so even warmed callbacks retain native execution"
    );
}

#[test]
fn uninitialized_derived_context_declines_without_early_throwing() {
    let (result, calls) = eval_with_prepared_calls(
        "class Parent {} class Child extends Parent {
           constructor() {
             var total = eval('1 + 2') + eval('1 + 2');
             super(); this.total = total;
           }
         }
         [new Child().total, new Child().total].join(',');",
    );
    assert_eq!(result, Ok(Value::String("6,6".into())));
    assert_eq!(
        calls, 0,
        "before-super context must decline even an unrelated warmed source"
    );

    // The native route's handling of eval('this') before super is a separate
    // conformance concern. This test proves the new route does not take it
    // over or eagerly throw while deciding eligibility.
    let (result, calls) = eval_with_prepared_calls(
        "class Parent {} class Child extends Parent {
           constructor() {
             try { eval('this'); } catch (e) {}
             try { eval('this'); } catch (e) {}
             super();
           }
         }
         new Child(); new Child(); true;",
    );
    assert_eq!(result, Ok(Value::Boolean(true)));
    assert_eq!(calls, 0);
}

#[test]
fn cached_super_construction_keeps_implicit_this_writes_on_native_execution() {
    let (result, calls) = eval_with_prepared_calls(
        "var constructions = 0;
         class Parent { constructor() { constructions++; throw 7; } }
         class Child extends Parent {
           constructor() {
             try { eval('super()'); } catch (e) { if (e !== 7) throw e; }
             try { eval('super(...[])'); } catch (e) { if (e !== 7) throw e; }
             try { eval('super()'); } catch (e) { if (e !== 7) throw e; }
             try { eval('super(...[])'); } catch (e) { if (e !== 7) throw e; }
             var uninitialized = false;
             try { this; } catch (e) { uninitialized = e instanceof ReferenceError; }
             return { constructions: constructions, uninitialized: uninitialized };
           }
         }
         var first = new Child(), second = new Child();
         [first.constructions, first.uninitialized,
          second.constructions, second.uninitialized].join(',');",
    );
    assert_eq!(result, Ok(Value::String("4,true,8,true".into())));
    assert_eq!(
        calls, 0,
        "both warmed implicit-this writes must decline prepared execution"
    );
}

#[test]
fn captured_arrow_does_not_make_uninitialized_constructor_this_ready() {
    let (result, calls) = eval_with_prepared_calls(
        "class Parent {} class Child extends Parent {
           constructor() {
             var x = 2, captured = () => this, results = [], guard = 0;
             try { eval('this'); } catch (e) {}
             for (var i = 0; i < 4; i++) {
               if (++guard > 4) throw 99;
               results.push(eval('x + 1'));
             }
             return { results: results.join(','), captured: captured };
           }
         }
         var made = new Child();
         var uninitialized = false;
         try { made.captured(); } catch (e) { uninitialized = e instanceof ReferenceError; }
         made.results + ':' + uninitialized;",
    );
    assert_eq!(result, Ok(Value::String("3,3,3,3:true".into())));
    assert_eq!(
        calls, 0,
        "captured this still has a TDZ before super despite any slot placeholder"
    );
}

#[test]
fn captured_this_before_super_native_probe_never_enters_prepared_execution() {
    let (result, calls) = eval_with_prepared_calls(
        "var observations=[], constructions=0;
         class B { constructor(){constructions++;this.x=4;} }
         class D extends B {
           constructor(){
             var captured=()=>this;
             try{observations.push('evalThis:'+typeof eval('this'));}
             catch(e){observations.push('evalThis:'+e.name);}
             try{eval('super()');observations.push('evalSuper:ok');}
             catch(e){observations.push('evalSuper:'+e.name);}
             try{observations.push('captured:'+captured().x);}
             catch(e){observations.push('captured:'+e.name);}
             return {};
           }
         }
         new D(); new D(); observations.join('|')+';'+constructions;",
    );
    // This deliberately retains the native chain; its existing conformance
    // discrepancy is owned by T043. Here only admission is under test.
    assert!(result.is_ok());
    assert_eq!(calls, 0);
}

#[test]
fn captured_constructor_this_allows_cached_eval_only_after_super_binds_it() {
    let (result, calls) = eval_with_prepared_calls(
        "class Parent {} class Child extends Parent {
           constructor() {
             var x = 2, captured = () => this, results = [], guard = 0;
             try { eval('this'); } catch (e) {}
             for (var i = 0; i < 4; i++) {
               if (++guard > 4) throw 99;
               results.push(eval('x + 1'));
             }
             super();
             for (var j = 0; j < 4; j++) {
               if (++guard > 8) throw 99;
               results.push(eval('x + 1'));
             }
             return { results: results.join(','), captured: captured };
           }
         }
         var made = new Child();
         made.results + ':' + (made.captured() instanceof Child);",
    );
    assert_eq!(result, Ok(Value::String("3,3,3,3,3,3,3,3:true".into())));
    assert_eq!(
        calls, 4,
        "the identical already-warmed source enters only after this is an object"
    );
}

#[test]
fn cached_eval_creates_fresh_objects_and_regexp_instances() {
    let (result, calls) = eval_with_prepared_calls(
        "function make() { eval('({})'); return eval('({})'); }
         var first = make(), second = make();
         function makeRegex() { eval('[/x/g][0]'); return eval('[/x/g][0]'); }
         var preparedFirst = makeRegex(), preparedSecond = makeRegex();
         var r1 = eval('/x/g'), r2 = eval('/x/g');
         (first !== second) + ':' + (preparedFirst !== preparedSecond) + ':' + (r1 !== r2);",
    );
    assert_eq!(result, Ok(Value::String("true:true:true".into())));
    assert_eq!(
        calls, 6,
        "objects and compound regexp sources are prepared; literal early route stays fresh"
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn prepared_eval_keeps_native_call_attribution() {
    crate::diagnostics::reset_counters();
    let (result, calls) = eval_with_prepared_calls(
        "function read() { eval('1 + 2'); return eval('1 + 2'); } read();",
    );
    assert_eq!(result, Ok(Value::Number(3.0)));
    assert_eq!(calls, 1);
    let counters = crate::diagnostics::counters().expect("counter build");
    assert_eq!(counters.native_calls, 2);
    assert_eq!(counters.prepared_readonly_eval_calls, 1);
    assert_eq!(counters.ordinary_call_attempts, 1);
}

/// A cached direct eval that writes and deletes no binding skips the
/// write-back to its caller's frame. What it reads is still current, a
/// function it calls still updates the caller's captured variables and the
/// realm, and an eval that does assign still writes back.
#[test]
fn a_read_only_direct_eval_leaves_its_caller_current() {
    assert_eq!(
        eval(
            "var out = [];
             function f(k) {
               var n = 1, plain = 10;
               function g() { n++; return n; }
               function h() { zz = (typeof zz === 'number' ? zz : 0) + k; return zz; }
               function del() { return plain; }
               for (var i = 0; i < 3; i++) out.push(eval('g()'), n, eval('h()'), eval('plain + n'));
               eval('plain = 20'); out.push(plain);
               out.push(eval('del()'));
               return n;
             }
             out.push(f(1), f(2), zz);
             var gv = 1; function r() { return eval('gv'); }
             out.push(r()); gv = 7; out.push(r(), eval('gv'));
             out.join(',');"
        ),
        Ok(Value::String(
            "2,2,1,12,3,3,2,13,4,4,3,14,20,20,2,2,5,12,3,3,7,13,4,4,9,14,20,20,4,4,9,1,7,7"
                .to_owned()
                .into()
        ))
    );
}

/// A function literal in eval code that resolves nothing in the eval's
/// scope is created without it; one that does, or that could see a later
/// declaration, keeps it.
#[test]
fn eval_created_closures_resolve_the_same_names_with_or_without_the_eval_scope() {
    assert_eq!(
        eval(
            "var out = [];
             function outer(a) { var local = 'L'; eval('var g = function () { return local + a + typeof String; }'); return g(); }
             function own() { eval('var y = 3; var g = function () { return y; }; y = 4;'); return g(); }
             function receiver() { eval('var g = function () { return this === globalThis ? \"G\" : this.v; }'); return g() + g.call({ v: 'o' }); }
             function args() { eval('var g = function () { return arguments.length + \":\" + arguments[0]; }'); return g(7, 8); }
             function later() { eval('var g = function () { return typeof q; }; eval(\"var q = 1\");'); return g(); }
             function writes() { var x = 1; eval('var g = function () { x = 2; return x; }'); return g() + x; }
             function nested() { eval('var g = (function () { return function () { return String.name; }; })();'); return g(); }
             for (var i = 0; i < 3; i++) {
                 out.push(outer('A'), own(), receiver(), args(), later(), writes(), nested());
             }
             out.join();"
        ),
        Ok(Value::String(
            "LAfunction,4,Go,2:7,number,4,String,"
                .repeat(3)
                .trim_end_matches(',')
                .into()
        ))
    );
}

/// A closure created in a function that has a direct `eval` bypasses that
/// function's dynamic scope only until the scope binds a new name: a later
/// `eval('var String = 5')` must then be visible to it. A generator's frame
/// keeps the scope throughout, and a frame creating many closures stops
/// bypassing past its limit.
#[test]
fn closures_see_names_a_later_eval_adds_to_their_function() {
    assert_eq!(
        eval(
            "var out = [];
             function later() { var g = function () { return typeof String; }; var r = g(); eval('var String = 5'); return r + ' ' + g(); }
             function hoisted() { function g() { return typeof zz; } var r = g(); eval('var zz = 1'); return r + ' ' + g(); }
             function captured() { var self = 3; function d() { return self + 1; } return eval('d()') + eval('d()'); }
             function* suspended() { var g = function () { return typeof yy; }; yield g(); eval('var yy = 1'); yield g(); }
             function many() { var fns = []; for (var i = 0; i < 70; i++) fns.push(function () { return typeof Math; }); eval('var Math = 1'); return fns[0]() + ' ' + fns[69](); }
             function walkTree(filter) {
                 function walk(k, v) { var i; if (v && typeof v === 'object') { for (i in v) { var n = walk(i, v[i]); if (n !== undefined) v[i] = n; } } return filter(k, v); }
                 return JSON.stringify(walk('', eval('({a: [1, 2, {b: 3}], c: 4})')));
             }
             for (var i = 0; i < 2; i++) {
                 var it = suspended();
                 out.push(later(), hoisted(), captured(), it.next().value + ' ' + it.next().value, many(),
                          walkTree(function (k, v) { return typeof v === 'number' ? v * 10 : v; }));
             }
             out.join('|');"
        ),
        Ok(Value::String(
            "function number|undefined number|8|undefined number|number number|{\"a\":[10,20,{\"b\":30}],\"c\":40}|"
                .repeat(2)
                .trim_end_matches('|')
                .into()
        ))
    );
}
