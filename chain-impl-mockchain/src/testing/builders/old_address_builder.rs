use crate::{legacy::UtxoDeclaration, quickcheck::RngCore, value::Value};
use cardano_legacy_address::Addr;
use cardano_legacy_address::ExtendedAddr;
use ed25519_bip32::{XPub, XPUB_SIZE};

#[derive(Default)]
pub struct OldAddressBuilder;

impl OldAddressBuilder {
    pub fn build_utxo_declaration(size: Option<usize>) -> UtxoDeclaration {
        let nb = match size {
            Some(size) => size,
            None => {
                let mut rng = rand_os::OsRng::new().unwrap();
                let nb: usize = rng.next_u32() as usize;
                nb % 255
            }
        };
        let mut addrs = Vec::with_capacity(nb);
        for _ in 0..nb {
            addrs.push(Self::build_old_address());
        }
        UtxoDeclaration { addrs }
    }

    pub fn build_old_address() -> (Addr, Value) {
        // some reasonable value
        let mut rng = rand_os::OsRng::new().unwrap();
        let value = Value(rng.next_u64() % 2_000_000 + 1);
        let xpub = {
            let mut buf = [0u8; XPUB_SIZE];
            rng.fill_bytes(&mut buf);
            match XPub::from_slice(&buf) {
                Ok(xpub) => xpub,
                Err(_) => panic!("xpub not built correctly"),
            }
        };
        let ea = ExtendedAddr::new_simple(&xpub, None);
        (ea.to_address(), value)
    }
}
