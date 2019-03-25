use crate::transaction::TransactionId;
use crate::value::Value;

use cardano::address::{AddrType, ExtendedAddr, SpendingData};
use cardano::hdwallet::XPub;

pub use cardano::address::Addr as OldAddress;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_core::property::Serialize;
use chain_crypto::{Ed25519Bip32, PublicKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoDeclaration {
    pub addrs: Vec<(OldAddress, Value)>,
}

pub fn oldaddress_from_xpub(address: &OldAddress, xpub: &PublicKey<Ed25519Bip32>) -> bool {
    match XPub::from_slice(xpub.as_ref()) {
        Err(_) => false,
        Ok(xpub_old) => {
            let a = address.deconstruct();
            let ea = ExtendedAddr::new(
                AddrType::ATPubKey,
                SpendingData::PubKeyASD(xpub_old),
                a.attributes.clone(),
            );
            ea == a
        }
    }
}

impl UtxoDeclaration {
    pub fn hash(&self) -> TransactionId {
        let v = self.serialize_as_vec().unwrap();
        TransactionId::hash_bytes(&v[..])
    }
}

impl Readable for UtxoDeclaration {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        use cardano::util::try_from_slice::TryFromSlice;

        let nb_entries = buf.get_u8()? as usize;
        if nb_entries >= 0xff {
            return Err(ReadError::StructureInvalid("nb entries".to_string()));
        }

        let mut addrs = Vec::with_capacity(nb_entries);
        for _ in 0..nb_entries {
            let value = Value::read(buf)?;
            let addr_size = buf.get_u16()? as usize;
            let addr = OldAddress::try_from_slice(buf.get_slice(addr_size)?).unwrap();
            addrs.push((addr, value))
        }

        Ok(UtxoDeclaration { addrs: addrs })
    }
}

impl property::Serialize for UtxoDeclaration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;

        assert!(self.addrs.len() < 255);

        let mut codec = Codec::from(writer);
        codec.put_u8(self.addrs.len() as u8)?;
        for (b, v) in &self.addrs {
            v.serialize(&mut codec)?;
            let bs = b.as_ref();
            codec.put_u16(bs.len() as u16)?;
            codec.write_all(bs)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cardano::address::ExtendedAddr;
    use cardano::config::NetworkMagic;
    use cardano::hdwallet::XPub;
    use cardano::hdwallet::XPUB_SIZE;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for UtxoDeclaration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut nb: usize = Arbitrary::arbitrary(g);
            nb = nb % 255;
            let mut addrs = Vec::with_capacity(nb);
            for _ in 0..nb {
                let value = Arbitrary::arbitrary(g);

                let xpub = {
                    let mut buf = [0u8; XPUB_SIZE];
                    for o in buf.iter_mut() {
                        *o = u8::arbitrary(g)
                    }
                    XPub::from_slice(&buf).unwrap()
                };
                let ea = ExtendedAddr::new_simple(xpub, NetworkMagic::NoMagic);
                let addr = ea.to_address();

                addrs.push((addr, value))
            }

            UtxoDeclaration { addrs }
        }
    }
}
