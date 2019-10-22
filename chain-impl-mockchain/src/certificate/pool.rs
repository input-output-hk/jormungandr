use crate::key::{deserialize_public_key, deserialize_signature};
use crate::leadership::genesis::GenesisPraosLeader;
use crate::rewards::TaxType;
use crate::transaction::Payload;
use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
use chain_crypto::{digest::DigestOf, Blake2b256, Ed25519, PublicKey, Signature, Verification};
use chain_time::{DurationSeconds, TimeOffsetSeconds};
use typed_bytes::{ByteArray, ByteBuilder};

/// Pool ID
pub type PoolId = DigestOf<Blake2b256, PoolRegistration>;

/// signatures with indices
pub type IndexSignatures<T> = Vec<(u16, Signature<ByteArray<T>, Ed25519>)>;

/// Pool information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRegistration {
    /// A random value, for user purpose similar to a UUID.
    /// it may not be unique over a blockchain, so shouldn't be used a unique identifier
    pub serial: u128,
    /// Beginning of validity for this pool, this is used
    /// to keep track of the period of the expected key and the expiry
    pub start_validity: TimeOffsetSeconds,
    /// Management threshold for owners, this need to be <= #owners and > 0
    pub management_threshold: u16,
    /// Owners of this pool
    pub owners: Vec<PublicKey<Ed25519>>,
    /// Rewarding
    pub rewards: TaxType,
    /// Genesis Praos keys
    pub keys: GenesisPraosLeader,
}

/// Updating info for a pool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolUpdate {
    pub pool_id: PoolId,
    pub start_validity: TimeOffsetSeconds,
    pub previous_keys: DigestOf<Blake2b256, GenesisPraosLeader>,
    pub updated_keys: GenesisPraosLeader,
}

/// Retirement info for a pool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRetirement {
    pub pool_id: PoolId,
    pub retirement_time: TimeOffsetSeconds,
}

/// Representant of a structure signed by a pool's owners
#[derive(Debug, Clone)]
pub struct PoolOwnersSigned<T: ?Sized> {
    pub signatures: IndexSignatures<T>,
}

impl PoolRegistration {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.u128(self.serial)
            .u64(self.start_validity.into())
            .u16(self.management_threshold)
            .iter16(&mut self.owners.iter(), |bb, o| bb.bytes(o.as_ref()))
            .sub(|sbb| self.rewards.serialize_in(sbb))
            .bytes(self.keys.vrf_public_key.as_ref())
            .bytes(self.keys.kes_public_key.as_ref())
    }
    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }

    pub fn to_id(&self) -> PoolId {
        let ba = self.serialize();
        DigestOf::digest_byteslice(&ba.as_byteslice())
    }
}

impl PoolUpdate {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.pool_id.as_ref())
            .u64(self.start_validity.into())
            .bytes(self.previous_keys.as_ref())
            .bytes(self.updated_keys.vrf_public_key.as_ref())
            .bytes(self.updated_keys.kes_public_key.as_ref())
    }

    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }
}

impl Readable for PoolUpdate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let pool_id = <[u8; 32]>::read(buf)?.into();
        let start_validity: DurationSeconds = buf.get_u64()?.into();
        let previous_keys = <[u8; 32]>::read(buf)?.into();
        let gpl = GenesisPraosLeader::read(buf)?;
        Ok(PoolUpdate {
            pool_id,
            start_validity: start_validity.into(),
            previous_keys,
            updated_keys: gpl,
        })
    }
}

impl PoolRetirement {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.pool_id.as_ref())
            .u64(self.retirement_time.into())
    }

    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }
}

impl Readable for PoolRetirement {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let pool_id = <[u8; 32]>::read(buf)?.into();
        let retirement_time = DurationSeconds::from(buf.get_u64()?).into();
        Ok(PoolRetirement {
            pool_id,
            retirement_time,
        })
    }
}

impl property::Serialize for PoolUpdate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.serialize().as_slice())?;
        Ok(())
    }
}

impl property::Serialize for PoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.serialize().as_slice())?;
        Ok(())
    }
}

impl Payload for PoolUpdate {
    const HAS_DATA : bool = true;
    const HAS_AUTH : bool = true;
    type Auth = PoolOwnersSigned<[u8]>;
}

impl Payload for PoolRetirement {
    const HAS_DATA : bool = true;
    const HAS_AUTH : bool = true;
    type Auth = PoolOwnersSigned<[u8]>;
}

impl property::Serialize for PoolRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.serialize().as_slice())?;
        Ok(())
    }
}

impl Readable for PoolRegistration {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let serial = buf.get_u128()?;
        let start_validity = DurationSeconds::from(buf.get_u64()?).into();
        let management_threshold = buf.get_u16()?;
        let owners_nb = buf.get_u16()?;

        let mut owners = Vec::with_capacity(owners_nb as usize);
        for _ in 0..owners_nb {
            owners.push(deserialize_public_key(buf)?);
        }

        let rewards = TaxType::read_frombuf(buf)?;
        let keys = GenesisPraosLeader::read(buf)?;

        let info = Self {
            serial,
            start_validity,
            management_threshold,
            owners,
            rewards,
            keys,
        };
        Ok(info)
    }
}

impl Payload for PoolRegistration {
    const HAS_DATA : bool = true;
    const HAS_AUTH : bool = true;
    type Auth = PoolOwnersSigned<[u8]>;
}

impl<T> PoolOwnersSigned<T> {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self>
    {
        bb.iter16(&mut self.signatures.iter(), |bb, (i, s)| {
            bb.u16(*i).bytes(s.as_ref())
        })
    }

    pub fn verify<F>(&self, pool_info: &PoolRegistration, verify_data: &[u8]) -> Verification
    {
        let signatories = self.signatures.len();

        if signatories < pool_info.management_threshold as usize {
            return Verification::Failed;
        }

        for (i, sig) in self.signatures.iter() {
            if *i as usize >= pool_info.owners.len() {
                return Verification::Failed;
            }
            let pk = &pool_info.owners[*i as usize];
            if sig.verify_slice(pk, verify_data) == Verification::Failed {
                return Verification::Failed;
            }
        }
        return Verification::Success;
    }
}

impl<T> Readable for PoolOwnersSigned<T> {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let sigs_nb = buf.get_u16()? as usize;
        let mut signatures = Vec::new();
        for _ in 0..sigs_nb {
            let nb = buf.get_u16()?;
            let sig = deserialize_signature(buf)?;
            signatures.push((nb, sig))
        }
        Ok(PoolOwnersSigned { signatures })
    }
}
