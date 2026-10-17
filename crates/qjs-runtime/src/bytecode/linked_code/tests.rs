use std::rc::Rc;

use qjs_parser::{parse_module, parse_script};

use super::{LinkedName, MAX_FOREIGN_LINKS, PropertyNameId};
use crate::Value;
use crate::bytecode::compile_script;
use crate::bytecode::ir::{Bytecode, Op};
use crate::bytecode::vm::Vm;
use crate::property::atom::AtomDomain;

fn compile(source: &str) -> Bytecode {
    compile_script(&parse_script(source).expect("parses")).expect("compiles")
}

fn named_operands(bytecode: &Bytecode) -> Vec<(Rc<str>, PropertyNameId)> {
    bytecode
        .code
        .iter()
        .filter_map(|op| match op {
            Op::GetPropNamed { key, name, .. } | Op::SetPropNamed { key, name, .. } => {
                Some((Rc::clone(key), *name))
            }
            _ => None,
        })
        .collect()
}

fn nested_function_bytecode(bytecode: &Bytecode) -> Rc<Bytecode> {
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(Rc::clone(bytecode)),
            _ => None,
        })
        .expect("a nested function")
}

/// Runs `source` as an independent root and returns its completion value and
/// how many atoms its domain created.
fn run_counting_atoms(source: &str) -> (Value, u64) {
    let bytecode = compile(source);
    let mut vm = Vm::new(&bytecode).expect("root frame");
    let value = vm.run().expect("runs");
    let created = vm.realm.atoms().created_count();
    (value, created)
}

#[test]
fn the_pool_holds_each_distinct_name_once() {
    let bytecode = compile("var o = {}, p = {}; o.a; o.b = 1; o.a = 2; p.a; p['c'];");
    let pool = bytecode.property_name_pool();
    let mut names: Vec<&str> = pool.iter().map(AsRef::as_ref).collect();
    names.sort_unstable();
    assert_eq!(names, ["a", "b"]);
    for (key, name) in named_operands(&bytecode) {
        assert_ne!(name, PropertyNameId::UNASSIGNED);
        assert_eq!(pool[name.index()].as_ref(), key.as_ref());
    }
}

#[test]
fn a_body_links_once_per_domain() {
    let bytecode = compile("var o = {}; o.x = 1; o.y = o.x;");
    let domain = AtomDomain::new();
    let operands = named_operands(&bytecode);
    for (key, name) in &operands {
        let atom = bytecode
            .linked_property_name(&domain, *name)
            .expect("links");
        assert!(matches!(atom, LinkedName::Primary(_)));
        assert!(atom.belongs_to(&domain));
        assert_eq!(atom.as_str(), key.as_ref());
    }
    let created = domain.created_count();
    assert_eq!(created, 2);
    for _ in 0..100 {
        for (_, name) in &operands {
            bytecode
                .linked_property_name(&domain, *name)
                .expect("links");
        }
    }
    assert_eq!(
        domain.created_count(),
        created,
        "dispatch must not re-intern"
    );
    assert_eq!(bytecode.linked_domain_count(), 1);
}

#[test]
fn a_foreign_domain_reinterns_by_content() {
    let bytecode = compile("var o = {}; o.x = 1; o.x;");
    let (_, name) = named_operands(&bytecode)[0].clone();
    let home = AtomDomain::new();
    let foreign = AtomDomain::new();
    let home_atom = bytecode
        .linked_property_name(&home, name)
        .expect("links")
        .clone();
    let foreign_atom = bytecode
        .linked_property_name(&foreign, name)
        .expect("links");
    assert!(matches!(foreign_atom, LinkedName::Foreign(_)));
    assert!(foreign_atom.belongs_to(&foreign));
    assert!(!foreign_atom.belongs_to(&home));
    assert_ne!(*foreign_atom, home_atom);
    assert_eq!(foreign_atom.as_str(), "x");
    drop(foreign_atom);
    // Both links stay cached: neither domain interns again.
    for _ in 0..10 {
        assert!(
            bytecode
                .linked_property_name(&home, name)
                .expect("links")
                .belongs_to(&home)
        );
        assert!(
            bytecode
                .linked_property_name(&foreign, name)
                .expect("links")
                .belongs_to(&foreign)
        );
    }
    assert_eq!(home.created_count(), 1);
    assert_eq!(foreign.created_count(), 1);
    assert_eq!(bytecode.linked_domain_count(), 2);
}

