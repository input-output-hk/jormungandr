use crate::key::{deserialize_signature, EitherEd25519SecretKey};
use crate::transaction::TransactionBindingAuthData;
use crate::value::{Value, ValueError};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_crypto::{digest::DigestOf, Blake2b256, Ed25519, PublicKey, Signature, Verification};

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

#[derive(Debug, Clone)]
pub struct AccountBindingSignature(pub(super) Signature<u32, Ed25519>);

impl AccountBindingSignature {
    pub fn verify_slice<'a>(
        &self,
        pk: &PublicKey<Ed25519>,
        data: TransactionBindingAuthData<'a>,
    ) -> Verification {
        self.0.verify_slice(pk, data.0)
    }

    pub fn new<'a>(sk: &EitherEd25519SecretKey, data: TransactionBindingAuthData<'a>) -> Self {
        AccountBindingSignature(sk.sign_slice(data.0))
    }
}

impl AsRef<[u8]> for AccountBindingSignature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Readable for AccountBindingSignature {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        deserialize_signature(buf).map(AccountBindingSignature)
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
