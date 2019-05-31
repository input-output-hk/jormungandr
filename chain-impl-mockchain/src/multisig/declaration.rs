use crate::{account, key};
use chain_crypto::{PublicKey, Signature};

use super::index::{Index, TreeIndex, LEVEL_MAXLIMIT};
pub use crate::transaction::WitnessMultisigData;

/// Account Identifier (also used as Public Key)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(key::Hash);

impl AsRef<[u8]> for Identifier {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; 32]> for Identifier {
    fn from(a: [u8; 32]) -> Self {
        Identifier(a.into())
    }
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub DeclarationError
        ThresholdInvalid = "Invalid threshold",
        HasNotEnoughOwners = "Not enough owners",
        HasTooManyOwners = "Too many owners",
        SubNotImplemented = "Sub not implemented",
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Declaration of a multisig account parameters which is:
///
/// * a threshold that need to be between 1 and the size of owners
/// * a bunch of owners which is either a hash of a key, or a sub declaration
#[derive(Debug, Clone)]
pub struct Declaration {
    pub(crate) threshold: u8, // between 1 and len(owners)
    pub(crate) owners: Vec<DeclElement>,
}

impl Declaration {
    pub fn threshold(&self) -> usize {
        self.threshold as usize
    }

    pub fn total(&self) -> usize {
        self.owners.len()
    }
}

#[derive(Debug, Clone)]
pub enum DeclElement {
    Sub(Declaration),
    Owner(key::Hash),
}

impl DeclElement {
    pub fn to_hash(&self) -> key::Hash {
        match self {
            DeclElement::Sub(d) => d.to_identifier().0,
            DeclElement::Owner(hash) => hash.clone(),
        }
    }

    pub fn from_publickey(key: &PublicKey<account::AccountAlg>) -> Self {
        DeclElement::Owner(key::Hash::hash_bytes(key.as_ref()))
    }
}

// Create an identifier by concatenating the threshold (as a byte) and all the owners
// and returning the hash of this content
pub(super) fn owners_to_identifier(threshold: u8, owners: &[DeclElement]) -> Identifier {
    let mut out = Vec::new();
    out.extend_from_slice(&[threshold]);
    for o in owners {
        out.extend_from_slice(o.to_hash().as_ref())
    }
    Identifier(key::Hash::hash_bytes(&out))
}

impl Declaration {
    /// Get the identifier associated with a declaration
    pub fn to_identifier(&self) -> Identifier {
        owners_to_identifier(self.threshold, &self.owners)
    }

    pub fn is_valid(&self) -> Result<(), DeclarationError> {
        if self.threshold < 1 || self.threshold as usize > self.owners.len() {
            return Err(DeclarationError::ThresholdInvalid);
        }
        if self.owners.len() <= 1 {
            return Err(DeclarationError::HasNotEnoughOwners);
        }
        if self.owners.len() > LEVEL_MAXLIMIT {
            return Err(DeclarationError::HasTooManyOwners);
        }
        Ok(())
    }

    pub fn get_path(&self, ti: TreeIndex) -> Option<(&Declaration, Index)> {
        match ti {
            TreeIndex::D1(idx) => Some((self, idx)),
            TreeIndex::D2(r, idx) if r.to_usize() < self.owners.len() => {
                match self.owners[r.to_usize()] {
                    DeclElement::Owner(_) => None,
                    DeclElement::Sub(ref d) => Some((d, idx)),
                }
            }
            TreeIndex::D2(_, _) => None,
        }
    }
}

pub type Pk = PublicKey<account::AccountAlg>;
pub type Sig = Signature<WitnessMultisigData, account::AccountAlg>;
