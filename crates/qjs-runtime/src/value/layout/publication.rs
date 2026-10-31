//! Publication state is independent of descriptor mutability. Private bound
//! layouts can mint their own version while retaining on-demand root metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Publication {
    Ready = 0,
    PrivateTransient = 1,
    Deferred = 2,
    Transient = 3,
}
impl Publication {
    pub(super) fn is_deferred(self) -> bool {
        self == Self::Deferred
    }
    pub(super) fn is_transient(self) -> bool {
        matches!(self, Self::PrivateTransient | Self::Transient)
    }
    pub(super) fn is_pending(self) -> bool {
        matches!(self, Self::Deferred | Self::Transient)
    }
    pub(super) fn copied(self, dictionary: bool) -> Self {
        match (self.is_transient(), dictionary) {
            (true, true) => Self::PrivateTransient,
            (true, false) => Self::Transient,
            (false, _) => Self::Ready,
        }
    }
}
