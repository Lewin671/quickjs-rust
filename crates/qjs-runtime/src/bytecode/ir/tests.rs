use super::*;
use crate::Property;
use crate::bytecode::compiler;

#[cfg(target_pointer_width = "64")]
#[test]
fn opcode_layout_keeps_cold_class_definitions_out_of_line() {
    assert!(
        std::mem::size_of::<Op>() <= 112,
        "Op grew to {} bytes; cold payloads must stay out of line",
        std::mem::size_of::<Op>()
    );
}

#[test]
fn compiler_temporaries_are_excluded_from_written_binding_caches() {
    let script = qjs_parser::parse_script(
        "var total = 0; for (var index = 0; index < 4; index++) total += index;",
    )
    .expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    let written = bytecode.written_binding_names();
    let temporary_names = bytecode
        .locals
        .iter()
        .filter(|local| local.compiler_temporary)
        .map(|local| local.name.clone())
        .collect::<Vec<_>>();

    assert!(!temporary_names.is_empty());
    assert!(written.iter().any(|name| name == "total"));
    assert!(written.iter().any(|name| name == "index"));
    for name in temporary_names {
        assert!(
            !written.contains(&name),
            "temporary leaked into writeback: {name:?}"
        );
        assert!(
            !bytecode.writes_binding(&name),
            "temporary leaked into recursive write set: {name:?}"
        );
    }
}

#[test]
fn index_receiver_codec_round_trips_shared_ir_layout() {
    let plain = encode_index_receiver(7, None).expect("plain index");
    assert_eq!(decode_index_receiver(plain), (7, None));

    if usize::BITS > u32::BITS {
        let fused = encode_index_receiver(11, Some(3)).expect("fused receiver");
        assert_eq!(decode_index_receiver(fused), (11, Some(3)));
        assert_eq!(encode_index_receiver(u32::MAX as usize + 1, None), None);
    } else {
        assert_eq!(encode_index_receiver(11, Some(3)), None);
    }
}

#[test]
fn operand_stack_pool_reuses_cleared_bounded_storage() {
    let bytecode = Bytecode::new(Vec::new(), Vec::new(), Vec::new());
    let recycler = bytecode.operand_stack_recycler();
    let mut first = recycler.take();
    first.push(Value::Number(1.0));
    let allocation = first.as_ptr();

    recycler.recycle(first);
    let reused = recycler.take();

    assert!(reused.is_empty());
    assert_eq!(reused.as_ptr(), allocation);
    recycler.recycle(reused);

    let _active = recycler.take();
    let oversized = Vec::with_capacity(OperandStackRecycler::MAX_RECYCLED_CAPACITY + 1);
    recycler.recycle(oversized);
    assert_eq!(recycler.pooled_len(), 0);

    // Nested frames each get their own recycled stack, up to the bound.
    let nested: Vec<_> = (0..OperandStackRecycler::MAX_POOLED + 2)
        .map(|_| recycler.take())
        .collect();
    for stack in nested {
        recycler.recycle(stack);
    }
    assert_eq!(recycler.pooled_len(), OperandStackRecycler::MAX_POOLED);
}

#[test]
fn a_recycler_handle_outlives_the_bytecode_it_came_from() {
    // This is the property the frame-stack migration needs: a frame can
    // return its operand stack when it ends without having borrowed the
    // bytecode for the frame's whole lifetime.
    let recycler = {
        let bytecode = Bytecode::new(Vec::new(), Vec::new(), Vec::new());
        bytecode.operand_stack_recycler()
    };
    let mut stack = recycler.take();
    stack.push(Value::Number(1.0));
    recycler.recycle(stack);
    assert_eq!(recycler.pooled_len(), 1);
}

#[test]
fn direct_parameter_slots_preserve_duplicate_positions() {
    let bytecode = Bytecode::new_function(Vec::new(), Vec::new(), Vec::new(), vec![3, 3, 7]);

    assert_eq!(bytecode.parameter_slots(), &[3, 3, 7]);
}

#[test]
fn direct_readonly_received_upvalue_mask_requires_read_only_slots() {
    let captured = Local {
        name: "captured".to_owned(),
        compiler_temporary: false,
        hoisted: false,
        hoisted_function: false,
        parameter: false,
        catch_binding: false,
        mutable: true,
        from_env: true,
        sloppy_global_fallback: false,
    };
    let read_only = Bytecode::new(Vec::new(), vec![captured.clone()], vec![Op::LoadLocal(0)]);
    assert_eq!(read_only.direct_readonly_received_upvalue_slots(), Some(1));
    assert_eq!(read_only.direct_readonly_received_upvalue_index(0), Some(0));

    let writes = Bytecode::new(Vec::new(), vec![captured], vec![Op::StoreLocal(0)]);
    assert_eq!(writes.direct_readonly_received_upvalue_slots(), None);
    assert_eq!(writes.direct_readonly_received_upvalue_index(0), None);

    let append = Bytecode::new(
        Vec::new(),
        vec![Local {
            name: "captured".to_owned(),
            compiler_temporary: false,
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable: true,
            from_env: true,
            sloppy_global_fallback: false,
        }],
        vec![Op::AppendStringLiteralLocal {
            slot: 0,
            value: "x".to_owned(),
            discard: true,
        }],
    );
    assert_eq!(append.direct_readonly_received_upvalue_slots(), None);
}

