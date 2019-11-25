/// This contains the current evaluation methods for the VRF and its link to
/// the stake distribution
use crate::date::SlotId;
use crate::key::Hash;
use crate::milli::Milli;
use crate::value::Value;
use chain_crypto::{
    vrf_evaluate_and_prove, vrf_verified_get_output, vrf_verify, Curve25519_2HashDH, PublicKey,
    SecretKey, VRFVerification, VerifiableRandomFunction,
};
use rand_os::OsRng;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Nonce gathered per block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce([u8; 32]);

impl Nonce {
    pub fn zero() -> Self {
        Nonce([0u8; 32])
    }

    /// Change the nonce to be the result of the hash of the current nonce
    /// and the new supplied nonce.
    ///
    /// Effectively: Self = H(Self, Supplied-Hash)
    pub fn hash_with(&mut self, other: &Self) {
        let mut buf = [0; 64];
        buf[0..32].copy_from_slice(&self.0);
        buf[32..64].copy_from_slice(&other.0);
        self.0.copy_from_slice(Hash::hash_bytes(&buf).as_ref())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActiveSlotsCoeffError {
    InvalidValue(Milli),
}

impl Display for ActiveSlotsCoeffError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ActiveSlotsCoeffError::InvalidValue(v) => {
                write!(f, "Invalid value {}, should be in range (0,1]", v)
            }
        }
    }
}

impl Error for ActiveSlotsCoeffError {}

/// Active slots coefficient used for calculating minimum stake to become slot leader candidate
/// Described in Ouroboros Praos paper, also referred to as parameter F of phi function
/// Always in range (0, 1]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveSlotsCoeff(Milli);

impl TryFrom<Milli> for ActiveSlotsCoeff {
    type Error = ActiveSlotsCoeffError;

    fn try_from(value: Milli) -> Result<Self, Self::Error> {
        if value > Milli::ZERO && value <= Milli::ONE {
            Ok(ActiveSlotsCoeff(value))
        } else {
            Err(ActiveSlotsCoeffError::InvalidValue(value))
        }
    }
}

impl From<ActiveSlotsCoeff> for Milli {
    fn from(coeff: ActiveSlotsCoeff) -> Milli {
        coeff.0
    }
}

impl From<ActiveSlotsCoeff> for f64 {
    fn from(coeff: ActiveSlotsCoeff) -> f64 {
        coeff.0.to_float()
    }
}

/// Threshold between 0.0 and 1.0
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Threshold(f64);

impl Threshold {
    pub fn from_u256(v: &[u8]) -> Self {
        assert_eq!(v.len(), 32);
        // TODO, only consider the highest part
        let v64 = (v[0] as u64) << 56
            | (v[1] as u64) << 48
            | (v[2] as u64) << 40
            | (v[3] as u64) << 32
            | (v[4] as u64) << 24
            | (v[5] as u64) << 16
            | (v[6] as u64) << 8
            | (v[7] as u64);
        Threshold((v64 as f64) / 18446744073709551616.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PercentStake {
    pub stake: Value,
    pub total: Value,
}

/// previous epoch nonce and the slotid encoded in big endian
struct Input([u8; 36]);

impl Input {
    /// Create an Input from previous epoch nonce and the current slotid
    fn create(epoch_nonce: &Nonce, slotid: SlotId) -> Self {
        let mut input = [0u8; 36];
        input[0..32].copy_from_slice(&epoch_nonce.0[..]);
        input[32..].copy_from_slice(&slotid.to_le_bytes());
        Input(input)
    }
}

/// Witness
pub type Witness = <Curve25519_2HashDH as VerifiableRandomFunction>::VerifiedRandomOutput;
pub type WitnessOutput = <Curve25519_2HashDH as VerifiableRandomFunction>::RandomOutput;

pub struct VrfEvaluator<'a> {
    pub stake: PercentStake,
    pub nonce: &'a Nonce,
    pub slot_id: SlotId,
    pub active_slots_coeff: ActiveSlotsCoeff,
}

pub(crate) fn witness_to_nonce(witness: &Witness) -> Nonce {
    let r = vrf_verified_get_output::<Curve25519_2HashDH>(&witness);
    get_nonce(&r)
}

impl<'a> VrfEvaluator<'a> {
    /// Evaluate if the threshold is above for a given input for the key and the associated stake
    ///
    /// On threshold success, the witness is returned, otherwise None is returned
    pub fn evaluate(&self, key: &SecretKey<Curve25519_2HashDH>) -> Option<Witness> {
        let input = Input::create(self.nonce, self.slot_id);
        let csprng = OsRng::new().unwrap();
        let vr = vrf_evaluate_and_prove(key, &input.0, csprng);
        let r = vrf_verified_get_output::<Curve25519_2HashDH>(&vr);
        let t = get_threshold(&input, &r);
        if above_stake_threshold(t, &self.stake, self.active_slots_coeff) {
            Some(vr)
        } else {
            None
        }
    }

    /// verify that the witness pass the threshold for this witness for a given
    /// key and its associated stake.
    ///
    /// On success, the nonce is returned, otherwise None is returned
    pub fn verify(
        &self,
        key: &PublicKey<Curve25519_2HashDH>,
        witness: &'a Witness,
    ) -> Option<Nonce> {
        let input = Input::create(&self.nonce, self.slot_id);
        if vrf_verify(key, &input.0, witness) == VRFVerification::Success {
            let r = vrf_verified_get_output::<Curve25519_2HashDH>(witness);
            let t = get_threshold(&input, &r);
            if above_stake_threshold(t, &self.stake, self.active_slots_coeff) {
                Some(get_nonce(&r))
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn above_stake_threshold(
    threshold: Threshold,
    stake: &PercentStake,
    active_slots_coeff: ActiveSlotsCoeff,
) -> bool {
    threshold < phi(active_slots_coeff, stake)
}

fn phi(active_slots_coeff: ActiveSlotsCoeff, rs: &PercentStake) -> Threshold {
    assert!(rs.stake <= rs.total);
    let t = (rs.stake.0 as f64) / (rs.total.0 as f64);
    let f: f64 = active_slots_coeff.into();
    Threshold(1.0 - (1.0 - f).powf(t))
}

const DOMAIN_NONCE: &'static [u8] = b"NONCE";
const DOMAIN_THRESHOLD: &'static [u8] = b"TEST";

fn get_threshold(input: &Input, os: &WitnessOutput) -> Threshold {
    let out = os.to_output(&input.0, DOMAIN_THRESHOLD);
    // read as big endian 64 bits values from left to right.
    Threshold::from_u256(out.as_ref())
}

fn get_nonce(os: &WitnessOutput) -> Nonce {
    let mut nonce = [0u8; 32];
    let out = os.to_output(&[], DOMAIN_NONCE);
    nonce.copy_from_slice(out.as_ref());
    Nonce(nonce)
}

#[cfg(test)]
mod tests {
    use super::Nonce;
    use quickcheck::{Arbitrary, Gen};
    use std::iter;

    impl Arbitrary for Nonce {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut nonce = [0; 32];
            let nonce_vec: Vec<u8> = iter::from_fn(|| Some(u8::arbitrary(g))).take(32).collect();
            nonce.copy_from_slice(&nonce_vec);
            Nonce(nonce)
        }
    }
}
