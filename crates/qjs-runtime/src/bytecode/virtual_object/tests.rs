use std::rc::Rc;

use qjs_parser::parse_script;

use super::super::{
    compile_script,
    ir::{Bytecode, Op},
};
use super::flow::analyze;
use super::*;

fn compile(source: &str) -> Bytecode {
    let script = parse_script(source).expect("test source should parse");
    compile_script(&script).expect("test source should compile")
}

fn named_function(source: &str, expected_name: &str) -> Rc<Bytecode> {
    fn find(bytecode: &Bytecode, expected_name: &str) -> Option<Rc<Bytecode>> {
        bytecode.code.iter().find_map(|op| match op {
            Op::NewFunction {
                name: Some(name),
                bytecode,
                ..
            } if name == expected_name => Some(bytecode.clone()),
            Op::NewFunction { bytecode, .. } => find(bytecode, expected_name),
            _ => None,
        })
    }
    let bytecode = compile(source);
    find(&bytecode, expected_name).expect("named function bytecode should exist")
}

fn object_candidates(analysis: &VirtualObjectAnalysis) -> Vec<&VirtualCandidate> {
    analysis
        .candidates
        .iter()
        .filter(|candidate| matches!(candidate.kind, VirtualKind::Object(_)))
        .collect()
}

fn dense_array_candidates(analysis: &VirtualObjectAnalysis) -> Vec<&VirtualCandidate> {
    analysis
        .candidates
        .iter()
        .filter(|candidate| matches!(candidate.kind, VirtualKind::DenseArray { .. }))
        .collect()
}

#[test]
fn cfg_builds_if_loop_edges_without_instruction_offsets() {
    let bytecode = named_function(
        r#"
            function flow(flag, count) {
                var total = 0;
                while (count > 0) {
                    if (flag) total += count * 2;
                    else total -= count - 1;
                    count--;
                }
                return total;
            }
            "#,
        "flow",
    );
    let cfg = ControlFlowGraph::build(&bytecode.code).expect("valid CFG");

    assert!(cfg.blocks.len() >= 6);
    assert!(cfg.blocks.iter().any(|block| block.successors.len() == 2));
    assert!(cfg.blocks.iter().any(|block| {
        block
            .successors
            .iter()
            .any(|successor| cfg.blocks[*successor].start <= block.start)
    }));
}

#[test]
fn follows_one_literal_through_loop_if_and_local_aliases() {
    let bytecode = named_function(
        r#"
            function projected(flag, count) {
                var total = 0;
                while (count > 0) {
                    var point = { x: count * 2, y: count - 1 };
                    var alias = point;
                    if (flag) total += alias.x;
                    else total += alias.y;
                    count--;
                }
                return total;
            }
            "#,
        "projected",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert!(analysis.complete);
    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable());
    assert!(
        objects[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::Alias { .. }))
    );
    assert_eq!(
        objects[0]
            .uses
            .iter()
            .filter(|use_kind| matches!(use_kind, VirtualUse::FieldRead { .. }))
            .count(),
        2
    );
}

#[test]
fn join_with_non_literal_alias_fails_closed() {
    let bytecode = named_function(
        r#"
            function joined(flag, other) {
                var point;
                if (flag) point = { x: 1, y: 2 };
                else point = other;
                return point.x;
            }
            "#,
        "joined",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert!(analysis.complete);
    assert_eq!(objects.len(), 1);
    assert!(!objects[0].is_virtualizable());
    assert!(
        objects[0]
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::AmbiguousAlias { .. }))
    );
}

#[test]
fn return_and_call_identity_uses_escape() {
    for source in [
        r#"function escaped() { var point = { x: 1 }; return point; }"#,
        r#"function escaped(sink) { var point = { x: 1 }; sink(point); return 0; }"#,
    ] {
        let bytecode = named_function(source, "escaped");
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert_eq!(objects.len(), 1);
        assert!(!objects[0].is_virtualizable());
        assert!(
            objects[0]
                .escape_reasons
                .iter()
                .any(|reason| matches!(reason, EscapeReason::IdentityUse { .. }))
        );
    }
}

#[test]
fn dynamic_eval_and_with_reject_candidates() {
    for source in [
        r#"function dynamic() { eval(""); var point = { x: 1 }; return point.x; }"#,
        r#"function dynamic(scope) { with (scope) { var point = { x: 1 }; return point.x; } }"#,
    ] {
        let bytecode = named_function(source, "dynamic");
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert_eq!(objects.len(), 1);
        assert!(!objects[0].is_virtualizable());
        assert!(
            objects[0]
                .escape_reasons
                .contains(&EscapeReason::DynamicScope)
        );
    }
}

