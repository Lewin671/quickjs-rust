//! Declaration instantiation and capture boundaries of lexical statement scopes.
use crate::{Value, eval};

#[test]
fn try_and_finally_instantiate_function_declarations_before_statements() {
    for strict in ["", "'use strict';"] {
        let source = format!(
            "{strict}var seen=[];function t(){{try{{return f();function f(){{return 7;}}}} \
            finally{{seen.push(g());function g(){{return 8;}}}}}}t()+':'+seen.join();"
        );
        assert_eq!(eval(&source), Ok(Value::String("7:8".into())), "{source}");
    }
    assert_eq!(
        eval("var f;try{function f(){return 1;}}finally{}f();"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'use strict';try{function f(){return 1;}}finally{}typeof f;"),
        Ok(Value::String("undefined".into()))
    );
}

#[test]
fn inner_block_shadows_do_not_hide_captures_outside_the_block() {
    for shadow in [
        "{let z='block';}",
        "{const z='block';}",
        "for(let z=0;z<1;z++){}",
        "try{throw 0;}catch(z){}",
    ] {
        for write in [false, true] {
            let use_z = if write { "z=1;return z;" } else { "return z;" };
            let source = format!(
                "function outer(){{var z='outer';function inner(){{{shadow}{use_z}}}return inner()+':'+z;}}outer();"
            );
            let expected = if write { "1:1" } else { "outer:outer" };
            assert_eq!(
                eval(&source),
                Ok(Value::String(expected.into())),
                "{source}"
            );
        }
    }
}

#[test]
fn named_expression_self_scope_precedes_an_outer_homonym_despite_inner_shadow() {
    assert_eq!(
        eval("var nf='outer';(function nf(){{let nf='block';}return typeof nf;})()"),
        Ok(Value::String("function".into()))
    );
}

#[test]
fn global_identifier_lookup_reaches_inherited_data_and_getters() {
    assert_eq!(
        eval("Object.prototype.protoG='fromProto';protoG+':'+typeof protoG;"),
        Ok(Value::String("fromProto:string".into()))
    );
    for read in [
        "protoG",
        "(function(){return protoG;})()",
        "(function nf(){nf;return protoG;})()",
        "eval('protoG')",
    ] {
        let source = format!(
            "var reads=0;Object.defineProperty(Object.prototype,'protoG',{{configurable:true,get(){{reads++;return this===globalThis?42:-1;}}}});var r={read};r+':'+reads;"
        );
        assert_eq!(eval(&source), Ok(Value::String("42:1".into())), "{source}");
    }
}

#[test]
fn inherited_global_getters_are_not_probed_or_folded_by_loop_plans() {
    assert_eq!(
        eval(
            "var reads=0;Object.defineProperty(Object.prototype,'protoG',{get(){reads++;return 1;}}); \
        function run(){function h(x){return protoG+x;}var s=0;for(var i=0;i<100;i++)s+=h(i);return s;} \
        run()+':'+reads;"
        ),
        Ok(Value::String("5050:100".into()))
    );
}

#[test]
fn with_tagged_and_optional_calls_preserve_the_selected_receiver() {
    for call in ["f`x`", "f?.()", "f?.(...[])"] {
        let source = format!(
            "var o={{v:7,f:function(){{return this===o?this.v:-1;}}}},r;with(o){{r={call};}}r;"
        );
        assert_eq!(eval(&source), Ok(Value::Number(7.0)), "{source}");
        let source = format!(
            "var o={{v:7,f:function(){{return this.v;}}}},g;with(o){{g=function(){{return {call};}};}}g();"
        );
        assert_eq!(eval(&source), Ok(Value::Number(7.0)), "{source}");
    }
    assert_eq!(
        eval("var n=0,r;with({f:null}){r=f?.(n++);}typeof r+':'+n;"),
        Ok(Value::String("undefined:0".into()))
    );
    assert_eq!(
        eval("var o={f(){return this===o;}};with(o){f?.(delete o.f);}"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn labeled_continue_reenters_nested_lexicals_in_tdz() {
    for loop_head in ["outer:for(var i=0;i<2;i++)", "var i=0;outer:while(i++<2)"] {
        let source = format!(
            "var out=[];{loop_head}{{{{try{{out.push(x);}}catch(e){{out.push(e.name);}}let x=1;continue outer;}}}}out.join(':');"
        );
        assert_eq!(
            eval(&source),
            Ok(Value::String("ReferenceError:ReferenceError".into())),
            "{source}"
        );
    }
    assert_eq!(
        eval(
            "var f=[];outer:for(var i=0;i<3;i++){{let x=i;f.push(()=>x);continue outer;}}f.map(g=>g()).join();"
        ),
        Ok(Value::String("0,1,2".into()))
    );
}

#[test]
fn abrupt_reentry_preserves_finally_reads_and_escaped_tdz_instances() {
    assert_eq!(
        eval(
            "var out=[];outer:for(var i=0;i<2;i++){let x=i;try{continue outer;}finally{out.push(x);}}out.join();"
        ),
        Ok(Value::String("0,1".into()))
    );
    assert_eq!(
        eval(
            "var fs=[];outer:for(var i=0;i<2;i++){{fs.push(()=>x);if(i===0)continue outer;let x=i;}}var r;try{fs[0]();}catch(e){r=e.name;}r+':'+fs[1]();"
        ),
        Ok(Value::String("ReferenceError:1".into()))
    );
}

#[test]
fn separate_scripts_reject_lexical_redeclaration_over_an_independent_property() {
    for declaration in ["let p=2;", "const p=2;", "class p{}"] {
        let source = format!(
            "globalThis.p=1;__quickjsRustEvalScript('let p=10;');var r; \
            try{{__quickjsRustEvalScript('var side;{declaration}');r='missed';}}catch(e){{r=e.name;}} \
            r+':'+p+':'+globalThis.p+':'+Object.prototype.hasOwnProperty.call(globalThis,'side');"
        );
        assert_eq!(
            eval(&source),
            Ok(Value::String("SyntaxError:10:1:false".into())),
            "{source}"
        );
    }
}
