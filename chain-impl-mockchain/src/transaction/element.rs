use crate::key::deserialize_signature;
use crate::value::{Value, ValueError};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_crypto::{digest::DigestOf, Blake2b256, Signature, VerificationAlgorithm};

pub struct TransactionSignData(Box<[u8]>);

impl From<Vec<u8>> for TransactionSignData {
    fn from(v: Vec<u8>) -> TransactionSignData {
        TransactionSignData(v.into())
    }
}

impl AsRef<[u8]> for TransactionSignData {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub type TransactionSignDataHash = DigestOf<Blake2b256, TransactionSignData>;

pub struct TransactionBindingSignature<A: VerificationAlgorithm>(pub(super) Signature<u32, A>);

impl<A: VerificationAlgorithm> Clone for TransactionBindingSignature<A> {
    fn clone(&self) -> Self {
        TransactionBindingSignature(self.0.clone())
    }
}

impl<A: VerificationAlgorithm> AsRef<[u8]> for TransactionBindingSignature<A> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<A: VerificationAlgorithm> Readable for TransactionBindingSignature<A> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        deserialize_signature(buf).map(TransactionBindingSignature)
    }
}

/// Amount of the balance in the transaction.
pub enum Balance {
    /// Balance is positive.
    Positive(Value),
    /// Balance is negative, such transaction can't be valid.
    Negative(Value),
    /// Balance is zero.
    Zero,
}

type Filler = ();
#[allow(unused_variables)]
custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub BalanceError
        InputsTotalFailed { source: ValueError, filler: Filler } = @{{
            let _ = (source, filler);
            "failed to compute total input"
        }},
        OutputsTotalFailed { source: ValueError, filler: Filler } = @{{
            let _ = (source, filler);
            "failed to compute total output"
        }},
        NotBalanced { inputs: Value, outputs: Value }
            = "transaction value not balanced, has inputs sum {inputs} and outputs sum {outputs}",
}
