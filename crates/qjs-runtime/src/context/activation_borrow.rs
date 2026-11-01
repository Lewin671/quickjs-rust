//! Service views borrow a full activation or an indexed frame's inputs. Only
//! demand for semantic Context state allocates an owner; metadata queries do not.
use super::{ActivationContext, CreationContext};
use crate::{Function, Value};
use std::{
    cell::{Cell, OnceCell},
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct CallSlotRef(pub(crate) u32);
impl CallSlotRef {
    pub(crate) const ABSENT: Self = Self(0);
    pub(crate) const PREFIX: Self = Self(1);
    pub(crate) const LEXICAL: Self = Self(2);
    pub(crate) const CALLEE: Self = Self(3);
    pub(crate) const CONTEXT: Self = Self(4);
}
#[derive(Default)]
#[repr(C)]
pub(crate) struct CallRoutes {
    pub(crate) receiver: Cell<CallSlotRef>,
    pub(crate) target: Cell<CallSlotRef>,
    pub(crate) home: Cell<CallSlotRef>,
}
pub(crate) type ContextOwner = OnceCell<Box<ActivationContext>>;

#[derive(Clone, Copy)]
pub(crate) struct CallSeed<'a> {
    pub(crate) function: &'a Function,
    pub(crate) prefix: &'a [Value; 4],
}
impl CallSeed<'_> {
    fn creation(&self) -> &Rc<CreationContext> {
        self.function.creation_context()
    }
    fn receiver(&self) -> Option<Value> {
        match self.function.lexical_call() {
            Some(owner) => owner.this.read().ok(),
            None => Some(self.prefix[1].clone()),
        }
    }
    fn materialize(&self) -> ActivationContext {
        crate::diagnostics::count!(indexed_context_materializations);
        let mut context = crate::function::enter_indexed_activation_with_creation(
            self.function,
            self.creation().clone(),
        );
        crate::function::bind_indexed_call_owner(
            self.function,
            &mut context,
            self.prefix[1].clone(),
            (!matches!(self.prefix[2], Value::Undefined)).then(|| self.prefix[2].clone()),
        );
        context
    }
}
pub(crate) enum ActivationBorrow<'a> {
    Full(&'a mut ActivationContext),
    Indexed {
        state: &'a mut crate::bytecode::frame_bank::FrameContext,
        seed: CallSeed<'a>,
    },
}
impl<'a> From<&'a mut ActivationContext> for ActivationBorrow<'a> {
    fn from(value: &'a mut ActivationContext) -> Self {
        Self::Full(value)
    }
}
impl ActivationBorrow<'_> {
    #[inline(always)]
    pub(crate) fn reborrow(&mut self) -> ActivationBorrow<'_> {
        match self {
            Self::Full(value) => ActivationBorrow::Full(value),
            Self::Indexed { state, seed } => ActivationBorrow::Indexed { state, seed: *seed },
        }
    }
    #[inline(always)]
    pub(crate) fn creation_context_ref(&self) -> &Rc<CreationContext> {
        match self {
            Self::Full(value) => value.creation_context_ref(),
            Self::Indexed { state, seed } => state
                .owner
                .get()
                .map_or_else(|| seed.creation(), |ctx| ctx.creation_context_ref()),
        }
    }
    #[inline(always)]
    pub(crate) fn strict(&self) -> bool {
        match self {
            Self::Full(value) => value.strict,
            Self::Indexed { state, seed } => state
                .owner
                .get()
                .map_or(seed.function.is_strict, |ctx| ctx.strict),
        }
    }
    pub(crate) fn receiver_seed(&self) -> Option<Value> {
        match self {
            Self::Full(value) => value.receiver_seed(),
            Self::Indexed { state, seed } => state
                .owner
                .get()
                .map_or_else(|| seed.receiver(), |ctx| ctx.receiver_seed()),
        }
    }
    pub(crate) fn initialized_receiver(&self) -> Option<&Value> {
        match self {
            Self::Full(value) => value.initialized_receiver(),
            Self::Indexed { state, seed } => match state.owner.get() {
                Some(context) => context.initialized_receiver(),
                None if !seed.function.lexical_this => Some(&seed.prefix[1]),
                None => self.ensure().initialized_receiver(),
            },
        }
    }
    pub(crate) fn this_value(&self) -> Result<Value, crate::RuntimeError> {
        match self {
            Self::Full(value) => value.this_value(),
            Self::Indexed { state, seed } => match state.owner.get() {
                Some(context) => context.this_value(),
                None => match seed.function.lexical_call() {
                    Some(owner) => owner.this.read(),
                    None => Ok(seed.prefix[1].clone()),
                },
            },
        }
    }
    pub(crate) fn scope_cursor(&self) -> Option<&crate::binding::ScopeCursor> {
        match self {
            Self::Full(ctx) => ctx.scope_cursor(),
            Self::Indexed { state, .. } => state.owner.get().and_then(|ctx| ctx.scope_cursor()),
        }
    }
    pub(crate) fn has_module_imports(&self) -> bool {
        !self.creation_context_ref().imports().is_empty()
    }
    pub(crate) fn has_module_live_bindings(&self) -> bool {
        match self {
            Self::Full(ctx) => ctx.has_module_live_bindings(),
            Self::Indexed { state, .. } => state
                .owner
                .get()
                .is_some_and(|ctx| ctx.has_module_live_bindings()),
        }
    }
    pub(crate) fn has_saved_references(&self) -> bool {
        match self {
            Self::Full(ctx) => ctx.saved_references.is_some(),
            Self::Indexed { state, .. } => state
                .owner
                .get()
                .is_some_and(|ctx| ctx.saved_references.is_some()),
        }
    }
    pub(crate) fn has_module_import(&self, name: &str) -> bool {
        self.creation_context_ref().imports().contains_key(name)
    }
    pub(crate) fn module_import_cell(&self, name: &str) -> Option<crate::function::BindingCell> {
        let (bindings, exported) = self.creation_context_ref().imports().get(name)?;
        bindings.cell(exported)
    }
    pub(crate) fn module_live_binding_cell(
        &self,
        name: &str,
    ) -> Option<crate::function::BindingCell> {
        match self {
            Self::Full(ctx) => ctx.module_live_binding_cell(name),
            Self::Indexed { state, .. } => state
                .owner
                .get()
                .and_then(|ctx| ctx.module_live_binding_cell(name)),
        }
    }
    fn ensure(&self) -> &ActivationContext {
        match self {
            Self::Full(value) => value,
            Self::Indexed { state, seed } => state.owner.get_or_init(|| {
                let context = Box::new(seed.materialize());
                state.routes.receiver.set(CallSlotRef::CONTEXT);
                state.routes.target.set(CallSlotRef::CONTEXT);
                state.routes.home.set(CallSlotRef::CONTEXT);
                context
            }),
        }
    }
}
impl Deref for ActivationBorrow<'_> {
    type Target = ActivationContext;
    fn deref(&self) -> &ActivationContext {
        self.ensure()
    }
}
impl DerefMut for ActivationBorrow<'_> {
    fn deref_mut(&mut self) -> &mut ActivationContext {
        self.ensure();
        match self {
            Self::Full(value) => value,
            Self::Indexed { state, .. } => {
                state.owner.get_mut().expect("materialized service owner")
            }
        }
    }
}
