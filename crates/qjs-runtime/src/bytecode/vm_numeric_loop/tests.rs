use super::*;
use crate::bytecode::compiler;
use crate::{Property, Value, eval};

fn reset_loop_counters() {
    NUMERIC_LOOP_ENTRY_HITS.with(|hits| hits.set(0));
    REALM_GLOBAL_LOOP_BATCH_COMMITS.with(|commits| commits.set(0));
}

fn loop_counters() -> (usize, usize) {
    (
        NUMERIC_LOOP_ENTRY_HITS.with(Cell::get),
        REALM_GLOBAL_LOOP_BATCH_COMMITS.with(Cell::get),
    )
}

fn nested_function(source: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("function bytecode should be nested in the script")
}

fn top_level_plan_count(source: &str) -> usize {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    NumericLoopPlan::compile_all(&bytecode).len()
}

#[test]
fn top_level_var_loop_batches_realm_writes_once() {
    reset_loop_counters();
    assert_eq!(
        eval(
            "function addOne(value) { return value + 1; } \
             var limit = 1000; var checksum = 0; \
             for (var index = 0; index < limit; index++) checksum += addOne(index); \
             checksum + ':' + index + ':' + globalThis.checksum + ':' + \
               Object.getOwnPropertyNames(globalThis).filter(function (name) { \
                 return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
               }).length;"
        ),
        Ok(Value::String("500500:1000:500500:0".to_owned().into()))
    );
    assert_eq!(loop_counters(), (1, 1));
}

#[test]
fn top_level_loop_plan_classifies_global_and_private_writes() {
    let script = qjs_parser::parse_script(
        "function addOne(value) { return value + 1; } var limit = 4, checksum = 0; for (var index = 0; index < limit; index++) checksum += addOne(index);",
    )
    .expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    let plans = NumericLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:?}", bytecode.code);
    assert!(matches!(
        plans[0].counter_write,
        NumericLoopWrite::RealmGlobal { .. }
    ));
    assert!(matches!(
        plans[0].accumulator_write,
        NumericLoopWrite::RealmGlobal { .. }
    ));
    assert!(bytecode.local_is_compiler_temporary(plans[0].block_result_slot));
    assert!(plans[0].loop_result_slot.is_none());
    assert!(
        bytecode
            .hoisted_local_names()
            .all(|name| !name.starts_with("\0\0"))
    );
    let vm = Vm::new(&bytecode).expect("top-level VM should initialize");
    for (slot, local) in bytecode.locals.iter().enumerate() {
        if !local.compiler_temporary {
            continue;
        }
        assert!(vm.slot_is_authoritative(slot));
        assert!(vm.local_upvalues.get(slot).is_none_or(Option::is_none));
        assert_eq!(vm.locals[slot], Some(Value::Undefined));
    }
}

#[test]
fn top_level_loop_rejects_aliasing_captured_global_reads() {
    reset_loop_counters();
    let source = "var limit = 4, sum = 0; function addCurrent(value) { return value + sum; } \
         for (var index = 0; index < limit; index++) sum += addCurrent(index); \
         sum + ':' + index;";
    assert_eq!(top_level_plan_count(source), 1);
    assert_eq!(eval(source), Ok(Value::String("11:4".to_owned().into())));
    assert_eq!(loop_counters(), (0, 0));
}

#[test]
fn top_level_loop_rejects_global_object_property_aliases() {
    for source in [
        "var mirror = globalThis, limit = 4, sum = 0; for (var index = 0; index < limit; index++) sum += mirror.index; sum + ':' + index;",
        "var mirror = globalThis, key = 'index', limit = 4, sum = 0; for (var index = 0; index < limit; index++) sum += mirror[key]; sum + ':' + index;",
    ] {
        reset_loop_counters();
        assert_eq!(top_level_plan_count(source), 1, "{source}");
        assert_eq!(
            eval(source),
            Ok(Value::String("6:4".to_owned().into())),
            "{source}"
        );
        assert_eq!(loop_counters(), (0, 0), "{source}");
    }
}