#[test]
fn a_dead_domain_is_never_matched_and_its_links_are_bounded() {
    let bytecode = compile("var o = {}; o.x;");
    let (_, name) = named_operands(&bytecode)[0].clone();
    {
        let first = AtomDomain::new();
        bytecode.linked_property_name(&first, name).expect("links");
    }
    for _ in 0..(3 * MAX_FOREIGN_LINKS) {
        let later = AtomDomain::new();
        let atom = bytecode.linked_property_name(&later, name).expect("links");
        assert!(atom.belongs_to(&later));
        assert_eq!(later.created_count(), 1);
    }
    assert!(bytecode.linked_domain_count() <= 1 + MAX_FOREIGN_LINKS);
}

#[test]
fn function_and_module_bodies_carry_their_own_pools() {
    let script = compile("function f(o) { return o.inner; } f({ inner: 1 }).outer;");
    let nested = nested_function_bytecode(&script);
    assert_eq!(
        script
            .property_name_pool()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["outer"]
    );
    assert_eq!(
        nested
            .property_name_pool()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["inner"]
    );
    let module = crate::bytecode::compile_module(
        &parse_module("const o = { m: 1 }; const v = o.m;").expect("parses"),
    )
    .expect("compiles");
    assert_eq!(
        module
            .property_name_pool()
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["m"]
    );
}

#[test]
fn steady_state_named_access_creates_no_atoms() {
    let source = |iterations: u32| {
        format!(
            "var o = {{ a: 1 }}; var total = 0;
             for (var i = 0; i < {iterations}; i++) {{ o.b = o.a + i; total += o.b; }}
             total;"
        )
    };
    let (_, few) = run_counting_atoms(&source(2));
    let (_, many) = run_counting_atoms(&source(500));
    assert_eq!(few, many);
}

#[test]
fn repeated_eval_and_function_bodies_reuse_their_links() {
    let eval_source = |iterations: u32| {
        format!(
            "var o = {{ p: 1, q: 2 }}; var total = 0;
             for (var i = 0; i < {iterations}; i++) total += eval('o.p + o.q');
             total;"
        )
    };
    let (value, few) = run_counting_atoms(&eval_source(2));
    assert_eq!(value, Value::Number(6.0));
    let (value, many) = run_counting_atoms(&eval_source(200));
    assert_eq!(value, Value::Number(600.0));
    assert_eq!(few, many);

    let function_source = |iterations: u32| {
        format!(
            "var f = new Function('o', 'o.seen = o.base + 1; return o.seen;');
             var o = {{ base: 1 }}; var total = 0;
             for (var i = 0; i < {iterations}; i++) total += f(o);
             total;"
        )
    };
    let (value, few) = run_counting_atoms(&function_source(2));
    assert_eq!(value, Value::Number(4.0));
    let (value, many) = run_counting_atoms(&function_source(200));
    assert_eq!(value, Value::Number(400.0));
    assert_eq!(few, many);
}

#[test]
fn one_body_runs_under_two_independent_roots() {
    let bytecode = compile("var o = {}; o.k = 20; o.k + 1;");
    let first_result = {
        let mut vm = Vm::new(&bytecode).expect("first root");
        vm.run().expect("runs")
    };
    let mut second = Vm::new(&bytecode).expect("second root");
    assert_eq!(second.run().expect("runs"), first_result);
    assert_eq!(first_result, Value::Number(21.0));
    let (_, name) = named_operands(&bytecode)[0].clone();
    assert!(
        bytecode
            .linked_property_name(second.realm.atoms(), name)
            .expect("links")
            .belongs_to(second.realm.atoms())
    );
}

#[test]
fn a_dynamic_function_shares_its_creators_domain() {
    let bytecode = compile("new Function('o', 'return o.shared;');");
    let mut vm = Vm::new(&bytecode).expect("root frame");
    let Value::Function(created) = vm.run().expect("runs") else {
        panic!("expected a function");
    };
    let creation_realm = created.realm.as_ref().expect("a user function has a realm");
    assert!(
        !Rc::ptr_eq(creation_realm, &vm.realm),
        "dynamic functions get a scope realm"
    );
    assert!(creation_realm.atoms().ptr_eq(vm.realm.atoms()));
}
