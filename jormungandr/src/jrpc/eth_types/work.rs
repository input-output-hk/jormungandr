use chain_evm::ethereum_types::H256;
use serde::{Serialize, Serializer};

/// Work
#[derive(Debug, PartialEq, Eq)]
pub struct Work {
    /// The proof-of-work hash.
    pub pow_hash: H256,
    /// The seed hash.
    pub seed_hash: H256,
    /// The target.
    pub target: H256,
}

impl Work {
    pub fn build() -> Self {
        Self {
            pow_hash: H256::zero(),
            seed_hash: H256::zero(),
            target: H256::zero(),
        }
    }
}

impl Serialize for Work {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        vec![self.pow_hash, self.seed_hash, self.target].serialize(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_serialize() {
        let work = Work {
            pow_hash: H256::zero(),
            seed_hash: H256::zero(),
            target: H256::zero(),
        };

        assert_eq!(
            serde_json::to_string(&work).unwrap(),
            r#"["0x0000000000000000000000000000000000000000000000000000000000000000","0x0000000000000000000000000000000000000000000000000000000000000000","0x0000000000000000000000000000000000000000000000000000000000000000"]"#
        );
    }
}
