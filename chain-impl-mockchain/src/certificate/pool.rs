use super::CertificateSlice;
use crate::key::{deserialize_public_key, deserialize_signature};
use crate::leadership::genesis::GenesisPraosLeader;
use crate::rewards::TaxType;
use crate::transaction::{
    AccountBindingSignature, Payload, PayloadAuthData, PayloadData, PayloadSlice,
    TransactionBindingAuthData,
};
use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
use chain_crypto::{digest::DigestOf, Blake2b256, Ed25519, PublicKey, Verification};
use chain_time::{DurationSeconds, TimeOffsetSeconds};
use std::marker::PhantomData;
use typed_bytes::{ByteArray, ByteBuilder};

/// Pool ID
pub type PoolId = DigestOf<Blake2b256, PoolRegistration>;

/// signatures with indices
pub type IndexSignatures = Vec<(u8, AccountBindingSignature)>;

/// Pool information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRegistration {
    /// A random value, for user purpose similar to a UUID.
    /// it may not be unique over a blockchain, so shouldn't be used a unique identifier
    pub serial: u128,
    /// Beginning of validity for this pool, this is used
    /// to keep track of the period of the expected key and the expiry
    pub start_validity: TimeOffsetSeconds,
    /// Permission system for this pool
    /// * Management threshold for owners, this need to be <= #owners and > 0.
    pub permissions: PoolPermissions,
    /// Owners of this pool
    pub owners: Vec<PublicKey<Ed25519>>,
    /// Operators of this pool
    pub operators: Box<[PublicKey<Ed25519>]>,
    /// Rewarding
    pub rewards: TaxType,
    /// Genesis Praos keys
    pub keys: GenesisPraosLeader,
}

/// Permission system related to the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolPermissions(u64);

pub type ManagementThreshold = u8;

const MANAGEMENT_THRESHOLD_BITMASK: u64 = 0b111111; // only support 32, reserved one for later extension if needed
const ALL_USED_BITMASK: u64 =
    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00111111;

impl PoolPermissions {
    pub fn new(management_threshold: u8) -> PoolPermissions {
        let v = management_threshold as u64 & MANAGEMENT_THRESHOLD_BITMASK;
        PoolPermissions(v)
    }

    pub fn from_u64(v: u64) -> Option<PoolPermissions> {
        if (v & !ALL_USED_BITMASK) > 0 {
            None
        } else {
            Some(PoolPermissions(v))
        }
    }

    pub fn management_threshold(self) -> ManagementThreshold {
        (self.0 & MANAGEMENT_THRESHOLD_BITMASK) as ManagementThreshold
    }
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

#[derive(Debug, Clone)]
pub enum PoolSignature {
    Operator(AccountBindingSignature),
    Owners(PoolOwnersSignature),
}

/// Representant of a structure signed by a pool's owners
#[derive(Debug, Clone)]
pub struct PoolOwnersSignature {
    pub signatures: IndexSignatures,
}

pub type PoolOwnersSigned = PoolOwnersSignature;

impl PoolRegistration {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        let oo = oo_mux(self.owners.len(), self.operators.len());
        bb.u128(self.serial)
            .u64(self.start_validity.into())
            .u64(self.permissions.0)
            .u8(0).u8(0).u8(0).u8(oo)
            .fold(&mut self.owners.iter(), |bb, o| bb.bytes(o.as_ref()))
            .fold(&mut self.operators.iter(), |bb, o| bb.bytes(o.as_ref()))
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

    pub fn management_threshold(&self) -> u8 {
        self.permissions.management_threshold()
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
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = PoolSignature;
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }
    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(
            auth.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }
    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        Some(CertificateSlice::from(p))
    }
}

impl Payload for PoolRetirement {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = PoolSignature;
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }
    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(
            auth.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }
    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        Some(CertificateSlice::from(p))
    }
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
        let permissions = PoolPermissions::from_u64(buf.get_u64()?).ok_or(
            ReadError::StructureInvalid("permission value not correct".to_string()),
        )?;

        let p1 = buf.get_u8()?;
        let p2 = buf.get_u8()?;
        let p3 = buf.get_u8()?;

        if p1 != 0 || p2 != 0 || p3 != 0 {
            Err(ReadError::StructureInvalid("pool registration padding is invalid".to_string()))?
        }

        let oo_nb = buf.get_u8()?;

        let (owners_nb, operators_nb) = oo_demux(oo_nb)
            .ok_or(ReadError::StructureInvalid("size owners-operators invalid".to_string()))?;

        let mut owners = Vec::with_capacity(owners_nb as usize);
        for _ in 0..owners_nb {
            owners.push(deserialize_public_key(buf)?);
        }

        let mut operators = Vec::with_capacity(operators_nb as usize);
        for _ in 0..operators_nb {
            operators.push(deserialize_public_key(buf)?);
        }

        let rewards = TaxType::read_frombuf(buf)?;
        let keys = GenesisPraosLeader::read(buf)?;

        let info = Self {
            serial,
            start_validity,
            permissions,
            owners,
            operators: operators.into(),
            rewards,
            keys,
        };
        Ok(info)
    }
}

