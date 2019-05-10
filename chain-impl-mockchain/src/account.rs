use crate::accounting::account;
use crate::key;
use chain_crypto::{Ed25519Extended, PublicKey};

pub use account::{LedgerError, SpendingCounter};

pub type AccountAlg = Ed25519Extended;

/// Account Identifier (also used as Public Key)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(PublicKey<AccountAlg>);

impl From<PublicKey<AccountAlg>> for Identifier {
    fn from(pk: PublicKey<AccountAlg>) -> Self {
        Identifier(pk)
    }
}

impl From<Identifier> for PublicKey<AccountAlg> {
    fn from(i: Identifier) -> Self {
        i.0
    }
}

/// Account Secret Key
pub type Secret = key::AccountSecretKey;

/// The public ledger of all accounts associated with their current state
pub type Ledger = account::Ledger<Identifier>;
