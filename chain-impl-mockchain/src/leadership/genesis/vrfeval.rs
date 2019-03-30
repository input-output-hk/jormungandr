/// This contains the current evaluation methods for the VRF and its link to
/// the stake distribution
use crate::date::SlotId;
use crate::value::Value;
use chain_crypto::{
    vrf_evaluate_and_proove, vrf_verified_get_output, vrf_verify, Curve25519_2HashDH, PublicKey,
    SecretKey, VRFVerification, VerifiableRandomFunction,
};
use rand::rngs::OsRng;

/// Nonce gathered per block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce([u8; 32]);

impl Nonce {
    pub fn zero() -> Self {
        Nonce([0u8; 32])
    }
}

/// number between 0.0 and 1.0 that is used to calculate to
pub struct F(f64);

impl F {
    // TODO: error handling and replace by TryFrom once more stable
    pub fn create(v: f64) -> F {
        assert!(v > 0.0 && v <= 1.0);
        F(v)
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

pub fn phi(f: F, rs: PercentStake) -> Threshold {
    assert!(rs.stake <= rs.total);
    let t = (rs.stake.0 as f64) / (rs.total.0 as f64);
    Threshold(1.0 - (1.0 - f.0).powf(t))
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

/// Evaluate if the threshold is above for a given input for the key and the associated stake
///
/// On threshold success, the witness is returned, otherwise None is returned
pub fn evaluate(
    my_stake: PercentStake,
    key: &SecretKey<Curve25519_2HashDH>,
    nonce: &Nonce,
    slotid: SlotId,
) -> Option<Witness> {
    let input = Input::create(nonce, slotid);
    let csprng = OsRng::new().unwrap();
    let vr = vrf_evaluate_and_proove(key, &input.0, csprng);
    let r = vrf_verified_get_output::<Curve25519_2HashDH>(&vr);
    let t = get_threshold(&input, &r);
    if above_stake_threshold(t, my_stake) {
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
    key_stake: PercentStake,
    key: &PublicKey<Curve25519_2HashDH>,
    nonce: &Nonce,
    slotid: SlotId,
    witness: &Witness,
) -> Option<Nonce> {
    let input = Input::create(nonce, slotid);
    if vrf_verify(key, &input.0, witness) == VRFVerification::Success {
        let r = vrf_verified_get_output::<Curve25519_2HashDH>(witness);
        let t = get_threshold(&input, &r);
        if above_stake_threshold(t, key_stake) {
            Some(get_nonce(&input, &r))
        } else {
            None
        }
    } else {
        None
    }
}

fn above_stake_threshold(threshold: Threshold, stake: PercentStake) -> bool {
    // TODO F is hardcoded here
    threshold >= phi(F::create(0.5), stake)
}

const DOMAIN_NONCE: &'static [u8] = b"NONCE";
const DOMAIN_THRESHOLD: &'static [u8] = b"TEST";

fn get_threshold(input: &Input, os: &WitnessOutput) -> Threshold {
    let out = os.to_output(&input.0, DOMAIN_THRESHOLD);
    // read as big endian 64 bits values from left to right.
    Threshold::from_u256(out.as_ref())
}

fn get_nonce(input: &Input, os: &WitnessOutput) -> Nonce {
    let mut nonce = [0u8; 32];
    let out = os.to_output(&input.0, DOMAIN_NONCE);
    nonce.copy_from_slice(out.as_ref());
    Nonce(nonce)
}