#[test]
fn captured_and_parameter_slots_are_not_authoritative() {
    let captured = named_function(
        r#"
            function captured() {
                var point = { x: 1 };
                return function read() { return point.x; };
            }
            "#,
        "captured",
    );
    let captured_analysis = analyze(&captured);
    let captured_objects = object_candidates(&captured_analysis);
    assert_eq!(captured_objects.len(), 1);
    assert!(!captured_objects[0].is_virtualizable());
    assert!(
        captured_objects[0]
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::UnsafeSlot { .. }))
    );

    let parameter = named_function(
        r#"
            function mapped(point) {
                point = { x: 1 };
                return arguments[0];
            }
            "#,
        "mapped",
    );
    let parameter_analysis = analyze(&parameter);
    let parameter_objects = object_candidates(&parameter_analysis);
    let point_slot = parameter.local_slot("point").expect("point slot");
    assert!(
        !parameter_analysis
            .slot_authority
            .is_authoritative(point_slot)
    );
    assert_eq!(parameter_objects.len(), 1);
    assert!(!parameter_objects[0].is_virtualizable());
    assert!(
        parameter_objects[0]
            .escape_reasons
            .contains(&EscapeReason::UnsafeSlot {
                ip: parameter_objects[0]
                    .escape_reasons
                    .iter()
                    .find_map(|reason| match reason {
                        EscapeReason::UnsafeSlot { ip, slot } if *slot == point_slot => Some(*ip),
                        _ => None,
                    })
                    .expect("parameter store should be rejected"),
                slot: point_slot,
            })
    );
}

#[test]
fn duplicate_keys_project_the_last_input() {
    let shape = ObjectLiteralShape::new(vec![Rc::from("x"), Rc::from("y"), Rc::from("x")]);
    assert_eq!(shape.final_input_index("x"), Some(2));
    assert_eq!(shape.final_input_index("y"), Some(1));
    assert_eq!(shape.final_input_index("missing"), None);

    let bytecode = named_function(
        r#"function duplicate() { var point = { x: 1, y: 2, x: 3 }; return point.x; }"#,
        "duplicate",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);
    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable());
    assert!(
        objects[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::FieldRead { input: 2, .. }))
    );
}

#[test]
fn kraken_freqz_shape_is_a_general_read_write_candidate() {
    // Reduced from Kraken 1.1 DSP.freqz: the literal/update/read topology
    // is preserved, while corpus data and benchmark harness code are not
    // copied into this non-benchmark unit fixture.
    let bytecode = named_function(
        r#"
            function freqz(b, a, w, cos, sin, sqrt) {
                var result = [];
                for (var i = 0; i < w.length; i++) {
                    var numerator = { real: 0.0, imag: 0.0 };
                    for (var j = 0; j < b.length; j++) {
                        numerator.real += b[j] * cos(-j * w[i]);
                        numerator.imag += b[j] * sin(-j * w[i]);
                    }
                    var denominator = { real: 0.0, imag: 0.0 };
                    for (var k = 0; k < a.length; k++) {
                        denominator.real += a[k] * cos(-k * w[i]);
                        denominator.imag += a[k] * sin(-k * w[i]);
                    }
                    result[i] = sqrt(
                        numerator.real * numerator.real + numerator.imag * numerator.imag
                    ) / sqrt(
                        denominator.real * denominator.real + denominator.imag * denominator.imag
                    );
                }
                return result;
            }
            "#,
        "freqz",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert!(analysis.complete);
    assert_eq!(objects.len(), 2);
    for candidate in objects {
        assert!(candidate.is_virtualizable(), "{candidate:#?}");
        assert!(
            candidate
                .uses
                .iter()
                .any(|use_kind| matches!(use_kind, VirtualUse::FieldWrite { .. }))
        );
        assert!(
            candidate
                .uses
                .iter()
                .any(|use_kind| matches!(use_kind, VirtualUse::FieldRead { .. }))
        );
    }
}

#[test]
fn named_store_rhs_escapes_for_unknown_receiver_and_self_cycle() {
    let unknown_receiver = named_function(
        r#"
            function storeIntoUnknown(target) {
                var point = { x: 1 };
                target.value = point;
                return 0;
            }
            "#,
        "storeIntoUnknown",
    );
    assert!(
        unknown_receiver
            .code
            .iter()
            .any(|op| matches!(op, Op::SetPropNamed { .. }))
    );
    let analysis = analyze(&unknown_receiver);
    let objects = object_candidates(&analysis);
    assert_eq!(objects.len(), 1);
    assert!(
        objects[0]
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::StoredInAggregate { .. }))
    );

    let self_cycle = named_function(
        r#"
            function selfCycle() {
                var point = { self: 0 };
                point.self = point;
                return 0;
            }
            "#,
        "selfCycle",
    );
    let analysis = analyze(&self_cycle);
    let objects = object_candidates(&analysis);
    assert_eq!(objects.len(), 1);
    assert!(
        objects[0]
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::StoredInAggregate { .. }))
    );
}

