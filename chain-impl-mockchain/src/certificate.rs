use crate::key::EitherEd25519SecretKey;
use crate::stake::{StakePoolId, StakePoolInfo};
use crate::transaction::AccountIdentifier;
use chain_core::mempack::{read_vec, ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519, PublicKey, Verification};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Clone)]
pub struct SignatureRaw(pub Vec<u8>);

impl property::Serialize for SignatureRaw {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        codec.put_u16(self.0.len() as u16)?;
        codec.into_inner().write_all(&self.0.as_ref())?;
        Ok(())
    }
}

impl Readable for SignatureRaw {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let u = buf.get_u16()?;
        let v = read_vec(buf, u as usize)?;
        Ok(SignatureRaw(v))
    }
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub content: CertificateContent,
    pub signatures: Vec<SignatureRaw>,
}

impl Certificate {
    pub fn sign(&mut self, secret_key: &EitherEd25519SecretKey) -> () {
        match &self.content {
            CertificateContent::StakeDelegation(v) => {
                let signature = v.make_certificate(secret_key);
                self.signatures.push(signature);
            }
            CertificateContent::StakePoolRegistration(v) => {
                let signature = v.make_certificate(secret_key);
                self.signatures.push(signature);
            }
            CertificateContent::StakePoolRetirement(v) => {
                let signature = v.make_certificate(secret_key);
                self.signatures.push(signature);
            }
        }
    }

    pub fn verify(&self) -> Verification {
        match &self.content {
            CertificateContent::StakeDelegation(v) => verify_certificate(v, &self.signatures),
            CertificateContent::StakePoolRegistration(v) => verify_certificate(v, &self.signatures),
            CertificateContent::StakePoolRetirement(v) => verify_certificate(v, &self.signatures),
        }
    }
}

/// Abstracts extracting public stake key identifiers
/// from a certificate.
pub(crate) trait HasPublicKeys<'a> {
    type PublicKeys: 'a + ExactSizeIterator<Item = &'a PublicKey<Ed25519>>;
    fn public_keys(self) -> Self::PublicKeys;
}

pub(crate) fn verify_certificate<'a, C>(
    _certificate: &'a C,
    _raw_signatures: &[SignatureRaw],
) -> Verification
where
    C: property::Serialize,
{
    Verification::Success
}

#[derive(Debug, Clone)]
pub enum CertificateContent {
    StakeDelegation(StakeDelegation),
    StakePoolRegistration(StakePoolInfo),
    StakePoolRetirement(StakePoolRetirement),
}

