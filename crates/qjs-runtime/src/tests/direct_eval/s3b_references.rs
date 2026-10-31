//! Assignment selects its environment before RHS effects (ES2025 13.15.2).
//! Several engines currently re-resolve these names at PutValue; these tests
//! follow the selected-base contract rather than copying that disagreement.
use crate::{Value, eval};

fn result(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())), "{source}");
}

#[test]
fn assignment_retains_outer_base_before_a_closer_eval_declaration() {
    result(
        "(function(){let x=1;function inner(){x=(eval('var x=2'),3);return x}return inner()+':'+x})()",
        "2:3",
    );
    result(
        "(function(){let x=1;function inner(){x+=(eval('var x=2'),3);return x}return inner()+':'+x})()",
        "2:4",
    );
    result(
        "(function(){const x=1;function inner(){let error='none';try{x=(eval('var x=2'),3)}catch(e){error=e.name}return error+':'+x}return inner()+':'+x})()",
        "TypeError:2:1",
    );
}

#[test]
fn assignment_keeps_deleted_holder_and_recreates_in_that_holder() {
    result(
        "(function(){eval('var x=1');function inner(){eval('var x=2');x=(eval('delete x'),9);return x}return inner()+':'+x})()",
        "9:1",
    );
}

#[test]
fn originally_missing_strict_assignment_stays_unresolvable() {
    result(
        "(function(){'use strict';try{s3bMissing=(globalThis.s3bMissing=1,2);return 'none:'+s3bMissing}catch(e){return e.name+':'+s3bMissing}})()",
        "ReferenceError:1",
    );
}

#[test]
fn suspended_assignment_preserves_original_missing_reference() {
    result(
        "(function(){function* f(){'use strict';try{s3bAfterYield=yield 1;return 'none'}catch(e){return e.name}}const it=f();it.next();Object.defineProperty(globalThis,'s3bAfterYield',{value:4,writable:true});return it.next(8).value+':'+s3bAfterYield})()",
        "ReferenceError:4",
    );
}

#[test]
fn update_keeps_parent_holder_across_numeric_conversion_callback() {
    result(
        "(function(){let x=100,it;function* p(){eval('var x={valueOf(){it.next();return 1}}');yield function(){x++;return x};eval('delete x')}it=p();const f=it.next().value;return f()+':'+x})()",
        "2:100",
    );
}

#[test]
fn destructuring_selects_target_before_source_getter_or_iterator() {
    result(
        "(function(){let x=1;function f(){({value:x=(eval('var x=2'),3)}={});return x}return f()+':'+x})()",
        "2:3",
    );
    result(
        "(function(){let x=1;function f(){[x=(eval('var x=2'),3)]=[];return x}return f()+':'+x})()",
        "2:3",
    );
}

#[test]
fn reference_scope_precedes_rhs_class_name_scope() {
    result(
        "(function(){let x=1;function f(){x=class x{[(eval('0'),'method')](){}};return typeof x}return f()+':'+typeof x})()",
        "function:function",
    );
}

#[test]
fn skipped_logical_assignment_releases_reference_before_loop_reentry() {
    result(
        "(function(){let x=1;function f(){let output='';for(let i=0;i<3;i++){x||=(eval('var x=9'),4);output+=x;if(i===0)eval('var x=0')}return output}return f()+':'+x})()",
        "144:1",
    );
}

#[test]
fn abrupt_rhs_releases_reference_before_catch_and_reentry() {
    result(
        "(function(){let x=1;function f(){for(let i=0;i<2;i++){try{x=(i===0?(()=>{throw 1})():(eval('var x=2'),3))}catch(e){eval('var x=8')}}return x}return f()+':'+x})()",
        "3:1",
    );
}

#[test]
fn with_call_receiver_and_reference_lifetimes_match_ng() {
    result(
        include_str!("../../../../../tests/fixtures/compare-qjs/s3b-scope-reference-lifetime.js"),
        "144:3:1:true",
    );
}