#[test]
fn top_level_loop_rejects_mutable_slot_aliases_at_compile_time() {
    for source in [
        "var sum = 0; for (var index = 0; index < index; index++) sum += 1;",
        "function addOne(value) { return value + 1; } var limit = 4; for (var index = 0; index < limit; index++) limit += addOne(index);",
        "function addOne(value) { return value + 1; } var sum = 0; for (var index = 0; index < 4; index++) sum += sum;",
    ] {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        assert!(
            NumericLoopPlan::compile_all(&bytecode).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn top_level_loop_descriptor_and_dynamic_scope_guards_fail_closed() {
    for (source, expected, expect_candidate) in [
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Object.defineProperty(globalThis, 'sum', { writable: false }); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "0:4",
            true,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; eval('sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
            false,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; (0, eval)('sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
            true,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; eval.call(undefined, 'sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
            true,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Reflect.apply(eval, undefined, ['sum = 5']); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
            true,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Function('sum = 5')(); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
            true,
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; with ({}) {} for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "10:4",
            false,
        ),
    ] {
        reset_loop_counters();
        if expect_candidate {
            assert_eq!(top_level_plan_count(source), 1, "{source}");
        }
        assert_eq!(
            eval(source),
            Ok(Value::String(expected.to_owned().into())),
            "{source}"
        );
        assert_eq!(loop_counters(), (0, 0), "{source}");
    }
}

#[test]
fn eval_loops_never_publish_compiler_temporaries() {
    let scratch_count = "Object.getOwnPropertyNames(globalThis).filter(function (name) { \
        return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
    }).length";
    for eval_call in [
        "eval('var evalTotal = 0; for (var evalIndex = 0; evalIndex < 4; evalIndex++) evalTotal += evalIndex;')",
        "(0, eval)('var evalTotal = 0; for (var evalIndex = 0; evalIndex < 4; evalIndex++) evalTotal += evalIndex;')",
    ] {
        let source = format!("{eval_call}; evalTotal + ':' + ({scratch_count});");
        assert_eq!(
            eval(&source),
            Ok(Value::String("6:0".to_owned().into())),
            "{source}"
        );
    }
}

#[test]
fn direct_function_eval_loop_does_not_write_back_compiler_temporaries() {
    let source = "function run() { \
        var total = 10; \
        eval('for (var inner = 0; inner < 4; inner++) total += inner;'); \
        for (var outer = 0; outer < 3; outer++) total += outer; \
        return total; \
    } \
    run() + ':' + Object.getOwnPropertyNames(globalThis).filter(function (name) { \
        return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
    }).length;";
    assert_eq!(eval(source), Ok(Value::String("19:0".to_owned().into())));
}

#[test]
fn top_level_loop_accepts_value_only_global_redefinition() {
    reset_loop_counters();
    assert_eq!(
        eval(
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; \
             Object.defineProperty(globalThis, 'sum', { value: 5 }); \
             for (var index = 0; index < limit; index++) sum += addOne(index); \
             sum + ':' + globalThis.sum;"
        ),
        Ok(Value::String("15:15".to_owned().into()))
    );
    assert_eq!(loop_counters(), (1, 1));
}

#[test]
fn realm_global_loop_commit_revalidates_all_targets_before_mutation() {
    let script = qjs_parser::parse_script("var first = 1, second = 2; first + second;")
        .expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    let mut vm = Vm::new(&bytecode).expect("script VM should initialize");
    assert_eq!(vm.run(), Ok(Value::Number(3.0)));

    let first_slot = bytecode
        .local_slot("first")
        .expect("first should have a slot");
    let second_slot = bytecode
        .local_slot("second")
        .expect("second should have a slot");
    let first = vm
        .prepare_realm_global_loop_write(first_slot, "first")
        .expect("first should initially be writable");
    let second = vm
        .prepare_realm_global_loop_write(second_slot, "second")
        .expect("second should initially be writable");
    let first_cell = first.cell();
    let first_local_before = vm.locals[first_slot].clone();
    let global_this = vm.cached_global_this().expect("global object should exist");
    global_this.define_property(
        "second".to_owned(),
        Property::data(Value::Number(2.0), true, false, false),
    );

    assert!(!vm.commit_realm_global_loop_writes(&[(first, 10.0), (second, 20.0)]));
    assert_eq!(
        global_this
            .own_property("first")
            .map(|property| property.value),
        Some(Value::Number(1.0))
    );
    assert_eq!(first_cell.get(), Value::Number(1.0));
    assert_eq!(vm.locals[first_slot], first_local_before);
}

#[test]
fn top_level_sloppy_accessor_and_delete_paths_keep_observable_execution() {
    reset_loop_counters();
    let source = "var limit = 4, reads = 0; \
         Object.defineProperty(globalThis, 'sloppySum', { configurable: true, \
           get: function () { reads += 1; return 1; } }); \
         for (var index = 0; index < limit; index++) sloppySum += index; \
         delete globalThis.sloppySum; sloppySum = 9; \
         reads + ':' + sloppySum;";
    // An undeclared sloppy global has no indexed realm-binding slot, so
    // this observable accessor/delete shape must fail closed at compile
    // time rather than reach the transactional runtime guards.
    assert_eq!(top_level_plan_count(source), 0);
    assert_eq!(eval(source), Ok(Value::String("4:9".to_owned().into())));
    assert_eq!(loop_counters(), (0, 0));
}

#[test]
fn top_level_zero_and_non_numeric_limits_keep_the_existing_path() {
    for (limit, expected) in [("0", "0:0"), ("'3'", "6:3"), ("NaN", "0:0")] {
        reset_loop_counters();
        let source = format!(
            "var sum = 0; for (var index = 0; index < {limit}; index++) sum += index + 1; sum + ':' + index;"
        );
        assert_eq!(
            eval(&source),
            Ok(Value::String(expected.to_owned().into())),
            "{source}"
        );
        assert_eq!(loop_counters(), (0, 0), "{source}");
    }
}

#[test]
fn recognizes_named_property_accumulation_loop() {
    let bytecode = nested_function(
        "function sum(n) { var o = { a: 1, b: 2 }; var s = 0; for (var i = 0; i < n; i++) { s += o.a; s += o.b; } return s; }",
    );
    let plans = NumericLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].terms.len(), 2);
}

