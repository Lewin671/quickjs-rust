use super::*;
use crate::bytecode::compiler;
use crate::bytecode::ir::Local;

fn nested_function(source: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|operation| match operation {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("function bytecode should be nested in the script")
}

fn append_compiler_temporaries(bytecode: &mut Bytecode, count: usize) -> Vec<usize> {
    (0..count)
        .map(|index| {
            let slot = bytecode.locals.len();
            bytecode.locals.push(Local {
                name: format!("<trace-test-{index}>"),
                compiler_temporary: true,
                hoisted: false,
                hoisted_function: false,
                parameter: false,
                catch_binding: false,
                mutable: true,
                from_env: false,
                sloppy_global_fallback: false,
            });
            slot
        })
        .collect()
}

#[test]
fn trace_live_out_alias_is_captured_at_its_store_before_the_source_changes() {
    let mut bytecode = nested_function("function f(source) { return 9; }");
    let source = bytecode.local_slot("source").expect("parameter slot");
    let target = append_compiler_temporaries(&mut bytecode, 1)[0];
    let changed = bytecode
        .constants
        .iter()
        .position(|value| matches!(value, Value::Number(number) if *number == 9.0))
        .expect("numeric constant");
    let live_outs = BTreeSet::from([target]);
    let mut translator = Translator::new_trace(&bytecode, &live_outs);

    for operation in [
        Op::LoadLocal(source),
        Op::StoreLocal(target),
        Op::LoadConst(changed),
        Op::AssignLocal(source),
    ] {
        translator
            .translate(&operation)
            .expect("numeric alias sequence should translate");
    }
    assert_eq!(translator.validate_trace_live_outs(), Some(1));

    let target_write = translator
        .writes
        .iter()
        .find(|write| write.local == target)
        .expect("live-out alias must be materialized at its store");
    let source_write = translator
        .writes
        .iter()
        .find(|write| write.local == source)
        .expect("later source assignment should remain distinct");
    assert!(target_write.value < source_write.value);
    assert!(matches!(
        translator.operations[target_write.value],
        NumberInstruction::LoadLocal(slot) if slot == source
    ));

    let mut registers = vec![0.0; translator.operations.len()];
    for (register, operation) in translator.operations.iter().enumerate() {
        registers[register] = match operation {
            NumberInstruction::LoadLocal(slot) if *slot == source => 3.0,
            NumberInstruction::Constant(value) => *value,
            operation => panic!("unexpected operation in alias regression: {operation:?}"),
        };
    }
    assert_eq!(registers[target_write.value], 3.0);
    assert_eq!(registers[source_write.value], 9.0);
    assert_eq!(
        registers[target_write.value], 3.0,
        "next K reads the captured value"
    );
}

#[test]
fn trace_live_out_alias_dependency_closure_preserves_two_level_carries() {
    let mut bytecode =
        nested_function("function f(source) { var x=1,y=2,z=3; x=y+z; y=z+x; z=x+y; return z; }");
    let temporaries = append_compiler_temporaries(&mut bytecode, 3);
    let [first, second, third] = temporaries.as_slice() else {
        panic!("alias carry regression needs three compiler temporaries");
    };
    let sequence = [
        Op::LoadLocal(*second),
        Op::StoreLocal(*first),
        Op::LoadLocal(*third),
        Op::StoreLocal(*second),
    ];

    let mut probe = Translator::new(&bytecode);
    for operation in &sequence {
        probe
            .translate(operation)
            .expect("alias probe should translate");
    }
    let mut live_outs = BTreeSet::from([*first]);
    super::super::close_trace_alias_dependencies(
        &bytecode,
        &mut live_outs,
        &probe.alias_dependencies.iter().copied().collect::<Vec<_>>(),
    );
    assert_eq!(live_outs, BTreeSet::from([*first, *second, *third]));

    let mut translator = Translator::new_trace(&bytecode, &live_outs);
    for operation in &sequence {
        translator
            .translate(operation)
            .expect("closed alias carries should materialize");
    }
    assert_eq!(translator.validate_trace_live_outs(), Some(2));

    let run_iteration = |entry: &BTreeMap<usize, f64>| {
        let mut registers = vec![0.0; translator.operations.len()];
        for (register, operation) in translator.operations.iter().enumerate() {
            registers[register] = match operation {
                NumberInstruction::LoadLocal(slot) => entry[slot],
                operation => panic!("unexpected carry operation: {operation:?}"),
            };
        }
        let mut next = entry.clone();
        for write in &translator.writes {
            next.insert(write.local, registers[write.value]);
        }
        next
    };
    let entry = BTreeMap::from([(*first, 1.0), (*second, 3.0), (*third, 5.0)]);
    let after_first = run_iteration(&entry);
    assert_eq!(after_first[first], 3.0);
    assert_eq!(after_first[second], 5.0);
    let after_second = run_iteration(&after_first);
    assert_eq!(after_second[first], 5.0);
    assert_eq!(after_second[second], 5.0);
}