#[test]
fn a_sloppy_global_fallback_keeps_only_the_frame_independent_read_only_mask() {
    let captured = Local {
        name: "captured".to_owned(),
        compiler_temporary: false,
        hoisted: false,
        hoisted_function: false,
        parameter: false,
        catch_binding: false,
        mutable: true,
        from_env: true,
        sloppy_global_fallback: false,
    };
    let fallback = Local {
        name: "assigned".to_owned(),
        from_env: false,
        sloppy_global_fallback: true,
        ..captured.clone()
    };
    let bytecode = Bytecode::new(
        Vec::new(),
        vec![captured, fallback],
        vec![
            Op::LoadLocal(0),
            Op::StoreLocalOrGlobalSloppy {
                slot: 1,
                name: "assigned".to_owned(),
            },
        ],
    );
    // An interpreter frame routes the fallback through its own cell
    // vector, so it cannot borrow the function's; the wide tier can.
    assert_eq!(bytecode.direct_readonly_received_upvalue_slots(), None);
    assert_eq!(bytecode.readonly_received_upvalue_slots(), Some(1));
    assert_eq!(bytecode.readonly_received_upvalue_index(0), Some(0));
}

#[test]
fn named_property_cache_reuses_literal_shape_across_objects() {
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let first = ObjectRef::with_literal_pair(
        shape.clone(),
        [Value::Number(1.0), Value::Number(2.0)],
        None,
    );
    let second = ObjectRef::with_literal_pair(
        shape.clone(),
        [Value::Number(3.0), Value::Number(4.0)],
        None,
    );
    let cache = NamedPropertyCache::default();

    cache.update(&first, "a", &Value::Number(1.0));
    assert_eq!(cache.get(&second), Some(Value::Number(3.0)));

    second.define_property(
        "a".to_owned(),
        Property::data(Value::Number(5.0), false, false, true),
    );
    assert_eq!(cache.get(&second), None);

    let third = ObjectRef::with_literal_pair(shape, [Value::Number(6.0), Value::Number(7.0)], None);
    assert_eq!(cache.get(&third), Some(Value::Number(6.0)));
}

#[test]
fn named_property_cache_remembers_two_alternating_receivers() {
    // A call site whose receiver alternates between exactly two distinct
    // objects (for example `a.f()`/`b.f()` behind a ternary) must not
    // thrash a single-entry cache: both identities should stay cached
    // rather than evicting each other on every access.
    let first = ObjectRef::new(HashMap::from([("value".to_owned(), Value::Number(1.0))]));
    let second = ObjectRef::new(HashMap::from([("value".to_owned(), Value::Number(2.0))]));
    let cache = NamedPropertyCache::default();

    cache.update(&first, "value", &Value::Number(1.0));
    cache.update(&second, "value", &Value::Number(2.0));

    assert_eq!(cache.get(&first), Some(Value::Number(1.0)));
    assert_eq!(cache.get(&second), Some(Value::Number(2.0)));

    // A third distinct receiver evicts the oldest slot (round robin),
    // not the most recently used one.
    let third = ObjectRef::new(HashMap::from([("value".to_owned(), Value::Number(3.0))]));
    cache.update(&third, "value", &Value::Number(3.0));
    assert_eq!(cache.get(&second), Some(Value::Number(2.0)));
    assert_eq!(cache.get(&third), Some(Value::Number(3.0)));
}

#[test]
fn named_property_cache_weakly_caches_object_values() {
    let child = ObjectRef::new(HashMap::new());
    let child_weak = child.downgrade();
    let receiver = ObjectRef::new(HashMap::from([(
        "child".to_owned(),
        Value::Object(child.clone()),
    )]));
    let cache = NamedPropertyCache::default();

    cache.update(&receiver, "child", &Value::Object(child.clone()));
    let Some(Value::Object(cached)) = cache.get(&receiver) else {
        panic!("cached object value should remain reachable through its receiver");
    };
    assert!(cached.ptr_eq(&child));

    drop(cached);
    drop(receiver);
    drop(child);
    assert!(child_weak.upgrade().is_none());
}