impl Payload for PoolRegistration {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = PoolSignature;
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(
            auth.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }

    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        Some(CertificateSlice::from(p))
    }
}

impl PoolSignature {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        match self {
            PoolSignature::Operator(op) => {
                bb.u8(0).bytes(op.0.as_ref())
            }
            PoolSignature::Owners(owners) => {
                assert!(owners.signatures.len() > 0);
                assert!(owners.signatures.len() < 256);
                bb.iter8(&mut owners.signatures.iter(), |bb, (i, s)| {
                    bb.u8(*i).bytes(s.as_ref())
                })

            }
        }
    }

    pub fn verify<'a>(
        &self,
        pool_info: &PoolRegistration,
        verify_data: &TransactionBindingAuthData<'a>,
    ) -> Verification {
        match self {
            PoolSignature::Operator(_) => Verification::Failed,
            PoolSignature::Owners(owners) => {
                owners.verify(pool_info, verify_data)
            }
        }
    }
}

impl PoolOwnersSignature {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.iter8(&mut self.signatures.iter(), |bb, (i, s)| {
            bb.u8(*i).bytes(s.as_ref())
        })
    }

    pub fn verify<'a>(
        &self,
        pool_info: &PoolRegistration,
        verify_data: &TransactionBindingAuthData<'a>,
    ) -> Verification {
        // fast track if we don't meet the management threshold already
        if self.signatures.len() < pool_info.management_threshold() as usize {
            return Verification::Failed;
        }

        let mut present = vec![false; pool_info.owners.len()];
        let mut signatories = 0;

        for (i, sig) in self.signatures.iter() {
            let i = *i as usize;
            // Check for out of bounds indices
            if i >= pool_info.owners.len() {
                return Verification::Failed;
            }

            // If already present, then we have a duplicate hence fail
            if present[i] {
                return Verification::Failed;
            } else {
                present[i] = true;
            }

            // Verify the cryptographic signature of a signatory
            let pk = &pool_info.owners[i];
            if sig.verify_slice(pk, verify_data) == Verification::Failed {
                return Verification::Failed;
            }
            signatories += 1
        }

        // check if we seen enough unique signatures; it is a redundant check
        // from the duplicated check + the threshold check
        if signatories < pool_info.management_threshold() as usize {
            return Verification::Failed;
        }

        Verification::Success
    }
}

impl Readable for PoolOwnersSigned {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let sigs_nb = buf.get_u8()? as usize;
        if sigs_nb == 0 {
            Err(ReadError::StructureInvalid("pool owner signature with 0 signatures".to_string()))?
        }
        let mut signatures = Vec::new();
        for _ in 0..sigs_nb {
            let nb = buf.get_u8()?;
            let sig = deserialize_signature(buf)?;
            signatures.push((nb, AccountBindingSignature(sig)))
        }
        Ok(PoolOwnersSigned { signatures })
    }
}

impl Readable for PoolSignature {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.peek_u8()? {
            0 => {
                let _ = buf.get_u8()?;
                let sig = deserialize_signature(buf)?;
                Ok(PoolSignature::Operator(AccountBindingSignature(sig)))
            }
            _ => PoolOwnersSigned::read(buf).map(PoolSignature::Owners)
        }
    }
}

// Operator-Owners-size (de-)multiplexing
// 5 bits for the owners for a maximum of 31 elements
// 2 bits for the operators for a maximum of 3 elements
// 1 bit of unused set to 0

const OO_OWNERS_BITMASK : u8 = 0b1_1111;
const OO_OPERATORS_BITMASK : u8 = 0b11;
const OO_OPERATORS_SHIFT : u32 = OO_OWNERS_BITMASK.count_ones();
const OO_UNUSED_BITMASK : u8 = 0b1000_0000;

fn oo_mux(owners: usize, operators: usize) -> u8 {
    assert!(owners < 32);
    assert!(operators < 4);
    (owners | (operators << OO_OPERATORS_SHIFT)) as u8
}

fn oo_demux(v: u8) -> Option<(usize, usize)> {
    if v & OO_UNUSED_BITMASK != 0 {
        None
    } else {
        Some((
            (v & OO_OWNERS_BITMASK) as usize,
            ((v >> OO_OPERATORS_SHIFT) & OO_OPERATORS_BITMASK) as usize
        ))
    }
}
