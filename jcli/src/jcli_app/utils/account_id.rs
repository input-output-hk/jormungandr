use bech32::{Bech32, FromBase32};
use chain_addr::{Address, Kind};
use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::account;
use hex;

#[derive(Debug)]
pub struct AccountId {
    arg: String,
    account: account::Identifier,
}

fn id_from_pub(pk: PublicKey<Ed25519>) -> account::Identifier {
    account::Identifier::from(pk)
}

impl AccountId {
    // accept either an address with the account kind
    // or a ed25519 publickey
    pub fn try_from_str(src: &str) -> Result<Self, Error> {
        use std::str::FromStr;
        if let Ok(b) = Bech32::from_str(src) {
            let dat = Vec::from_base32(b.data()).unwrap();
            if let Ok(addr) = Address::from_bytes(&dat) {
                match addr.kind() {
                    Kind::Account(pk) => Ok(Self {
                        arg: src.to_string(),
                        account: id_from_pub(pk.clone()),
                    }),
                    _ => Err(Error::AddressNotAccount {
                        addr: src.to_string(),
                        kind: format!("{:?}", addr.kind()),
                    }),
                }
            } else if let Ok(pk) = PublicKey::from_binary(&dat) {
                Ok(Self {
                    arg: src.to_string(),
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

custom_error! { pub Error
    NotRecognized { addr: String } = "account parameter '{addr}' isn't a valid address or publickey",
    AddressNotAccount { addr: String, kind: String } = "account parameter '{addr}' isn't an account address, found: '{kind}'",
}
