//! Declaration identities and positional routes, independent of live frames.

use super::*;
use crate::binding::{BindingKind, InitPolicy, ScopeKind};
use crate::bytecode::{binding_captures::CaptureRoute, compiler::compile_script};

fn compile(source: &str) -> Bytecode {
    compile_script(&qjs_parser::parse_script(source).unwrap()).unwrap()
}

fn functions(code: &Bytecode) -> Vec<&Bytecode> {
    code.code
        .iter()
        .filter_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn captures_follow_received_order_across_parent_local_and_received_cells() {
    let root = compile(
        "function outer(a) { let z = 1; return function middle(b) { return () => a + z + b; }; }",
    );
    let outer = functions(&root)[0];
    let middle = functions(outer)[0];
    let inner = functions(middle)[0];
    for (index, &slot) in middle.received_upvalue_slots().iter().enumerate() {
        let name = &middle.locals[slot].name;
        assert_eq!(
            middle.binding_plan.received[index],
            CaptureRoute::ParentLocal(crate::binding::LocalSlot(outer.local_slot(name).unwrap()))
        );
    }
    for (index, &slot) in inner.received_upvalue_slots().iter().enumerate() {
        let name = &inner.locals[slot].name;
        let parent = middle.local_slot(name).unwrap();
        let expected = middle.binding_plan.received_indices[parent].map_or(
            CaptureRoute::ParentLocal(crate::binding::LocalSlot(parent)),
            CaptureRoute::ParentUpvalue,
        );
        assert_eq!(inner.binding_plan.received[index], expected);
    }
}

#[test]
fn descriptors_keep_parameter_body_catch_loop_and_class_name_boundaries() {
    let root = compile(
        "function f(a = 1) { var x; let y; try {} catch (err) { let c; } for (let i = 0; i < 1; i++) { const v = i; } class C {} }",
    );
    let body = functions(&root)[0];
    let plan = &body.binding_plan;
    assert_eq!(plan.scopes[0].kind, ScopeKind::Parameters);
    assert_eq!(plan.scopes[1].kind, ScopeKind::FunctionBody);
    let parameter = plan.scopes[0].lookup["a"];
    assert_eq!(plan.bindings[parameter.0].kind, BindingKind::Parameter);
    let variable = plan.scopes[1].lookup["x"];
    assert_eq!(
        plan.bindings[variable.0].initialization,
        InitPolicy::Undefined
    );
    let lexical = plan.scopes[1].lookup["y"];
    assert_eq!(
        plan.bindings[lexical.0].initialization,
        InitPolicy::Uninitialized
    );
    for (kind, name) in [
        (ScopeKind::Catch, "err"),
        (ScopeKind::Loop, "i"),
        (ScopeKind::ClassName, "C"),
    ] {
        let scope = plan
            .scopes
            .iter()
            .find(|scope| scope.kind == kind && scope.lookup.contains_key(name))
            .unwrap();
        assert!(scope.parent.is_some());
        assert_ne!(scope.lookup[name], parameter);
    }
}

#[test]
fn closure_exposure_keeps_future_tdz_and_excludes_inactive_sibling_blocks() {
    let root = compile(
        "function f(value) { { let sibling = 1; } { let read = () => eval('future + value'); let future = 2; return read; } }",
    );
    let body = functions(&root)[0];
    assert!(body.has_dynamic_scope_tree());
    let site = body
        .binding_plan
        .exposure
        .sites
        .iter()
        .find(|site| matches!(body.code[site.pc], Op::NewFunction { .. }))
        .unwrap();
    let names: Vec<_> = site
        .scopes
        .iter()
        .flat_map(|scope| {
            body.binding_plan.scopes[scope.0]
                .lookup
                .keys()
                .map(String::as_str)
        })
        .collect();
    assert!(names.contains(&"value"));
    assert!(names.contains(&"future"));
    assert!(!names.contains(&"sibling"));
    let future = body
        .binding_plan
        .scopes
        .iter()
        .find_map(|scope| scope.lookup.get("future"))
        .unwrap();
    assert_eq!(
        body.binding_plan.bindings[future.0].initialization,
        InitPolicy::Uninitialized
    );
}

#[test]
fn transitive_eval_demand_includes_methods_fields_and_deferred_private_keys() {
    for source in [
        "function f(x) { return () => () => eval('x'); }",
        "function f(x) { return function () { return function () { return eval('x'); }; }; }",
        "function f(x) { return class { method() { return eval('x'); } }; }",
        "function f(x) { return class { field = eval('x'); }; }",
        "function f(x) { return class { #p; [(#p in {}, eval('x'))]() {} }; }",
    ] {
        let root = compile(source);
        let body = functions(&root)[0];
        assert!(body.has_dynamic_scope_tree(), "{source}");
        assert!(
            body.binding_plan
                .exposure
                .sites
                .iter()
                .any(|site| site.scopes.contains(&crate::binding::ScopeId(0))),
            "{source}"
        );
    }
    assert!(!functions(&compile("function f(x) { return () => x; }"))[0].has_dynamic_scope_tree());
}

#[test]
fn duplicate_parameter_routes_and_shadowed_declarations_keep_distinct_identities() {
    let root = compile("function f(a, a) { { let a; } return a; }");
    let body = functions(&root)[0];
    assert_eq!(body.parameter_slots().len(), 2);
    assert_eq!(body.parameter_slots()[0], body.parameter_slots()[1]);
    let bindings: Vec<_> = body
        .binding_plan
        .bindings
        .iter()
        .filter(|meta| meta.name == "a")
        .collect();
    assert_eq!(bindings.len(), 2);
    assert_ne!(bindings[0].id, bindings[1].id);
    assert_ne!(bindings[0].slot, bindings[1].slot);
}

#[test]
fn with_scope_keeps_its_variable_target_and_ordered_parent() {
    let root = compile("function f(x) { with ({}) { eval('var x = 2'); } }");
    let body = functions(&root)[0];
    let site = body
        .binding_plan
        .exposure
        .sites
        .iter()
        .find(|site| matches!(body.code[site.pc], Op::CallDirectEval { .. }))
        .unwrap();
    let scopes: Vec<_> = site
        .scopes
        .iter()
        .map(|id| &body.binding_plan.scopes[id.0])
        .collect();
    let with = scopes
        .iter()
        .find(|scope| scope.kind == ScopeKind::With)
        .unwrap();
    assert_eq!(with.variable_target, crate::binding::ScopeId(0));
    assert_eq!(with.parent, Some(crate::binding::ScopeId(1)));
    assert_eq!(scopes.last().unwrap().parent, Some(with.id));
    assert!(with.declarations.is_empty());
}

#[test]
fn simple_parameters_share_the_variable_environment_with_body_vars() {
    for (source, target) in [
        ("function f(a) { var b; }", 0),
        ("function f(a = 1) { var b; }", 1),
    ] {
        let root = compile(source);
        let body = functions(&root)[0];
        assert_eq!(body.binding_plan.scopes[1].variable_target.0, target);
        let variable = body.binding_plan.scopes[target].lookup["b"];
        assert_eq!(
            body.binding_plan.bindings[variable.0].kind,
            BindingKind::Var
        );
    }
}

#[test]
fn eval_demand_owns_arguments_in_the_parameter_descriptor() {
    let root = compile("function f(x){eval('arguments[0]=2');return x}f(1);");
    let body = functions(&root)[0];
    let id = body.binding_plan.scopes[0].lookup["arguments"];
    let meta = &body.binding_plan.bindings[id.0];
    assert_eq!(meta.kind, BindingKind::Arguments);
    assert!(!body.locals[meta.slot.0].from_env);
    assert!(matches!(
        body.binding_plan.initial_slots[meta.slot.0],
        InitialSlot::Arguments
    ));
}
