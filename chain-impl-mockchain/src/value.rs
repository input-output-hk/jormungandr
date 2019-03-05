/// Unspent transaction value.
#[cfg_attr(feature = "generic-serialization", derive(serde_derive::Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u64);
