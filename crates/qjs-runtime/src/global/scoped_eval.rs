//! Direct eval executes against live leases and the selected own var holder.
//! Declarations choose their owner once; effects go directly to canonical cells.

use crate::binding::{
    AccessPolicy, BindingCell, BindingKind, CellReference, ScopeCursor, ScopeDescriptorRef,
    ScopeId, ScopeLease, VarHolder, VariableEnvIdentity,
};
use crate::{Bytecode, CallEnv, RuntimeError, Value};
use std::rc::Rc;

pub(super) fn evaluate(
    bytecode: &Bytecode,
    env: &mut CallEnv,
    strict: bool,
) -> Result<Value, RuntimeError> {
    let caller = env.scope_cursor().cloned();
    let in_parameters = matches!(
        env.get(crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING),
        Some(Value::Boolean(true))
    );
    let holder = strict.then(|| {
        Rc::new(VarHolder::new(VariableEnvIdentity {
            activation: env.realm().next_variable_identity(),
            boundary: ScopeId(0),
        }))
    });
    let mut cells = Vec::new();
    let mut eval_env = env.clone();
    eval_env.clear_deopt_bindings();
    eval_env.set_direct_eval_with_stack(Vec::new());
    if !strict {
        for meta in bytecode.scope_declarations(ScopeId(0)) {
            if matches!(meta.kind, BindingKind::Var | BindingKind::Function) {
                caller
                    .as_ref()
                    .expect("sloppy direct eval has caller")
                    .validate_eval_declaration(&meta.name, in_parameters)?;
            }
        }
    }
    for reference in bytecode.scope_declarations(ScopeId(0)) {
        let name = reference.name.clone();
        let kind = reference.kind;
        let cell = match kind {
            BindingKind::Var | BindingKind::Function => {
                if let Some(holder) = &holder {
                    let cell = BindingCell::with_metadata(reference, Some(Value::Undefined));
                    holder.insert_cell(name.clone(), cell.clone());
                    cell
                } else {
                    caller
                        .as_ref()
                        .expect("sloppy direct eval has caller")
                        .declare_var(&name)
                }
            }
            _ => BindingCell::with_metadata(reference, None),
        };
        eval_env.insert_frame_cell(name, cell.clone());
        cells.push(CellReference {
            cell,
            access: AccessPolicy::Mutable,
        });
    }
    let root = Rc::new(ScopeLease {
        descriptor: ScopeDescriptorRef {
            owner: bytecode.scope_descriptors(),
            id: ScopeId(0),
        },
        parent: caller.as_ref().map(|cursor| cursor.head.clone()),
        cells: cells.into_boxed_slice(),
        object: None,
        own_vars: holder,
    });
    eval_env.set_eval_scope(ScopeCursor {
        head: root.clone(),
        variable: if strict {
            root
        } else {
            caller.expect("sloppy direct eval has caller").variable
        },
    });
    crate::bytecode::eval_bytecode_with_env_ephemeral_global_lexicals(bytecode, eval_env).value
}
