use crate::key::{
    deserialize_signature, serialize_signature, Hash, SpendingPublicKey, SpendingSecretKey,
    SpendingSignature,
};
use crate::value::*;
use chain_core::property;
use chain_crypto::Verification;

// FIXME: should this be a wrapper type?
pub type TransactionId = Hash;

pub type TransactionIndex = u8;

/// Unspent transaction pointer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtxoPointer {
    /// the transaction identifier where the unspent output is
    pub transaction_id: TransactionId,
    /// the output index within the pointed transaction's outputs
    pub output_index: TransactionIndex,
    /// the value we expect to read from this output
    ///
    /// This setting is added in order to protect undesired withdrawal
    /// and to set the actual fee in the transaction.
    pub value: Value,
}

impl UtxoPointer {
    pub fn new(
        transaction_id: TransactionId,
        output_index: TransactionIndex,
        value: Value,
    ) -> Self {
        UtxoPointer {
            transaction_id,
            output_index,
            value,
        }
    }
}

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone)]
pub struct Witness(SpendingSignature<TransactionId>);

impl Witness {
    /// Creates new `Witness` value.
    pub fn new(transaction_id: &TransactionId, secret_key: &SpendingSecretKey) -> Self {
        Witness(SpendingSignature::generate(secret_key, transaction_id))
    }

    /// Verify the given `TransactionId` using the witness.
    pub fn verifies(
        &self,
        public_key: &SpendingPublicKey,
        transaction_id: &TransactionId,
    ) -> Verification {
        self.0.verify(public_key, transaction_id)
    }
}

impl PartialEq<Self> for Witness {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}
impl Eq for Witness {}

impl property::Serialize for Witness {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_signature(&self.0, writer)
    }
}

impl property::Deserialize for Witness {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        deserialize_signature(reader).map(Witness)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Clone)]
    pub struct TransactionSigningKey(pub SpendingSecretKey);

    impl std::fmt::Debug for TransactionSigningKey {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "TransactionSigningKey(<secret-key>)")
        }
    }

    impl Arbitrary for TransactionSigningKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            TransactionSigningKey(SpendingSecretKey::generate(&mut rng))
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let sk = TransactionSigningKey::arbitrary(g);
            let txid = TransactionId::arbitrary(g);
            Witness(SpendingSignature::generate(&sk.0, &txid))
        }
    }

    quickcheck! {

        /// ```
        /// \forall w=Witness(tx) => w.verifies(tx)
        /// ```
        fn prop_witness_verifies_own_tx(sk: TransactionSigningKey, tx:TransactionId) -> bool {
            let pk = sk.0.to_public();
            let witness = Witness::new(&tx, &sk.0);
            witness.verifies(&pk, &tx) == Verification::Success
        }
    }
}