#[test]
fn recognizes_stable_local_read_accumulation_loop() {
    let bytecode = nested_function(
        "function sum(n) { var first = 1, second = 2, s = 0; for (var i = 0; i < n; i++) { s += first; s += second; } return s; }",
    );
    let plans = NumericLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].terms.len(), 2);
    assert!(
        plans[0]
            .terms
            .iter()
            .all(|term| matches!(term, NumericLoopTerm::LocalRead { .. }))
    );
}

#[test]
fn recognizes_stable_global_read_accumulation_loop() {
    let source = "var value = 2; function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += value; } return s; }";
    let bytecode = nested_function(source);
    let plans = NumericLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1);
    let [NumericLoopTerm::LocalRead { slot }] = plans[0].terms.as_slice() else {
        panic!("read-only global should compile as a realm-cell local read");
    };
    assert!(bytecode.local_is_from_env(*slot));
    assert_eq!(eval(&format!("{source} sum(4);")), Ok(Value::Number(8.0)));
}

#[test]
fn rejects_mutating_local_read_terms() {
    for source in [
        "function sum(n) { var s = 1; for (var i = 0; i < n; i++) { s += s; } return s; }",
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += i; } return s; }",
    ] {
        let bytecode = nested_function(source);
        assert!(
            NumericLoopPlan::compile_all(&bytecode).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn recognizes_dense_array_accumulation_loop() {
    let bytecode = nested_function(
        "function sum(n) { var a = [1, 2, 3]; var s = 0; for (var i = 0; i < n; i++) { s += a[0]; s += a[1]; s += a[2]; } return s; }",
    );
    let plans = NumericLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].terms.len(), 3);
}

