//! Cross-body storage reuse, owner teardown and common capacity limits.
use super::*;
use crate::{ObjectRef, bytecode::frame_bank::StackPool};

#[test]
fn operand_capacity_is_cleared_reused_and_bounded() {
    let pool = GeneralPool::new(false);
    let mut first = pool.take_operands();
    first.push(Value::Number(1.0));
    let address = first.as_ptr();
    pool.put_operands(first);
    let next = pool.take_operands();
    assert!(next.is_empty());
    assert_eq!(next.as_ptr(), address);
    pool.put_operands(Vec::with_capacity(MAX_CAPACITY + 1));
    assert!(pool.operands.borrow().is_empty());
    let active: Vec<_> = (0..MAX_VECTORS + 2).map(|_| pool.take_operands()).collect();
    for values in active {
        pool.put_operands(values);
    }
    assert_eq!(pool.operands.borrow().len(), MAX_VECTORS);
}

#[test]
fn local_poison_is_checked_and_cleared_across_slot_counts() {
    let pool = GeneralPool::new(true);
    let mut values = pool.take_locals(8);
    values[0] = Some(Value::Number(42.0));
    let address = values.as_ptr();
    pool.put_locals(values);
    let small = pool.take_locals(2);
    assert_eq!(small.as_ptr(), address);
    assert!(small.iter().all(Option::is_none));
    pool.put_locals(small);
    let large = pool.take_locals(7);
    assert_eq!(large.as_ptr(), address);
    assert!(large.iter().all(Option::is_none));
}

#[test]
#[should_panic(expected = "released local poison changed")]
fn modification_of_released_local_poison_is_detected() {
    let pool = GeneralPool::new(true);
    pool.put_locals(vec![None; 4]);
    pool.locals.borrow_mut()[0].slots[0] = Some(Value::Number(2.0));
    pool.take_locals(3);
}

#[test]
fn all_storage_families_share_the_record_budget() {
    let mut owner = StackPool::new(true);
    let pool = owner.general();
    for index in 0..200 {
        match index % 4 {
            0 => {
                let mut storage = FrameStorage::default();
                storage.reserve(128).unwrap();
                owner.put(storage);
            }
            1 => pool.put_operands(Vec::with_capacity(64)),
            2 => pool.put_locals(vec![None; 128]),
            _ => pool.put_cold(Box::default()),
        }
    }
    assert_eq!(pool.count.get(), MAX_RECORDS);
    assert!(pool.retained_bytes() <= MAX_BYTES);
    drop(owner);
    assert_eq!(pool.count.get(), 0);
    assert_eq!(pool.bytes.get(), 0);
}

#[test]
fn payload_and_directory_capacity_share_the_byte_budget() {
    let mut owner = StackPool::default();
    let pool = owner.general();
    for _ in 0..MAX_RECORDS {
        let mut storage = FrameStorage::default();
        storage.reserve(512).unwrap();
        owner.put(storage);
    }
    assert!(pool.count.get() < MAX_RECORDS, "the byte limit wins first");
    assert!(pool.retained_bytes() <= MAX_BYTES);
}

#[test]
fn cold_reuse_releases_values_and_charges_inner_capacity() {
    let pool = GeneralPool::new(false);
    let root = crate::context::HostRoot::script();
    let object = ObjectRef::new(Default::default(), root.activation.realm().atoms());
    let weak = object.downgrade();
    let mut cold = pool.take_cold();
    cold.pending_return = Some(Value::Object(object.clone()));
    cold.with_stack.push(Value::Object(object));
    let address = &*cold as *const _;
    pool.put_cold(cold);
    assert!(weak.upgrade().is_none());
    let mut cold = pool.take_cold();
    assert_eq!(&*cold as *const _, address);
    assert!(cold.pending_return.is_none() && cold.with_stack.is_empty());
    assert!(cold.retained_bytes() > size_of::<ColdFrame>());
    cold.with_stack.reserve(MAX_RECORD_BYTES);
    pool.put_cold(cold);
    assert_eq!(pool.count.get(), 0, "oversized inner storage is dropped");
}

#[test]
fn a_retained_handle_outlives_services_without_retaining_free_capacity() {
    let retained = {
        let owner = StackPool::default();
        let pool = owner.general();
        pool.put_operands(pool.take_operands());
        pool
    };
    assert_eq!(retained.count.get(), 0);
    let values = retained.take_operands();
    retained.put_operands(values);
    assert_eq!(retained.count.get(), 1);
}

#[test]
fn releasing_a_value_can_drop_another_frame_using_the_same_pool() {
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    let holder = ObjectRef::new(Default::default(), activation.realm().atoms());
    let weak = holder.downgrade();
    let code = std::rc::Rc::new(
        crate::compile_script(&qjs_parser::parse_script("let inner=1;").unwrap()).unwrap(),
    );
    let record = crate::bytecode::prepare_suspended_start(
        code,
        activation,
        Vec::new(),
        Vec::new(),
        None,
        &mut runtime,
    );
    *holder.generator_state().borrow_mut() =
        Some(crate::bytecode::GeneratorState::SuspendedStart(record));
    runtime
        .frame_stacks
        .general()
        .put_locals(vec![Some(Value::Object(holder))]);
    assert!(
        weak.upgrade().is_none(),
        "nested teardown must not borrow a bin recursively"
    );
}

#[test]
fn escaped_cells_and_mapped_arguments_survive_cross_body_reuse() {
    assert_eq!(crate::eval("function owner(x){var y=x+1;return [function(){return x+y},function(v){x=v;y=v+1},arguments];} var held=owner(5); function other(...a){var p=a.length;let q=p+1;return q;} for(var i=0;i<100;i++)other(1,2,3); held[2][0]=7; var first=held[0](); held[1](9); first+':'+held[0]()+':'+held[2][0];").unwrap(), Value::string("13:19:9".into()));
}
