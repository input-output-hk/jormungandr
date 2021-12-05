use bech32::{self, FromBase32};
use chain_addr::{Address, Kind};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::account;
use thiserror::Error;

#[derive(Debug)]
pub struct AccountId {
    account: account::Identifier,
}

fn id_from_pub(pk: PublicKey<Ed25519>) -> account::Identifier {
    account::Identifier::from(pk)
}

impl AccountId {
    // accept either an address with the account kind
    // or a ed25519 publickey
    pub fn try_from_str(src: &str) -> Result<Self, Error> {
        if let Ok((_, data, _variant)) = bech32::decode(src) {
            let dat = Vec::from_base32(&data).unwrap();
            if let Ok(addr) = Address::from_bytes(&dat) {
                match addr.kind() {
                    Kind::Account(pk) => Ok(Self {
                        account: id_from_pub(pk.clone()),
                    }),
                    _ => Err(Error::AddressNotAccount {
                        addr: src.to_string(),
                        kind: format!("{:?}", addr.kind()),
                    }),
                }
            } else if let Ok(pk) = PublicKey::from_binary(&dat) {
                Ok(Self {
                    account: id_from_pub(pk),
                })
            } else {
                Err(Error::NotRecognized {
                    addr: src.to_string(),
                })
            }
        } else {
            Err(Error::NotRecognized {
                addr: src.to_string(),
            })
        }
    }

    // account id is encoded in hexadecimal in url argument
    pub fn to_url_arg(&self) -> String {
        hex::encode(self.account.as_ref().as_ref())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("account parameter '{addr}' isn't a valid address or publickey")]
    NotRecognized { addr: String },
    #[error("account parameter '{addr}' isn't an account address, found: '{kind}'")]
    AddressNotAccount { addr: String, kind: String },
}
