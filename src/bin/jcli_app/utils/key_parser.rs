use bech32::{Bech32, FromBase32};
use chain_crypto::{AsymmetricKey, PublicKey};

pub fn parse_pub_key<A: AsymmetricKey>(s: &str) -> PublicKey<A> {
    let bech32: Bech32 = s.parse().unwrap();
    if bech32.hrp() == A::PUBLIC_BECH32_HRP {
        let pub_key_bytes = Vec::<u8>::from_base32(bech32.data()).unwrap();
        PublicKey::from_binary(&pub_key_bytes).unwrap()
    } else {
        panic!(
            "Invalid Key Type, received {} but was expecting {}",
            bech32.hrp(),
            A::PUBLIC_BECH32_HRP
        )
    }
}
