//! Prove admission exclusions and shared-cell behavior, not just opcode shape.
use super::{compiler, ir::Op};
use crate::{Value, eval};

fn closure(source: &str) -> crate::Function {
    let Value::Function(function) = eval(source).expect("valid program") else {
        panic!("program must return a closure");
    };
    function
}

#[test]
fn initialized_mutable_requires_static_provenance_at_link() {
    for source in [
        "function make(){var x=0;return function(){x++;return x}} make()",
        "function make(x){return function(){x++;return x}} make(0)",
        "function make(){var x=0;return function(){return function(){x++;return x}}} make()()",
    ] {
        let f = closure(source);
        assert_ne!(*f.initialized_mutable_upvalues(), 0, "{source}");
    }
    for source in [
        "function make(){let x=0;return function(){x++;return x}} make()",
        "function make(){const x=0;return function(){x++;return x}} make()",
        "function make(x=0){return function(){x++;return x}} make()",
        "function make(){var x=0;eval('');return function(){x++;return x}} make()",
        "function make(){var x=0;return function(){eval('');x++;return x}} make()",
        "function make(){var x=0;with({}){return function(){x++;return x}}} make()",
        "var x=0; (function(){x++;return x})",
        "function make(){var f=0;return function f(){f=1;return f}} make()",
    ] {
        let f = closure(source);
        assert_eq!(*f.initialized_mutable_upvalues(), 0, "{source}");
    }
    let ast =
        qjs_parser::parse_script("function make(){var x=0;return function(){x=1}}").expect("parse");
    let script = compiler::compile_script(&ast).expect("compile");
    let Op::NewFunction { bytecode: make, .. } = script
        .code
        .iter()
        .find(|op| matches!(op, Op::NewFunction { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    let Op::NewFunction {
        bytecode: inner, ..
    } = make
        .code
        .iter()
        .find(|op| matches!(op, Op::NewFunction { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    // Same name and initialized value cannot bless a dynamically created cell.
    let cell = crate::binding::BindingCell::with_metadata(
        crate::binding::BindingMetaRef::dynamic("x".into(), true, false),
        Some(Value::Number(0.0)),
    );
    assert_eq!(inner.link_write_capabilities(&[cell]), 0);
}

#[test]
fn proven_capture_writes_preserve_aliases_resources_and_checked_errors() {
    assert_eq!(eval("function make(x){var a=arguments;return [function(v){x=v},a]} var p=make(1);p[0]({n:9});p[0](7);p[1][0]").unwrap(), Value::Number(7.0));
    assert_eq!(eval("function make(){var x={n:1};return [function(v){x=v},function(){return x}]} var p=make();p[0](4);p[0]({n:8});p[1]().n").unwrap(), Value::Number(8.0));
    assert_eq!(eval("function make(){var x=0;return [function(){x++},function(){return x}]} var p=make();for(var i=0;i<40;i++)p[0]();p[1]()").unwrap(), Value::Number(40.0));
    assert!(
        eval("function f(a=(()=>{b=1})(),b){} f()")
            .unwrap_err()
            .message
            .contains("ReferenceError")
    );
    assert!(
        eval("function make(){const x=1;return function(){x=2}} make()()")
            .unwrap_err()
            .message
            .contains("TypeError")
    );
    assert_eq!(
        eval(
            "var f;function make(){f=function(){x=2};try{f()}catch(e){return e.name}let x=1}make()"
        )
        .unwrap(),
        Value::String("ReferenceError".into())
    );
}
