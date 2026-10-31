//! Immutable declaration and exposure authority, recorded during compilation.
//!
//! Slot indices and scope IDs remain stable for the lifetime of a compiled
//! body. No descriptor owns a live value or a reference into an activation.

use crate::value::name_hash::NameMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingId(pub(crate) usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScopeId(pub(crate) usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalSlot(pub(crate) usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Script,
    Module,
    Eval,
    Parameters,
    FunctionBody,
    FunctionName,
    Block,
    Catch,
    Loop,
    ClassName,
    With,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingKind {
    ObjectStorage,
    Parameter,
    Arguments,
    Var,
    Function,
    Lexical,
    Catch,
    Received,
    FunctionName,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InitPolicy {
    Uninitialized,
    Undefined,
    Parameter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WritePolicy {
    Mutable,
    Throw,
    IgnoreSloppy,
}

#[derive(Clone, Debug)]
pub(crate) struct BindingMeta {
    pub(crate) id: BindingId,
    pub(crate) name: String,
    pub(crate) kind: BindingKind,
    pub(crate) mutable: bool,
    pub(crate) strict_write: WritePolicy,
    pub(crate) deletable: bool,
    pub(crate) initialization: InitPolicy,
    pub(crate) slot: LocalSlot,
}

#[derive(Clone, Debug)]
pub(crate) struct ScopeDescriptor {
    pub(crate) id: ScopeId,
    pub(crate) parent: Option<ScopeId>,
    pub(crate) kind: ScopeKind,
    pub(crate) declarations: Box<[BindingId]>,
    pub(crate) lookup: NameMap<String, BindingId>,
    pub(crate) variable_target: ScopeId,
}

/// The expression's immutable self environment, compiled once per body.
#[derive(Clone, Debug)]
pub(crate) struct FunctionNamePlan {
    pub(crate) binding: BindingMetaRef,
    pub(crate) scopes: std::rc::Rc<super::ScopeDescriptors>,
}
impl FunctionNamePlan {
    pub(crate) fn compile(name: String) -> std::rc::Rc<Self> {
        let binding = BindingMetaRef::function_name(name.clone());
        let id = ScopeId(0);
        let descriptor = ScopeDescriptor {
            id,
            parent: None,
            kind: ScopeKind::FunctionName,
            declarations: vec![binding.id].into_boxed_slice(),
            lookup: [(name, binding.id)].into_iter().collect(),
            variable_target: id,
        };
        std::rc::Rc::new(Self {
            binding,
            scopes: std::rc::Rc::new(super::ScopeDescriptors(vec![descriptor].into_boxed_slice())),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExposureSite {
    pub(crate) pc: usize,
    pub(crate) scopes: Box<[ScopeId]>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EvalExposurePlan {
    pub(crate) sites: Box<[ExposureSite]>,
}

/// The compiler's scope bookkeeping. It is consumed by BindingPlan, never
/// retained by a frame. Names are registered where the compiler declares them,
/// so shadowed slots and future TDZ declarations keep their source identity.
#[derive(Clone, Debug)]
pub(crate) struct ScopePlanBuilder {
    pub(crate) scopes: Vec<ScopeDescriptor>,
    pub(crate) active: Vec<ScopeId>,
    pub(crate) declarations: Vec<(String, LocalSlot, ScopeId)>,
    pub(crate) declaration_by_slot: Vec<Option<usize>>,
    pub(crate) sites: Vec<ExposureSite>,
    pub(crate) variable: ScopeId,
    pub(crate) implicit_arguments: Option<LocalSlot>,
    pub(crate) pc_scopes: Vec<ScopeId>,
}

impl Default for ScopePlanBuilder {
    fn default() -> Self {
        Self {
            scopes: vec![ScopeDescriptor {
                id: ScopeId(0),
                parent: None,
                kind: ScopeKind::Script,
                declarations: Box::default(),
                lookup: NameMap::default(),
                variable_target: ScopeId(0),
            }],
            active: vec![ScopeId(0)],
            declarations: Vec::new(),
            declaration_by_slot: Vec::new(),
            sites: Vec::new(),
            variable: ScopeId(0),
            implicit_arguments: None,
            pc_scopes: Vec::new(),
        }
    }
}

impl ScopePlanBuilder {
    pub(crate) fn enter_function_body(&mut self, split_parameters: bool) {
        let variable = self.variable;
        self.enter(ScopeKind::FunctionBody);
        if !split_parameters {
            self.variable = variable;
            self.scopes.last_mut().expect("body scope").variable_target = variable;
        }
    }

    pub(crate) fn enter(&mut self, kind: ScopeKind) {
        let id = ScopeId(self.scopes.len());
        let is_variable = matches!(kind, ScopeKind::FunctionBody | ScopeKind::Parameters);
        if is_variable {
            self.variable = id;
        }
        self.scopes.push(ScopeDescriptor {
            id,
            parent: self.active.last().copied(),
            kind,
            declarations: Box::default(),
            lookup: NameMap::default(),
            variable_target: self.variable,
        });
        self.active.push(id);
    }

    pub(crate) fn exit(&mut self) {
        self.active.pop().expect("balanced compiler scope");
        self.variable =
            self.scopes[self.active.last().expect("root scope remains").0].variable_target;
    }

    pub(crate) fn declare(&mut self, name: &str, slot: usize, lexical: bool) {
        let scope = if lexical {
            *self.active.last().expect("compiler scope")
        } else {
            self.variable
        };
        if self.declaration_by_slot.len() <= slot {
            self.declaration_by_slot.resize(slot + 1, None);
        }
        if self.declaration_by_slot[slot].is_none() {
            self.declaration_by_slot[slot] = Some(self.declarations.len());
            self.declarations
                .push((name.to_owned(), LocalSlot(slot), scope));
        }
    }

    pub(crate) fn record_pc(&mut self, pc: usize) {
        let scope = *self.active.last().expect("compiler scope");
        self.pc_scopes.resize(pc + 1, scope);
        self.pc_scopes[pc] = scope;
    }

    pub(crate) fn exposure_site(&mut self, pc: usize) {
        self.sites.push(ExposureSite {
            pc,
            scopes: self.active.clone().into_boxed_slice(),
        });
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BindingMetadata(pub(crate) Box<[BindingMeta]>);

impl std::ops::Deref for BindingMetadata {
    type Target = [BindingMeta];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BindingMetaRef {
    pub(crate) owner: std::rc::Rc<BindingMetadata>,
    pub(crate) id: BindingId,
}

impl std::ops::Deref for BindingMetaRef {
    type Target = BindingMeta;
    fn deref(&self) -> &Self::Target {
        &self.owner.0[self.id.0]
    }
}