#[test]
fn recognizes_computed_object_and_array_accumulation_loops() {
    for (source, term_count) in [
        (
            "function sum(n) { var o = { a: 1, b: 2 }; var x = 'a', y = 'b'; var s = 0; for (var i = 0; i < n; i++) { s += o[x]; s += o[y]; } return s; }",
            2,
        ),
        (
            "function sum(n) { var a = [1, 2, 3]; var x = 0, y = 1, z = 2; var s = 0; for (var i = 0; i < n; i++) { s += a[x]; s += a[y]; s += a[z]; } return s; }",
            3,
        ),
    ] {
        let bytecode = nested_function(source);
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{source}");
        assert_eq!(plans[0].terms.len(), term_count, "{source}");
    }
}

#[test]
fn computed_accessors_keep_the_observable_loop_path() {
    assert_eq!(
        eval(
            "function run(n) { var reads = 0, o = {}, key = 'a', sum = 0; Object.defineProperty(o, 'a', { get: function () { reads += 1; return 2; } }); for (var i = 0; i < n; i++) { sum += o[key]; } return sum + ':' + reads; } run(4);"
        ),
        Ok(Value::String("8:4".to_owned().into()))
    );
}

#[test]
fn rejects_computed_keys_mutated_by_the_loop() {
    for source in [
        "function sum(n) { var a = [1, 2, 3]; var s = 0; for (var i = 0; i < n; i++) { s += a[i]; } return s; }",
        "function sum(n) { var o = { 0: 1, 1: 2 }; var s = 0; for (var i = 0; i < n; i++) { s += o[s]; } return s; }",
    ] {
        let bytecode = nested_function(source);
        assert!(
            NumericLoopPlan::compile_all(&bytecode).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn leaves_callable_admission_to_runtime_guards() {
    let bytecode = nested_function(
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += Number(i); } return s; }",
    );
    assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
}

#[test]
fn recognizes_numeric_global_local_and_method_calls() {
    for source in [
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i); } return s; }",
        "function sum(n) { var f = makeLeaf(); var s = 0; for (var i = 0; i < n; i++) { s += f(i); } return s; }",
        "function sum(n) { var o = { f: leaf }; var s = 0; for (var i = 0; i < n; i++) { s += o.f(i); } return s; }",
        "function runMethodCall(iterations) { var receiver = { addOne: function (value) { return value + 1; } }; var checksum = 0; for (var i = 0; i < iterations; i++) { checksum += receiver.addOne(i); } return { operations: iterations, checksum: checksum }; }",
        "function sum(n) { var f = makeWriter(); var s = 0; for (var i = 0; i < n; i++) { s += f(); } return s; }",
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i, 2); } return s; }",
        "function sum(n) { var f = makeLeaf(); var s = 0; for (var i = 0; i < n; i++) { s += f(i, 3); } return s; }",
        "function sum(n) { var o = { f: leaf }; var s = 0; for (var i = 0; i < n; i++) { s += o.f(i, 4); } return s; }",
    ] {
        let bytecode = nested_function(source);
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
    }
}

#[test]
fn benchmark_function_loops_enter_the_numeric_trace() {
    for (source, expected) in [
        (
            "function addOne(value) { return value + 1; } \
             function run(iterations) { var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += addOne(i); \
               return checksum; } run(1000);",
            Value::Number(500500.0),
        ),
        (
            "function run(iterations) { \
               var receiver = { addOne: function (value) { return value + 1; } }; \
               var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += receiver.addOne(i); \
               return checksum; } run(1000);",
            Value::Number(500500.0),
        ),
        (
            "var broadGlobalOne = 1; \
             function run(iterations) { var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += broadGlobalOne; \
               return checksum; } run(1000);",
            Value::Number(1000.0),
        ),
    ] {
        reset_loop_counters();
        assert_eq!(eval(source), Ok(expected), "{source}");
        assert_eq!(loop_counters(), (1, 0), "{source}");
    }
}

