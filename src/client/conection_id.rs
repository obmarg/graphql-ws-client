use std::num::NonZero;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// An opaque identifier for a subscription
///
/// Currently this wraps a `NonZero<usize>` though that may be subject to change
/// in the future - as a result the underlying type is not exposed publically
pub struct SubscriptionId(NonZero<usize>);

impl SubscriptionId {
    pub(super) fn new(id: usize) -> Option<Self> {
        Some(SubscriptionId(NonZero::new(id)?))
    }

    #[expect(clippy::inherent_to_string)] // Don't want this to be public, which implementing Display would make it.
    pub(super) fn to_string(self) -> String {
        self.0.to_string()
    }

    pub(super) fn from_str(s: &str) -> Option<Self> {
        SubscriptionId::new(s.parse::<usize>().ok()?)
    }
}
