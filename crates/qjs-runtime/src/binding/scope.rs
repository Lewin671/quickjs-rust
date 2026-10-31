//! Escaping lexical environments and own eval declaration holders.
//!
//! Leases retain immutable descriptors and positional cells, never a frame or
//! a slot pointer. An extensible variable holder owns only its declarations;
//! walking parent leases keeps later parent additions visible without forks.

use super::{BindingCell, BindingMetaRef, ScopeDescriptor, ScopeId};
use crate::{RuntimeError, Value, value::name_hash::NameMap};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VariableEnvIdentity {
    pub(crate) activation: u64,
    pub(crate) boundary: ScopeId,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ScopeDescriptors(pub(crate) Box<[ScopeDescriptor]>);
impl std::ops::Deref for ScopeDescriptors {
    type Target = [ScopeDescriptor];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ScopeDescriptorRef {
    pub(crate) owner: Rc<ScopeDescriptors>,
    pub(crate) id: ScopeId,
}
impl std::ops::Deref for ScopeDescriptorRef {
    type Target = ScopeDescriptor;
    fn deref(&self) -> &Self::Target {
        &self.owner[self.id.0]
    }
}

#[derive(Debug)]
pub(crate) struct VarHolder {
    pub(crate) identity: VariableEnvIdentity,
    entries: RefCell<NameMap<String, BindingCell>>,
    structure: Cell<u64>,
}

impl VarHolder {
    pub(crate) fn new(identity: VariableEnvIdentity) -> Self {
        Self {
            identity,
            entries: RefCell::default(),
            structure: Cell::new(0),
        }
    }
    pub(crate) fn cell(&self, name: &str) -> Option<BindingCell> {
        self.entries.borrow().get(name).cloned()
    }
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.entries.borrow().contains_key(name)
    }
    pub(crate) fn insert_cell(&self, name: String, cell: BindingCell) {
        self.entries.borrow_mut().insert(name, cell);
        self.changed();
    }

    pub(crate) fn declare(&self, name: &str) -> BindingCell {
        if let Some(cell) = self.cell(name) {
            return cell;
        }
        let cell = BindingCell::with_metadata(
            BindingMetaRef::dynamic(name.to_owned(), true, true),
            Some(Value::Undefined),
        );
        self.entries
            .borrow_mut()
            .insert(name.to_owned(), cell.clone());
        self.changed();
        cell
    }
    pub(crate) fn delete(&self, name: &str) -> bool {
        let mut entries = self.entries.borrow_mut();
        if entries
            .get(name)
            .is_some_and(|cell| !cell.metadata().deletable)
        {
            return false;
        }
        let removed = entries.remove(name).is_some();
        drop(entries);
        if removed {
            self.changed();
        }
        true
    }
    fn changed(&self) {
        let Some(next) = self.structure.get().checked_add(1) else {
            std::process::abort();
        };
        self.structure.set(next);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessPolicy {
    Mutable,
    Import,
}

#[derive(Clone, Debug)]
pub(crate) struct CellReference {
    pub(crate) cell: BindingCell,
    pub(crate) access: AccessPolicy,
}
impl CellReference {
    pub(crate) fn read(&self) -> Result<Value, RuntimeError> {
        self.cell.read()
    }
    pub(crate) fn assign(&self, value: Value, strict: bool) -> Result<(), RuntimeError> {
        if self.access == AccessPolicy::Import {
            // An indirect import binding rejects writes without reading the
            // exporter's cell; its TDZ is observable only through GetBindingValue.
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        self.cell.assign(value, strict)
    }
}

#[derive(Debug)]
pub(crate) struct ScopeLease {
    pub(crate) descriptor: ScopeDescriptorRef,
    pub(crate) parent: Option<Rc<ScopeLease>>,
    pub(crate) cells: Box<[CellReference]>,
    pub(crate) object: Option<Value>,
    pub(crate) own_vars: Option<Rc<VarHolder>>,
}
impl ScopeLease {
    pub(crate) fn binding(&self, name: &str) -> Option<CellReference> {
        let id = self.descriptor.lookup.get(name)?;
        let ordinal = self
            .descriptor
            .declarations
            .binary_search_by_key(&id.0, |declared| declared.0)
            .ok()?;
        let binding = self.cells.get(ordinal)?;
        if self.descriptor.kind == super::ScopeKind::Eval
            && matches!(
                binding.cell.metadata().kind,
                super::BindingKind::Var | super::BindingKind::Function
            )
        {
            return None;
        }
        Some(binding.clone())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ScopeCursor {
    pub(crate) head: Rc<ScopeLease>,
    pub(crate) variable: Rc<ScopeLease>,
}
impl ScopeCursor {
    /// Fixed declarative mappings can be omitted when the child's compiled
    /// routes cannot select any of them. Extensible holders and with objects
    /// always require live crossing lookup, including future declarations.
    pub(crate) fn needs_crossing_lookup(&self, mut name_is_free: impl FnMut(&str) -> bool) -> bool {
        let mut current = Some(self.head.as_ref());
        while let Some(scope) = current {
            if scope.object.is_some()
                || scope.own_vars.is_some()
                || scope
                    .cells
                    .iter()
                    .any(|binding| name_is_free(&binding.cell.metadata().name))
            {
                return true;
            }
            current = scope.parent.as_deref();
        }
        false
    }

    /// An expression's self name belongs outside its parameter environment.
    /// Calls and escaping descendants share its cell; ordinary body bindings
    /// can shadow it without changing it.
    pub(crate) fn with_function_name(
        parent: Option<Self>,
        cell: BindingCell,
        plan: &super::FunctionNamePlan,
    ) -> Self {
        let id = ScopeId(0);
        crate::diagnostics::count!(scope_lease_allocations);
        let lease = Rc::new(ScopeLease {
            descriptor: ScopeDescriptorRef {
                owner: plan.scopes.clone(),
                id,
            },
            parent: parent.as_ref().map(|cursor| cursor.head.clone()),
            cells: vec![CellReference {
                cell,
                access: AccessPolicy::Mutable,
            }]
            .into_boxed_slice(),
            object: None,
            own_vars: None,
        });
        Self {
            head: lease.clone(),
            variable: parent.map_or(lease, |cursor| cursor.variable),
        }
    }

    /// Declaration ownership never follows a lookup into a parent holder.
    pub(crate) fn declare_var(&self, name: &str) -> BindingCell {
        debug_assert_eq!(
            self.variable
                .own_vars
                .as_ref()
                .expect("variable holder")
                .identity
                .boundary,
            self.variable.descriptor.id
        );
        if let Some(binding) = self.variable.binding(name) {
            return binding.cell;
        }
        self.variable
            .own_vars
            .as_ref()
            .expect("variable lease owns its holder")
            .declare(name)
    }
    pub(crate) fn validate_eval_declaration(
        &self,
        name: &str,
        in_parameters: bool,
    ) -> Result<(), RuntimeError> {
        let mut scope = Some(self.head.clone());
        while let Some(lease) = scope {
            if let Some(binding) = lease.binding(name) {
                let kind = binding.cell.metadata().kind;
                if kind == super::BindingKind::Lexical
                    || in_parameters
                        && matches!(
                            kind,
                            super::BindingKind::Parameter | super::BindingKind::Arguments
                        )
                {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("SyntaxError: eval declaration conflicts with `{name}`"),
                    });
                }
            }
            if Rc::ptr_eq(&lease, &self.variable) {
                break;
            }
            scope = lease.parent.clone();
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn lookup(&self, name: &str) -> Option<CellReference> {
        let mut scope = Some(self.head.clone());
        while let Some(lease) = scope {
            if let Some(binding) = lease.binding(name) {
                return Some(binding);
            }
            if let Some(cell) = lease.own_vars.as_ref().and_then(|holder| holder.cell(name)) {
                return Some(CellReference {
                    cell,
                    access: AccessPolicy::Mutable,
                });
            }
            scope = lease.parent.clone();
        }
        None
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedReference {
    Global(super::GlobalReference),
    Cell {
        binding: CellReference,
    },
    Holder {
        holder: Rc<VarHolder>,
        name: String,
    },
    Property {
        base: Value,
        name: String,
        strict: bool,
        with_environment: bool,
    },
    Unresolvable,
}

impl ScopeCursor {
    pub(crate) fn resolve(
        &self,
        name: &str,
        strict: bool,
        env: &mut crate::context::Context<'_>,
    ) -> Result<ResolvedReference, RuntimeError> {
        let mut scope = Some(self.head.clone());
        while let Some(lease) = scope {
            if let Some(object) = &lease.object {
                if crate::property::has_property(object.clone(), env, name)? {
                    let blocked = match crate::symbol::unscopables_symbol(env) {
                        Some(symbol) => {
                            let unscopables = crate::property_value_key(
                                object.clone(),
                                &crate::PropertyKey::Symbol(symbol),
                                env,
                            )?;
                            if matches!(
                                unscopables,
                                Value::Object(_)
                                    | Value::Array(_)
                                    | Value::Function(_)
                                    | Value::Proxy(_)
                            ) {
                                crate::is_truthy(&crate::property_value(unscopables, name, env)?)
                            } else {
                                false
                            }
                        }
                        None => false,
                    };
                    if !blocked {
                        return Ok(ResolvedReference::Property {
                            base: object.clone(),
                            name: name.to_owned(),
                            strict,
                            with_environment: true,
                        });
                    }
                }
            } else if let Some(binding) = lease.binding(name) {
                return Ok(ResolvedReference::Cell { binding });
            }
            if let Some(holder) = &lease.own_vars
                && holder.contains(name)
            {
                return Ok(ResolvedReference::Holder {
                    holder: holder.clone(),
                    name: name.to_owned(),
                });
            }
            scope = lease.parent.clone();
        }
        Ok(ResolvedReference::Unresolvable)
    }
}

impl ResolvedReference {
    pub(crate) fn read(
        &self,
        env: &mut crate::context::Context<'_>,
    ) -> Result<Value, RuntimeError> {
        match self {
            Self::Global(reference) => reference.read(env),
            Self::Cell { binding } => binding.read(),
            Self::Holder { holder, name } => holder
                .cell(name)
                .ok_or_else(|| missing_binding(name))?
                .read(),
            Self::Property {
                base, name, strict, ..
            } => {
                if !crate::property::has_property(base.clone(), env, name)? {
                    if *strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    }
                    return Ok(Value::Undefined);
                }
                crate::property_value(base.clone(), name, env)
            }
            Self::Unresolvable => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: unresolvable binding".to_owned(),
            }),
        }
    }
    pub(crate) fn assign(
        &self,
        value: Value,
        strict: bool,
        env: &mut crate::context::Context<'_>,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Global(reference) => reference.assign(value, env),
            Self::Cell { binding } => binding.assign(value, strict),
            Self::Holder { holder, name } => {
                let cell = match holder.cell(name) {
                    Some(cell) => cell,
                    None if strict => return Err(missing_binding(name)),
                    None => holder.declare(name),
                };
                cell.assign(value, strict)
            }
            Self::Property {
                base, name, strict, ..
            } => {
                if !crate::property::has_property(base.clone(), env, name)? && *strict {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                let written =
                    crate::bytecode::set_object_property(base.clone(), name.clone(), value, env)?;
                if let Value::Object(object) = base {
                    env.sync_realm_global_object_property(object, name);
                }
                if !written && *strict {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: cannot set property".to_owned(),
                    });
                }
                Ok(())
            }
            Self::Unresolvable => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: unresolvable binding".to_owned(),
            }),
        }
    }
    pub(crate) fn delete(
        &self,
        env: &mut crate::context::Context<'_>,
    ) -> Result<bool, RuntimeError> {
        match self {
            Self::Global(reference) => reference.delete(env),
            Self::Holder { holder, name } => Ok(holder.delete(name)),
            Self::Cell { .. } => Ok(false),
            Self::Property { base, name, .. } => crate::bytecode::delete_object_property(
                base.clone(),
                &crate::PropertyKey::String(name.clone()),
                env,
            ),
            Self::Unresolvable => Ok(true),
        }
    }
}

fn missing_binding(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("ReferenceError: undefined identifier `{name}`"),
    }
}