#[derive(FromPrimitive)]
enum CertificateTag {
    StakeDelegation = 1,
    StakePoolRegistration = 2,
    StakePoolRetirement = 3,
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        match &self.content {
            CertificateContent::StakeDelegation(s) => {
                codec.put_u8(CertificateTag::StakeDelegation as u8)?;
                s.serialize(&mut codec)
            }
            CertificateContent::StakePoolRegistration(s) => {
                codec.put_u8(CertificateTag::StakePoolRegistration as u8)?;
                s.serialize(&mut codec)
            }
            CertificateContent::StakePoolRetirement(s) => {
                codec.put_u8(CertificateTag::StakePoolRetirement as u8)?;
                s.serialize(&mut codec)
            }
        }?;
        codec.put_u8(self.signatures.len() as u8)?;
        for sig in &self.signatures {
            sig.serialize(&mut codec)?;
        }
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let tag = buf.get_u8()?;
        let content = match CertificateTag::from_u8(tag) {
            Some(CertificateTag::StakePoolRegistration) => {
                CertificateContent::StakePoolRegistration(StakePoolInfo::read(buf)?)
            }
            Some(CertificateTag::StakePoolRetirement) => {
                CertificateContent::StakePoolRetirement(StakePoolRetirement::read(buf)?)
            }
            Some(CertificateTag::StakeDelegation) => {
                CertificateContent::StakeDelegation(StakeDelegation::read(buf)?)
            }

            None => panic!("not a certificate"),
        };
        let len = buf.get_u8()?;
        let signatures = chain_core::mempack::read_vec(buf, len as usize)?;
        Ok(Certificate {
            content,
            signatures,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub stake_key_id: AccountIdentifier,
    pub pool_id: StakePoolId,
}

impl StakeDelegation {
    pub fn make_certificate(&self, stake_private_key: &EitherEd25519SecretKey) -> SignatureRaw {
        // FIXME: "It must be signed by sks_source, and that key must
        // be included in the witness." - why?
        use crate::key::make_signature;
        match stake_private_key {
            EitherEd25519SecretKey::Extended(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
            EitherEd25519SecretKey::Normal(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
        }
    }
}

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;
        let mut codec = Codec::new(writer);
        codec.write_all(self.stake_key_id.as_ref())?;
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let account_identifier = <[u8; 32]>::read(buf)?;
        Ok(StakeDelegation {
            stake_key_id: account_identifier.into(),
            pool_id: StakePoolId::read(buf)?,
        })
    }
}

impl StakePoolInfo {
    /// Create a certificate for this stake pool registration, signed
    /// by the pool's staking key and the owners.
    pub fn make_certificate(&self, pool_private_key: &EitherEd25519SecretKey) -> SignatureRaw {
        use crate::key::make_signature;
        match pool_private_key {
            EitherEd25519SecretKey::Extended(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
            EitherEd25519SecretKey::Normal(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_id: StakePoolId,
    // TODO: add epoch when the retirement will take effect
    pub pool_info: StakePoolInfo,
}

impl StakePoolRetirement {
    /// Create a certificate for this stake pool retirement, signed
    /// by the pool's staking key.
    pub fn make_certificate(&self, pool_private_key: &EitherEd25519SecretKey) -> SignatureRaw {
        use crate::key::make_signature;
        match pool_private_key {
            EitherEd25519SecretKey::Extended(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
            EitherEd25519SecretKey::Normal(sk) => {
                SignatureRaw(make_signature(sk, &self).as_ref().to_vec())
            }
        }
    }
}

impl property::Serialize for StakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.pool_id.serialize(&mut codec)?;
        self.pool_info.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakePoolRetirement {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakePoolRetirement {
            pool_id: StakePoolId::read(buf)?,
            pool_info: StakePoolInfo::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::leadership::genesis::GenesisPraosLeader;
    use chain_crypto::{Curve25519_2HashDH, PublicKey, SecretKey, SumEd25519_12};
    use lazy_static::lazy_static;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Certificate {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let content = match g.next_u32() % 3 {
                0 => CertificateContent::StakeDelegation(Arbitrary::arbitrary(g)),
                1 => CertificateContent::StakePoolRegistration(Arbitrary::arbitrary(g)),
                _ => CertificateContent::StakePoolRetirement(Arbitrary::arbitrary(g)),
            };
            let signatures = Arbitrary::arbitrary(g);
            Certificate {
                content,
                signatures,
            }
        }
    }

    impl Arbitrary for SignatureRaw {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignatureRaw(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for StakeDelegation {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeDelegation {
                stake_key_id: Arbitrary::arbitrary(g),
                pool_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakePoolInfo {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            lazy_static! {
                static ref PK_KES: PublicKey<SumEd25519_12> = {
                    let sk: SecretKey<SumEd25519_12> =
                        SecretKey::generate(&mut rand_chacha::ChaChaRng::from_seed([0; 32]));
                    sk.to_public()
                };
            }
            let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
            let vrf_sk: SecretKey<Curve25519_2HashDH> = SecretKey::generate(&mut rng);
            StakePoolInfo {
                serial: Arbitrary::arbitrary(g),
                owners: vec![Arbitrary::arbitrary(g)],
                initial_key: GenesisPraosLeader {
                    vrf_public_key: vrf_sk.to_public(),
                    kes_public_key: PK_KES.clone(),
                },
            }
        }
    }

    impl Arbitrary for StakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRetirement {
                pool_id: Arbitrary::arbitrary(g),
                pool_info: Arbitrary::arbitrary(g),
            }
        }
    }
}