#[test]
fn named_store_with_candidate_receiver_tracks_write_without_receiver_escape() {
    let bytecode = named_function(
        r#"
            function writeExternal(value) {
                var point = { x: 0 };
                point.x = value;
                return 1;
            }
            "#,
        "writeExternal",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable(), "{:#?}", objects[0]);
    assert!(
        objects[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::FieldWrite { .. }))
    );

    let bytecode = named_function(
        r#"
            function storeCandidate() {
                var target = { value: 0 };
                var payload = { x: 1 };
                target.value = payload;
                return 1;
            }
            "#,
        "storeCandidate",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);
    assert_eq!(objects.len(), 2);
    assert!(objects[0].is_virtualizable(), "{:#?}", objects[0]);
    assert!(
        objects[1]
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::StoredInAggregate { .. }))
    );
}

#[test]
fn field_reads_preserve_possible_home_function_effects() {
    let bytecode = named_function(
        r#"
            function propagateHomeFunction() {
                var source = { method: function () {} };
                var target = { method: source.method };
                return target.method === source.method;
            }
            "#,
        "propagateHomeFunction",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert_eq!(objects.len(), 2);
    assert!(objects.iter().all(|candidate| {
        candidate
            .escape_reasons
            .iter()
            .any(|reason| matches!(reason, EscapeReason::HomeObjectSideEffect { .. }))
    }));
}

#[test]
fn append_string_ops_push_results_and_preserve_exact_local_keys() {
    let local = named_function(
        r#"
            function localAppend() {
                var key = "x";
                key += "";
                var point = { x: 1 };
                return point[key];
            }
            "#,
        "localAppend",
    );
    assert!(
        local
            .code
            .iter()
            .any(|op| matches!(op, Op::AppendStringLiteralLocal { .. }))
    );
    let analysis = analyze(&local);
    let objects = object_candidates(&analysis);
    assert!(analysis.complete);
    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable(), "{:#?}", objects[0]);
    assert!(
        objects[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::FieldRead { .. }))
    );

    let global = named_function(
        r#"
            function globalAppend() {
                externalKey += "x";
                var point = { x: 1 };
                return point.x;
            }
            "#,
        "globalAppend",
    );
    assert!(
        global
            .code
            .iter()
            .any(|op| matches!(op, Op::AppendStringLiteralGlobal { .. }))
    );
    let analysis = analyze(&global);
    let objects = object_candidates(&analysis);
    assert!(analysis.complete);
    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable(), "{:#?}", objects[0]);
}

#[test]
fn nested_direct_eval_deopts_outer_slot_authority() {
    let bytecode = named_function(
        r#"
            function outer() {
                var point = { x: 1 };
                function inner() { eval("point = { x: 2 }"); }
                return point.x;
            }
            "#,
        "outer",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);

    assert!(analysis.slot_authority.dynamic_scope);
    assert_eq!(objects.len(), 1);
    assert!(
        objects[0]
            .escape_reasons
            .contains(&EscapeReason::DynamicScope)
    );
}

#[test]
fn assign_local_rejects_immutable_and_tdz_sensitive_slots() {
    for (source, name) in [
        (
            r#"function immutable() { const point = 0; point = { x: 1 }; return 0; }"#,
            "immutable",
        ),
        (
            r#"function beforeInit() { point = { x: 1 }; let point; return 0; }"#,
            "beforeInit",
        ),
    ] {
        let bytecode = named_function(source, name);
        let analysis = analyze(&bytecode);
        let objects = object_candidates(&analysis);

        assert_eq!(objects.len(), 1);
        assert!(
            objects[0]
                .escape_reasons
                .iter()
                .any(|reason| matches!(reason, EscapeReason::UnsafeSlot { .. }))
        );
    }

    let bytecode = named_function(
        r#"function mutableVar() { var point; point = { x: 1 }; return point.x; }"#,
        "mutableVar",
    );
    let analysis = analyze(&bytecode);
    let objects = object_candidates(&analysis);
    assert_eq!(objects.len(), 1);
    assert!(objects[0].is_virtualizable(), "{:#?}", objects[0]);
}

#[test]
fn indexed_store_contract_matches_pending_set_prop_index_stack_semantics() {
    let bytecode = named_function(
        r#"
            function indexed(value) {
                var array = [0];
                array["0"] = value;
                return array[0];
            }
            "#,
        "indexed",
    );
    let analysis = analyze(&bytecode);
    let arrays = dense_array_candidates(&analysis);

    assert!(analysis.complete);
    assert_eq!(arrays.len(), 1);
    assert!(arrays[0].is_virtualizable(), "{:#?}", arrays[0]);
    assert!(
        arrays[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::ElementWrite { index: 0, .. }))
    );
    assert!(
        arrays[0]
            .uses
            .iter()
            .any(|use_kind| matches!(use_kind, VirtualUse::ElementRead { index: 0, .. }))
    );
}
