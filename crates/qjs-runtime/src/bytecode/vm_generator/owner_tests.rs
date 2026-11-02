//! Suspendable ownership identity and retirement checks.
use super::*;
fn frame_address(generator: &ObjectRef) -> usize {
    let state = generator.generator_state().borrow();
    let Some(GeneratorState::SuspendedStart(suspended) | GeneratorState::SuspendedYield(suspended)) =
        state.as_ref()
    else {
        panic!("suspended owning record");
    };
    &*suspended.frame as *const super::super::vm::FrameState<'static> as usize
}
#[test]
fn resumes_the_same_box_from_prologue_through_multiple_yields() {
    let Value::Object(generator) = crate::eval("function* g(){yield 1; yield 2;} g()").unwrap()
    else {
        panic!("generator")
    };
    let address = frame_address(&generator);
    assert!(matches!(
        generator.generator_state().borrow().as_ref(),
        Some(GeneratorState::SuspendedStart(_))
    ));
    let mut root = crate::context::HostRoot::script();
    for n in [1.0, 2.0] {
        let result = resume_generator(
            &generator,
            Resume::Next(Value::Undefined),
            &mut root.context(),
        )
        .unwrap();
        assert!(matches!(result, GeneratorOutcome::Yield(Value::Number(v)) if v == n));
        assert_eq!(frame_address(&generator), address);
    }
}

fn prepared() -> (
    crate::context::RuntimeServices,
    SuspendedActivation,
    Rc<Bytecode>,
) {
    let code = Rc::new(crate::compile_script(&qjs_parser::parse_script("1").unwrap()).unwrap());
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    let record = prepare_suspended_start(
        code.clone(),
        activation,
        Vec::new(),
        Vec::new(),
        None,
        &mut runtime,
    );
    (runtime, record, code)
}

#[test]
fn every_suspension_kind_keeps_one_box_and_one_code_handle() {
    let (mut runtime, mut owner, code) = prepared();
    let address = &*owner.frame as *const _ as usize;
    owner.frame.cold_mut().pending_return = Some(Value::Number(42.0));
    for (completion, kind) in [
        (
            Completion::Yield(Value::Undefined),
            SuspensionKind::Ordinary,
        ),
        (
            Completion::Await(Value::Undefined),
            SuspensionKind::Ordinary,
        ),
        (
            Completion::YieldDelegate(Value::Undefined),
            SuspensionKind::DelegateYield,
        ),
        (
            Completion::YieldDelegateAwait(Value::Undefined),
            SuspensionKind::DelegateAwait,
        ),
        (
            Completion::YieldDelegateAwaitReturn(Value::Undefined),
            SuspensionKind::DelegateAwaitReturn,
        ),
        (
            Completion::YieldDelegateAwaitReturnValue(Value::Undefined),
            SuspensionKind::DelegateAwaitReturnValue,
        ),
        (
            Completion::YieldDelegateAsync(Value::Undefined),
            SuspensionKind::DelegateYieldAsync,
        ),
    ] {
        let vm = owner.attach(&mut runtime);
        let (GeneratorState::SuspendedYield(record), _) = drive(Ok(completion), vm).unwrap() else {
            panic!("suspended owner")
        };
        assert_eq!(&*record.frame as *const _ as usize, address);
        assert_eq!(
            std::mem::discriminant(&record.suspension),
            std::mem::discriminant(&kind)
        );
        assert_eq!(
            record.frame.cold().unwrap().pending_return,
            Some(Value::Number(42.0))
        );
        assert_eq!(
            Rc::strong_count(&code),
            2,
            "only the caller and frame own code"
        );
        owner = record;
    }
    // The seventh kind is entered by injecting return at an async delegate
    // yield, before awaiting the return operand and invoking the iterator.
    let (GeneratorState::SuspendedYield(record), outcome) =
        run_from_yield(owner, Resume::Return(Value::Number(9.0)), &mut runtime).unwrap()
    else {
        panic!("awaiting delegate return")
    };
    assert!(matches!(
        outcome,
        GeneratorOutcome::Await(Value::Number(9.0))
    ));
    assert!(matches!(
        record.suspension,
        SuspensionKind::DelegateYieldReturnAwait
    ));
    assert_eq!(&*record.frame as *const _ as usize, address);
    assert_eq!(Rc::strong_count(&code), 2);
}

#[test]
fn dropping_unresumed_owner_releases_its_private_values() {
    let (_runtime, mut owner, _code) = prepared();
    let object = ObjectRef::new(Default::default(), owner.frame.env.realm().atoms());
    let weak = object.downgrade();
    owner.frame.stack.push(Value::Object(object));
    assert!(weak.upgrade().is_some());
    drop(owner);
    assert!(weak.upgrade().is_none());
}

#[test]
fn abrupt_completion_before_start_drops_the_prepared_frame() {
    for resume in [
        Resume::Return(Value::Number(3.0)),
        Resume::ReturnAlreadyAwaited(Value::Number(3.0)),
        Resume::Throw(Value::Number(3.0)),
    ] {
        let (_runtime, mut owner, _code) = prepared();
        let generator = ObjectRef::new(Default::default(), owner.frame.env.realm().atoms());
        let object = ObjectRef::new(Default::default(), owner.frame.env.realm().atoms());
        let weak = object.downgrade();
        owner.frame.stack.push(Value::Object(object));
        *generator.generator_state().borrow_mut() = Some(GeneratorState::SuspendedStart(owner));
        let throwing = matches!(resume, Resume::Throw(_));
        let result = resume_generator(
            &generator,
            resume,
            &mut crate::context::HostRoot::script().context(),
        );
        assert_eq!(result.is_err(), throwing);
        assert!(weak.upgrade().is_none());
        assert!(matches!(
            generator.generator_state().borrow().as_ref(),
            Some(GeneratorState::Completed)
        ));
    }
}
#[test]
fn segment_resume_clears_declines_without_losing_semantic_state() {
    let code = Rc::new(crate::compile_script(&qjs_parser::parse_script("1").unwrap()).unwrap());
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    let mut vm = Vm::with_frame_bytecode(
        code.into(),
        &mut runtime,
        activation,
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
    .into_suspendable();
    vm.declined_typed_loop_programs = u128::MAX;
    vm.cold_mut().wide_handback = Some(Some(10));
    vm.cold_mut().pending_return = Some(Value::Number(42.0));
    let address = &*vm.current as *const _ as usize;
    vm.reset_segment();
    assert_eq!(vm.declined_typed_loop_programs, 0);
    assert!(vm.cold().unwrap().wide_handback.is_none());
    assert_eq!(vm.cold().unwrap().pending_return, Some(Value::Number(42.0)));
    assert_eq!(&*vm.current as *const _ as usize, address);
}
