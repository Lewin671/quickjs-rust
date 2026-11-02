//! Relocation proof and observable selected-base/scope behavior.
use super::{
    compiler,
    ir::{Bytecode, Op},
};
use crate::{Value, eval};

fn body(source: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).unwrap();
    compiler::compile_script(&script)
        .unwrap()
        .code
        .iter()
        .find_map(|op| {
            if let Op::NewFunction { bytecode, .. } = op {
                Some(bytecode.as_ref().clone())
            } else {
                None
            }
        })
        .unwrap()
}

#[test]
fn empty_joined_scopes_do_not_obstruct_a_private_numeric_region() {
    let code = body("function f(a,n){var s=0;for(var i=0;i<n;i++){s+=a[i];}return s}");
    assert!(
        !code
            .code
            .iter()
            .any(|op| matches!(op, Op::BindingBoundary { .. }))
    );
    assert!(code.binding_plan.pc_scopes.iter().all(|id| id.0 == 0));
}

#[test]
fn every_pc_owner_is_relocated_before_a_tier_can_build_its_maps() {
    let code = body(
        "async function f(a=eval('1')){let n=0;try{for(let i=0;i<2;i++){missing=(await i,n++);}}catch(e){return n;}finally{n=3;}return n;}",
    );
    let plan = &code.binding_plan;
    assert_eq!(plan.pc_scopes.len(), code.code.len());
    assert!(matches!(
        code.code[plan.prologue_end.unwrap()],
        Op::FunctionPrologueEnd
    ));
    assert!(!plan.exposure.sites.is_empty());
    for site in &plan.exposure.sites {
        assert!(matches!(
            code.code[site.pc],
            Op::CallDirectEval { .. } | Op::CallDirectEvalSpread { .. }
        ));
    }
    for (index, event) in plan.references.events.iter().enumerate() {
        assert!(
            matches!(code.code[event.pc - 1], Op::BindingBoundary { event: Some(id), .. } if id == index)
        );
        assert!(!matches!(code.code[event.pc], Op::BindingBoundary { .. }));
    }
    assert!(!plan.references.lifetimes.is_empty());
    for life in &plan.references.lifetimes {
        assert!(matches!(code.code[life.start], Op::BindingBoundary { .. }));
        assert!(matches!(code.code[life.end], Op::BindingBoundary { .. }));
    }
    for (pc, op) in code.code.iter().enumerate() {
        match op {
            Op::Jump(target)
            | Op::JumpIfFalse(target)
            | Op::JumpIfTrue(target)
            | Op::JumpIfNotNullish(target)
            | Op::AbruptJump(target) => {
                let leaves = plan.references.lifetimes.iter().any(|life| {
                    pc >= life.start
                        && pc < life.end
                        && !(*target >= life.start && *target < life.end)
                });
                if plan.pc_scopes[pc] != plan.pc_scopes[*target] || leaves {
                    assert!(matches!(code.code[*target], Op::BindingBoundary { .. }));
                }
            }
            Op::EnterTry { catch, finally, .. } => {
                for target in catch.iter().chain(finally.iter()) {
                    assert!(matches!(code.code[*target], Op::BindingBoundary { .. }));
                }
            }
            Op::Yield | Op::Await | Op::YieldDelegate { .. } => {
                assert!(matches!(code.code[pc + 1], Op::BindingBoundary { .. }));
            }
            _ => {}
        }
    }
    assert!(code.compact_wide_program.get().is_none());
    assert!(code.virtual_object_program.get().is_none());
}

#[test]
fn a_tier_releases_a_selected_reference_at_its_explicit_end() {
    let code = body("function f(){__boundaryMissing=1}");
    let plan = &code.binding_plan.references;
    let event = plan
        .events
        .iter()
        .position(|event| !event.starts.is_empty())
        .unwrap();
    let life = &plan.lifetimes[plan.sites[plan.events[event].starts[0]].lifetime];
    let mut root = crate::context::HostRoot::script();
    let mut context = root.context();
    plan.select_checked_starts(event, &mut context).unwrap();
    assert!(context.activation.has_saved_references());
    plan.cleanup_checked(life.start, &mut context);
    assert!(context.activation.has_saved_references());
    plan.cleanup_checked(life.end, &mut context);
    assert!(!context.activation.has_saved_references());
}

#[test]
fn a_global_reference_keeps_its_selected_base_across_yield() {
    assert_eq!(
        eval(
            "var ES=__quickjsRustEvalScript;function* g(){__boundaryName=(yield 1,ES('let __boundaryName=0;'),9);}var it=g();it.next();it.next();''+globalThis.__boundaryName+':'+__boundaryName"
        ),
        Ok(Value::String("9:0".into()))
    );
    assert_eq!(
        eval(
            "function f(){var a=[];for(let x=0;x<2;x++){a.push(()=>x);}return a.map(g=>g()).join(',');}f()"
        ),
        Ok(Value::String("0,1".into()))
    );
}
