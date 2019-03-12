use crate::value::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input {
    index_or_account: u8,
    value: Value,
    input_ptr: [u8; 32],
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Output<Address>(pub Address, pub Value);