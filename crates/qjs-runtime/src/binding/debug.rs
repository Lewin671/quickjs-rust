//! Debug-only binding stress policy, captured once by RuntimeServices.
//! Production builds never inspect these environment variables.
#[derive(Clone, Copy, Default)]
pub(crate) struct BindingDebug {
    pub(crate) verify: bool,
    pub(crate) force: bool,
    pub(crate) poison: bool,
    pub(crate) poison_activations: bool,
}
impl BindingDebug {
    #[inline(always)]
    pub(crate) fn forcing(&self) -> bool {
        cfg!(debug_assertions) && self.force
    }
    #[inline(always)]
    pub(crate) fn verifying(&self) -> bool {
        cfg!(debug_assertions) && self.verify
    }
    #[inline(always)]
    pub(crate) fn poisoning(&self) -> bool {
        cfg!(debug_assertions) && self.poison
    }
    /// Released or pooled call activations carry a checked poison.
    #[inline(always)]
    pub(crate) fn poisoning_activations(&self) -> bool {
        cfg!(debug_assertions) && self.poison_activations
    }

    pub(crate) fn capture() -> Self {
        #[cfg(debug_assertions)]
        {
            let enabled = |name| std::env::var_os(name).is_some_and(|value| value == "1");
            Self {
                verify: enabled("QJS_VERIFY_BINDINGS"),
                force: enabled("QJS_FORCE_EVAL_EXPOSURE"),
                poison: enabled("QJS_POISON_RELEASED_SCOPES"),
                poison_activations: enabled("QJS_POISON_RELEASED_ACTIVATIONS"),
            }
        }
        #[cfg(not(debug_assertions))]
        Self::default()
    }
}
