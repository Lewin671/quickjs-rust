//! Eval declarations retain canonical entry cells and a selected variable base.
//! Global declarations use the live global object; function eval uses its own
//! variable holder. Completion never returns a value map to apply to its caller.

use crate::binding::{
    AccessPolicy, BindingCell, BindingKind, CellReference, ScopeCursor, ScopeDescriptorRef,
    ScopeId, ScopeLease, VarHolder, VariableEnvIdentity,
};
use crate::{Bytecode, RuntimeError, Value};
use std::rc::Rc;

pub(super) fn evaluate(
    bytecode: &Bytecode,
    env: &mut crate::context::Context<'_>,
    strict: bool,
) -> Result<Value, RuntimeError> {
    let caller = env.scope_cursor().cloned();
    let global = !strict && (env.has_global_binding_root() || caller.is_none());
    let in_parameters = env
        .direct_eval_site()
        .is_some_and(|site| site.in_parameters);
    if !strict {
        for meta in bytecode.scope_declarations(ScopeId(0)) {
            if matches!(meta.kind, BindingKind::Var | BindingKind::Function) {
                if let Some(cursor) = &caller {
                    cursor.validate_eval_declaration(&meta.name, in_parameters)?;
                }
                if global
                    && env.can_cache_realm_globals()
                    && env.is_global_lexical_binding(&meta.name)
                {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "SyntaxError: eval declaration conflicts with `{}`",
                            meta.name
                        ),
                    });
                }
            }
        }
    }
    if global {
        prepare_global_declarations(bytecode, env)?;
    }
    let holder = strict.then(|| {
        // Eval code cannot suspend; its holder changes only before it runs.
        Rc::new(VarHolder::new(
            VariableEnvIdentity {
                activation: env.realm().next_variable_identity(),
                boundary: ScopeId(0),
            },
            env.realm().atoms().clone(),
            false,
        ))
    });
    let mut cells = Vec::new();
    for meta in bytecode.scope_declarations(ScopeId(0)) {
        let name = meta.name.clone();
        let cell = match meta.kind {
            BindingKind::Var | BindingKind::Function if strict => {
                let cell = BindingCell::with_metadata(meta, Some(Value::Undefined));
                holder
                    .as_ref()
                    .expect("strict eval holder")
                    .insert_cell(name, cell.clone());
                cell
            }
            BindingKind::Var | BindingKind::Function if !global => caller
                .as_ref()
                .expect("function variable cursor")
                .declare_var(&name),
            // Global object-record routes never read/write this positional
            // placeholder; the root descriptor retains its declaration ordinal.
            _ => BindingCell::with_metadata(meta, None),
        };
        cells.push(CellReference {
            cell,
            access: AccessPolicy::Mutable,
        });
    }
    crate::diagnostics::count!(scope_lease_allocations);
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
    let variable = match (&caller, strict) {
        (Some(caller), false) => caller.variable.clone(),
        _ => root.clone(),
    };
    let mut creation = env.creation_context();
    let private = env.private_environment();
    let same_private = match (&creation.private, &private) {
        (None, None) => true,
        (Some(left), Some(right)) => left.same(right),
        _ => false,
    };
    if !same_private {
        Rc::make_mut(&mut creation).private = private;
    }
    let owner = env.capture_call_owner();
    let mut selected =
        crate::context::ActivationContext::new(creation, crate::context::ExecutionMode::Eval);
    if let Some(owner) = owner {
        selected.inherit_call_owner(owner);
    }
    selected.strict = strict;
    selected.set_eval_scope(
        ScopeCursor {
            head: root,
            variable,
        },
        global,
    );
    crate::bytecode::eval_bytecode_with_env_ephemeral_global_lexicals(
        bytecode,
        selected,
        env.runtime,
    )
    .value
}

fn prepare_global_declarations(
    bytecode: &Bytecode,
    env: &mut crate::context::Context<'_>,
) -> Result<(), RuntimeError> {
    let Some(Value::Object(object)) = env.selected_global_this() else {
        for meta in bytecode.scope_declarations(ScopeId(0)) {
            if matches!(meta.kind, BindingKind::Var | BindingKind::Function)
                && !env.realm_contains(&meta.name)
            {
                env.insert_realm(meta.name.clone(), Value::Undefined);
            }
        }
        return Ok(());
    };
    // Validate everything before creating bindings. Function declarations are
    // installed first; newly declared vars follow, preserving property order.
    for function in [true, false] {
        for meta in bytecode.scope_declarations(ScopeId(0)) {
            if !matches!(meta.kind, BindingKind::Var | BindingKind::Function)
                || (meta.kind == BindingKind::Function) != function
            {
                continue;
            }
            let allowed = if function {
                super::can_declare_global_function(&object, &meta.name)
            } else {
                super::can_declare_global_var(&object, &meta.name)
            };
            if !allowed {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("TypeError: cannot declare global binding `{}`", meta.name),
                });
            }
        }
    }
    for function in [true, false] {
        for meta in bytecode.scope_declarations(ScopeId(0)) {
            if !matches!(meta.kind, BindingKind::Var | BindingKind::Function)
                || (meta.kind == BindingKind::Function) != function
            {
                continue;
            }
            if function || !object.has_own_property(&meta.name) {
                let property = match object.own_property(&meta.name) {
                    Some(existing) if !existing.configurable => existing,
                    _ => crate::Property::data(Value::Undefined, true, true, true),
                };
                object.define_property(meta.name.clone(), property);
            }
            if env.can_cache_realm_globals() {
                match object.own_property(&meta.name) {
                    Some(property) if !property.is_accessor() => {
                        env.insert_realm(meta.name.clone(), property.value);
                    }
                    _ => {
                        env.remove_realm(&meta.name);
                    }
                }
            }
        }
    }
    Ok(())
}
