use chain_evm::ethereum_types::H256;

/// The result of an `eth_getWork` call: it differs based on an option
/// whether to send the block number.
#[derive(Debug, PartialEq, Eq)]
pub struct Work {
    /// The proof-of-work hash.
    pub pow_hash: H256,
    /// The seed hash.
    pub seed_hash: H256,
    /// The target.
    pub target: H256,
    /// The block number: this isn't always stored.
    pub number: Option<u64>,
}