#[test]
fn recognizes_reordered_numeric_global_call() {
    let source =
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; }";
    let bytecode = nested_function(source);
    assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
    assert_eq!(
        eval(
            "function leaf(value) { return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(1000);"
        ),
        Ok(Value::Number(500500.0))
    );
}

#[test]
fn reordered_non_numeric_and_mutating_calls_keep_the_observable_path() {
    assert_eq!(
        eval(
            "function leaf(value) { return 'x' + value; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);"
        ),
        Ok(Value::String("x2x1x00".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function leaf(value) { if (value === 1) { leaf = function (next) { return next + 10; }; } return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);"
        ),
        Ok(Value::Number(15.0))
    );
}

#[test]
fn recognizes_reordered_numeric_local_call_and_rejects_counter_accumulation() {
    let source = "function sum(n) { var offset = 1; var leaf = function (value) { return value + offset; }; var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; }";
    let bytecode = nested_function(source);
    assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
    assert_eq!(
        eval(&format!("{source} sum(1000);")),
        Ok(Value::Number(500500.0))
    );

    let counter_source =
        "function sum(n) { for (var i = 0; i < n; i++) { i = leaf(i) + i; } return i; }";
    let counter_bytecode = nested_function(counter_source);
    assert!(
        NumericLoopPlan::compile_all(&counter_bytecode).is_empty(),
        "{counter_source}"
    );
}

#[test]
fn recognizes_numeric_global_object_method_calls() {
    let bytecode = nested_function(
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += Math.abs(-1); } return s; }",
    );
    assert_eq!(
        NumericLoopPlan::compile_all(&bytecode).len(),
        1,
        "{:?}",
        bytecode.code
    );
}

#[test]
fn recognizes_dense_array_index_of_calls() {
    let bytecode = nested_function(
        "function sum(n) { var array = [1, 2, 3, 4]; var s = 0; for (var i = 0; i < n; i++) { s += array.indexOf(3); } return s; }",
    );
    assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
}

#[test]
fn recognizes_numeric_string_slice_length_calls() {
    let bytecode = nested_function(
        "function sum(n) { var text = 'the quick brown fox'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 4).length; } return s; }",
    );
    assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
    assert_eq!(
        eval(
            "function sum(n) { var text = 'the quick brown fox'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 4).length; } return s; } sum(1000);"
        ),
        Ok(Value::Number(3000.0))
    );
    assert_eq!(
        eval(
            "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(0, 1).length; } return s; } sum(4);"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(
            "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "function sum(n) { var text = '\\u{F0000}x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "function sum(n) { var text = 'abcdef'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(-3, -1).length; } return s; } sum(4);"
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn overridden_string_slice_keeps_the_observable_loop_path() {
    assert_eq!(
        eval(
            "String.prototype.slice = function () { return { length: 7 }; }; function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s; } sum(4);"
        ),
        Ok(Value::Number(28.0))
    );
    assert_eq!(
        eval(
            "var reads = 0; var slice = String.prototype.slice; Object.defineProperty(String.prototype, 'slice', { get: function () { reads += 1; return slice; } }); function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s + ':' + reads; } sum(4);"
        ),
        Ok(Value::String("4:4".to_owned().into()))
    );
}

#[test]
fn rejects_non_numeric_call_constants() {
    let bytecode = nested_function(
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i, 'x'); } return s; }",
    );
    assert!(NumericLoopPlan::compile_all(&bytecode).is_empty());
}
