//! Exits: handing a wide activation to an interpreter frame, and the loop
//! probes that run a loop's typed program in place instead.

use super::*;

/// The arity an exit is spelled with; no call has it (`MAX_CALL_ARITY`).
pub(super) const EXIT_ARGC: u8 = u8::MAX;

/// Where a root activation's received cells come from, for an exit to hand
/// the same source to the interpreter frame. An inlined callee's comes from
/// its own `Function`.
#[derive(Clone, Copy)]
pub(super) struct RootExit<'a> {
    pub(super) upvalues: crate::bytecode::DirectCallUpvalues<'a>,
    pub(super) realm_upvalue_slots: u128,
}

/// What became of an exit.
pub(super) enum ExitOutcome {
    /// The interpreter frame finished the activation.
    Finished(Result<Value, RuntimeError>),
    /// The activation continues here at wide instruction `pc`: a loop no
    /// accelerator claims came back, or its exit was already known to.
    Continue { pc: usize },
}

/// Hands the current activation to an interpreter frame that resumes at
/// bytecode instruction `ip`, and returns that frame's completion value. The
/// frame is the one the general call path builds for this call -- the same
/// environment (which admission proved the activation's own), the same
/// received cells and `this` -- brought to this activation's state.
///
/// At a probed backedge the frame may hand the activation back instead
/// (`vm/wide_resume.rs`); that backedge's exit then stays declined, so the
/// loop runs here from then on.
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(super) fn exit_to_interpreter(
    callee: &Value,
    root_bytecode: &Bytecode,
    root: RootExit<'_>,
    ip: u32,
    depth: u16,
    resume_pc: usize,
    window: &mut [Value],
    env: &CallEnv,
    this_value: Option<Value>,
) -> ExitOutcome {
    let bytecode = running_bytecode(callee, root_bytecode);
    let Some(program) = super::super::program_for(bytecode) else {
        return ExitOutcome::Finished(Err(missing_program()));
    };
    if let Some((header, backedge)) = program.probed_header(resume_pc) {
        return enter_loop_at_header(
            callee, bytecode, program, root, header, backedge, depth, resume_pc, window, env,
            this_value,
        );
    }
    let probe = program.probed_backedge(ip);
    if let Some(index) = probe
        && program.backedge_is_native(index)
    {
        return ExitOutcome::Continue {
            pc: program.backedge_jump_pc(index),
        };
    }
    if let Some(crate::bytecode::ir::Op::NewObjectDataLiteral { shape }) =
        bytecode.code.get(ip as usize)
    {
        let top = usize::from(program.local_registers) + usize::from(depth);
        let base = top - shape.input_len();
        let object = property::object_data_literal(shape, &mut window[base..top], env);
        execute::store(&mut window[base], object);
        return ExitOutcome::Continue { pc: resume_pc };
    }
    if let Some(crate::bytecode::ir::Op::SetPropIndex { index, .. }) =
        bytecode.code.get(ip as usize)
    {
        let top = program.local_registers + depth;
        if property::try_plain_set_index(window, top - 2, top - 1, *index, env) {
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    // A `for-in`'s key list and its per-key recheck, answered in place so
    // the loop stays on this tier (`compile::is_exit_safe`).
    match bytecode.code.get(ip as usize) {
        Some(crate::bytecode::ir::Op::EnumerateKeys { cache }) => {
            if let Some(pc) = program.resume_pc(ip as usize + 1, usize::from(depth)) {
                let top = usize::from(program.local_registers) + usize::from(depth);
                let target = std::mem::replace(&mut window[top - 1], Value::Undefined);
                let mut call_env = env.empty_frame();
                return match crate::bytecode::vm_ops::enumerate_keys_cached(
                    &target,
                    cache,
                    &mut call_env,
                ) {
                    Ok(keys) => {
                        window[top - 1] = Value::Array(keys);
                        ExitOutcome::Continue { pc }
                    }
                    Err(error) => ExitOutcome::Finished(Err(error)),
                };
            }
        }
        Some(crate::bytecode::ir::Op::ForInKeyIsEnumerable) => {
            let top = usize::from(program.local_registers) + usize::from(depth);
            if let Some(pc) = program.resume_pc(ip as usize + 1, usize::from(depth) - 1)
                && let Value::String(key) = &window[top - 1]
            {
                let key = key.clone();
                let target = std::mem::replace(&mut window[top - 2], Value::Undefined);
                execute::store(&mut window[top - 1], Value::Undefined);
                let mut call_env = env.empty_frame();
                return match crate::bytecode::vm_ops::for_in_property_is_enumerable(
                    target,
                    &key,
                    &mut call_env,
                ) {
                    Ok(enumerable) => {
                        window[top - 2] = Value::Boolean(enumerable);
                        ExitOutcome::Continue { pc }
                    }
                    Err(error) => ExitOutcome::Finished(Err(error)),
                };
            }
        }
        _ => {}
    }
    // `g = g + value` on a global string: appended in place, then the store
    // is skipped (see `compile::appends_to_global`).
    if let (
        Some(crate::bytecode::ir::Op::Binary(qjs_ast::BinaryOp::Add)),
        Some(crate::bytecode::ir::Op::StoreLocalOrGlobalSloppy { name, .. }),
    ) = (
        bytecode.code.get(ip as usize),
        bytecode.code.get(ip as usize + 1),
    ) && let Some(pc) = program.resume_pc(ip as usize + 2, usize::from(depth) - 2)
    {
        let top = usize::from(program.local_registers) + usize::from(depth);
        let (left, right) = window[top - 2..top].split_at_mut(1);
        if property::try_append_global_var(name, &mut left[0], &mut right[0], env) {
            return ExitOutcome::Continue { pc };
        }
    }
    if let Some(crate::bytecode::ir::Op::StoreLocalOrGlobalSloppy { name, .. }) =
        bytecode.code.get(ip as usize)
    {
        let top = usize::from(program.local_registers) + usize::from(depth) - 1;
        if property::try_store_global_var(name, &window[top], env) {
            execute::store(&mut window[top], Value::Undefined);
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    if let Some(crate::bytecode::ir::Op::SetProp { .. }) = bytecode.code.get(ip as usize) {
        let operand = |offset: u16| program.local_registers + depth - offset;
        if property::try_plain_set_prop(window, operand(3), operand(2), operand(1), env) {
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    let (upvalues, realm_upvalue_slots) = match callee {
        Value::Function(function) => (
            crate::bytecode::DirectCallUpvalues::Function(function),
            function.realm_upvalue_slots,
        ),
        _ => (root.upvalues, root.realm_upvalue_slots),
    };
    let (mut ip, mut depth) = (ip, depth);
    let probe_target = probe.and_then(|_| match bytecode.code.get(ip as usize) {
        Some(crate::bytecode::ir::Op::Jump(target)) => Some(*target),
        _ => None,
    });
    let mut from = match probe_target {
        Some(target) => ResumeFrom::ProbedBackedge(target),
        None => ResumeFrom::Exit,
    };
    if let Some(header) = probe_target {
        match run_typed_loop_here(
            bytecode,
            program,
            env,
            window,
            upvalues.as_slice(),
            this_value.as_ref(),
            header,
            ip as usize,
            depth as usize,
        ) {
            LoopHere::Declined => {}
            LoopHere::Continue { pc } => {
                if let Some(index) = probe {
                    program.mark_typed_ready(index);
                }
                return ExitOutcome::Continue { pc };
            }
            // The program deoptimized: the interpreter resumes where it
            // stopped, with the stack it rebuilt.
            LoopHere::Deoptimized {
                ip: resume,
                depth: stack_depth,
                declined_typed_loop_programs,
            } => {
                let (Ok(resume), Ok(stack_depth)) =
                    (u32::try_from(resume), u16::try_from(stack_depth))
                else {
                    return ExitOutcome::Finished(Err(missing_program()));
                };
                (ip, depth) = (resume, stack_depth);
                from = ResumeFrom::LoopDeoptimized {
                    declined_typed_loop_programs,
                };
            }
        }
    }
    // `QJS_CF_TRACE=1` names every exit: the body, and the instruction the
    // interpreter resumes at.
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_CF_TRACE").is_some() {
        eprintln!(
            "CFEXIT params=({}) len={} ip {} op {:?}{}",
            bytecode.parameter_names().join(","),
            bytecode.code.len(),
            ip,
            bytecode.code.get(ip as usize),
            if probe.is_some() { " probed" } else { "" }
        );
    }
    resume_in_interpreter(
        bytecode,
        program,
        upvalues,
        realm_upvalue_slots,
        ip,
        depth,
        from,
        window,
        env,
        this_value,
    )
}

/// Continues an activation on the interpreter at bytecode `ip`, with `depth`
/// operand-stack values in its stack registers.
#[allow(clippy::too_many_arguments)]
fn resume_in_interpreter(
    bytecode: &Bytecode,
    program: &WideProgram,
    upvalues: crate::bytecode::DirectCallUpvalues<'_>,
    realm_upvalue_slots: u128,
    ip: u32,
    depth: u16,
    from: ResumeFrom,
    window: &mut [Value],
    env: &CallEnv,
    this_value: Option<Value>,
) -> ExitOutcome {
    let local_registers = program.local_registers as usize;
    let (locals, stack) = window.split_at_mut(local_registers);
    let slots = DirectCallSlots {
        this_value,
        parameter_slots: bytecode.parameter_slots(),
        arguments: &[],
        upvalues,
        realm_upvalue_slots,
    };
    let registers = crate::bytecode::vm::WideRegisters {
        locals,
        own_locals: program.own_locals,
        stack,
        depth: depth as usize,
        tdz_marker: &program.tdz_marker,
    };
    match crate::bytecode::vm::resume_direct_call_bytecode(
        bytecode,
        env.clone(),
        slots,
        ip as usize,
        registers,
        from,
    ) {
        Resumed::Finished(result) => {
            program.record_exit();
            ExitOutcome::Finished(result)
        }
        // The interpreter frame only ran the accelerated loop, which is the
        // cost the general path would have paid too; such an exit does not
        // count toward judging the body exit-heavy.
        Resumed::LoopFinished { ip: resume, depth } => match program.resume_pc(resume, depth) {
            Some(pc) => ExitOutcome::Continue { pc },
            None => ExitOutcome::Finished(Err(missing_program())),
        },
        Resumed::HandedBack { backedge } => {
            program.record_exit();
            let Some(index) = u32::try_from(backedge)
                .ok()
                .and_then(|backedge| program.probed_backedge(backedge))
            else {
                return ExitOutcome::Finished(Err(missing_program()));
            };
            // `QJS_CF_TRACE=1` names each loop handed back to this tier.
            #[cfg(feature = "perf-counters")]
            if std::env::var_os("QJS_CF_TRACE").is_some() {
                eprintln!(
                    "CFNATIVE params=({}) len={} ip {backedge}",
                    bytecode.parameter_names().join(","),
                    bytecode.code.len()
                );
            }
            program.keep_backedge_native(index);
            ExitOutcome::Continue {
                pc: program.backedge_jump_pc(index),
            }
        }
    }
}

/// A loop entered from above (`ProbedHeader`): once its typed program has
/// run a loop to its end from the backedge, it runs from the header, before
/// the first iteration; otherwise the activation continues into the loop.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn enter_loop_at_header(
    callee: &Value,
    bytecode: &Bytecode,
    program: &WideProgram,
    root: RootExit<'_>,
    header: usize,
    backedge: usize,
    depth: u16,
    resume_pc: usize,
    window: &mut [Value],
    env: &CallEnv,
    this_value: Option<Value>,
) -> ExitOutcome {
    let ready = u32::try_from(backedge)
        .ok()
        .and_then(|backedge| program.probed_backedge(backedge))
        .is_some_and(|index| program.typed_ready(index) && !program.backedge_is_native(index));
    if !ready {
        return ExitOutcome::Continue { pc: resume_pc };
    }
    let (upvalues, realm_upvalue_slots) = match callee {
        Value::Function(function) => (
            crate::bytecode::DirectCallUpvalues::Function(function),
            function.realm_upvalue_slots,
        ),
        _ => (root.upvalues, root.realm_upvalue_slots),
    };
    match run_typed_loop_here(
        bytecode,
        program,
        env,
        window,
        upvalues.as_slice(),
        this_value.as_ref(),
        header,
        backedge,
        usize::from(depth),
    ) {
        LoopHere::Declined => ExitOutcome::Continue { pc: resume_pc },
        LoopHere::Continue { pc } => ExitOutcome::Continue { pc },
        LoopHere::Deoptimized {
            ip,
            depth,
            declined_typed_loop_programs,
        } => {
            let (Ok(ip), Ok(depth)) = (u32::try_from(ip), u16::try_from(depth)) else {
                return ExitOutcome::Finished(Err(missing_program()));
            };
            resume_in_interpreter(
                bytecode,
                program,
                upvalues,
                realm_upvalue_slots,
                ip,
                depth,
                ResumeFrom::LoopDeoptimized {
                    declined_typed_loop_programs,
                },
                window,
                env,
                this_value,
            )
        }
    }
}
