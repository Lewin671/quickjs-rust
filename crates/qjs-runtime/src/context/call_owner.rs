//! An activation owns call metadata inline until the first lexical retention.
//! Promotion moves that owner into one Rc and redirects the original; arrows
//! and eval share it. Only derived-this initialization mutates retained state.
use crate::{RuntimeError, Value, binding::BindingCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnerKind {
    Script,
    Module,
    Function,
    Field,
    StaticBlock,
}
impl OwnerKind {
    pub(crate) fn in_function(self) -> bool {
        matches!(self, Self::Function | Self::Field | Self::StaticBlock)
    }
    pub(crate) fn forbids_arguments(self) -> bool {
        matches!(self, Self::Field | Self::StaticBlock)
    }
}

#[derive(Debug)]
pub(crate) enum ThisBinding {
    Initialized(Value),
    Derived(BindingCell),
}
impl ThisBinding {
    pub(crate) fn read(&self) -> Result<Value, RuntimeError> {
        match self {
            Self::Initialized(value) => Ok(value.clone()),
            Self::Derived(cell) => cell.read().map_err(|_| RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing this".into(),
            }),
        }
    }
    pub(crate) fn derived_cell(&self) -> Option<BindingCell> {
        match self {
            Self::Derived(cell) => Some(cell.clone()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CallOwner {
    pub(crate) this: ThisBinding,
    pub(crate) new_target: Option<Value>,
    pub(crate) kind: OwnerKind,
    /// Method/class state, absent for ordinary calls so their owner stays
    /// small to build, move and drop.
    pub(crate) class: Option<Box<ClassOwner>>,
}

/// The method home object, the class heritage and the derived constructor an
/// owner carries only when one of them exists.
#[derive(Debug, Default)]
pub(crate) struct ClassOwner {
    pub(crate) home: Option<Value>,
    pub(crate) super_constructor: Option<Value>,
    pub(crate) active_constructor: Option<Value>,
}

impl CallOwner {
    pub(crate) fn root(this: Value, kind: OwnerKind) -> Self {
        Self {
            this: ThisBinding::Initialized(this),
            new_target: None,
            kind,
            class: None,
        }
    }

    /// Records class state, allocating it only when some part exists.
    pub(crate) fn set_class_state(
        &mut self,
        home: Option<Value>,
        super_constructor: Option<Value>,
        active_constructor: Option<Value>,
    ) {
        self.class = (home.is_some()
            || super_constructor.is_some()
            || active_constructor.is_some())
        .then(|| {
            Box::new(ClassOwner {
                home,
                super_constructor,
                active_constructor,
            })
        });
    }

    pub(crate) fn home(&self) -> Option<&Value> {
        self.class.as_ref()?.home.as_ref()
    }

    pub(crate) fn super_constructor(&self) -> Option<&Value> {
        self.class.as_ref()?.super_constructor.as_ref()
    }

    pub(crate) fn active_constructor(&self) -> Option<&Value> {
        self.class.as_ref()?.active_constructor.as_ref()
    }
}

#[derive(Debug, Default)]
pub(crate) enum LexicalCallContext {
    #[default]
    None,
    Inline(CallOwner),
    Shared(Rc<CallOwner>),
}
impl LexicalCallContext {
    pub(crate) fn owner(&self) -> Option<&CallOwner> {
        match self {
            Self::None => None,
            Self::Inline(owner) => Some(owner),
            Self::Shared(owner) => Some(owner),
        }
    }
    pub(crate) fn capture(&mut self) -> Option<Rc<CallOwner>> {
        match self {
            Self::None => None,
            Self::Shared(owner) => Some(owner.clone()),
            Self::Inline(_) => {
                let Self::Inline(owner) = std::mem::take(self) else {
                    unreachable!()
                };
                crate::diagnostics::count!(lexical_owner_allocations);
                let owner = Rc::new(owner);
                *self = Self::Shared(owner.clone());
                Some(owner)
            }
        }
    }
    pub(crate) fn shared_view(&mut self) -> Self {
        self.capture().map_or(Self::None, Self::Shared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn promotion_moves_one_owner_and_reuses_its_identity() {
        let mut call =
            LexicalCallContext::Inline(CallOwner::root(Value::Number(3.0), OwnerKind::Function));
        let first = call.capture().unwrap();
        let second = call.capture().unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        let LexicalCallContext::Shared(original) = &call else {
            panic!("owner not redirected");
        };
        assert!(Rc::ptr_eq(original, &first));
        assert_eq!(first.this.read(), Ok(Value::Number(3.0)));
    }
    #[test]
    fn retained_derived_this_can_only_initialize_once() {
        let cell = BindingCell::with_metadata(
            crate::binding::BindingMetaRef::dynamic("this".into(), true, false),
            None,
        );
        let mut owner = CallOwner::root(Value::Undefined, OwnerKind::Function);
        owner.this = ThisBinding::Derived(cell.clone());
        let mut call = LexicalCallContext::Inline(owner);
        let captured = call.capture().unwrap();
        assert!(captured.this.read().is_err());
        cell.initialize(Value::Number(8.0)).unwrap();
        assert_eq!(captured.this.read(), Ok(Value::Number(8.0)));
        assert!(cell.initialize(Value::Number(9.0)).is_err());
    }
}
