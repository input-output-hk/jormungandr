// Verifiable Random Function for Genesis
//
// The VRF is used for 2 reasons:
//
// * generate verifiable and non-predictable/malleable nonces
// * random generate for threshold evaluate
//
// Evaluate VRF with the following input:
//
//     nonce_epoch || slotid
//
// A key difference with the ouroboros praos/genesis paper,
// is that we do not evaluate the VRF for two different inputs,
// but instead leverage the initial verified output (an elliptic point)
// of a single evaluation of VRF, to create the two different values:
//
// * H(input | VRF-output | NONCE) for the block nonce for next epoch
// * H(input | VRF-output | TESTE) for the threshold evaluation
//

use super::super::super::secure::crypto::vrf;
use super::params::{phi, Threshold, F};
use super::stake::PercentStake;
use rand::OsRng;
use sha2::Sha256;

pub type SecretKey = vrf::SecretKey;
pub type PublicKey = vrf::PublicKey;

pub type Witness = vrf::ProvenOutputSeed;

/// Nonce generated per block
pub struct Nonce([u8; 32]);

/// previous epoch nonce and the slotid encoded in big endian
pub struct Input([u8; 36]);

impl Input {
    /// Create an Input from previous epoch nonce and the current slotid
    pub fn create(epoch_nonce: &Nonce, slotid: u32) -> Self {
        let mut input = [0u8; 36];
        input[0..32].copy_from_slice(&epoch_nonce.0[..]);
        input[32] = (slotid >> 24) as u8;
        input[33] = (slotid >> 16) as u8;
        input[34] = (slotid >> 8) as u8;
        input[35] = slotid as u8;
        Input(input)
    }
}

/// Evaluate if the threshold is above for a given input for the key and the associated stake
///
/// On threshold success, the witness is returned, otherwise None is returned
pub fn evaluate(my_stake: PercentStake, key: &SecretKey, input: &Input) -> Option<Witness> {
    let (input_as_point, outputseed) = key.verifiable_output(&input.0);
    let t = get_threshold(input, &outputseed);
    if above_stake_threshold(t, my_stake) {
        let mut csprng = OsRng::new().unwrap();
        let po = key.proove_simple(&mut csprng, input_as_point, outputseed);
        Some(po)
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
    key: &PublicKey,
    input: &Input,
    witness: &Witness,
) -> Option<Nonce> {
    match witness.to_verifiable_output(key, &input.0) {
        None => None,
        Some(vof) => {
            let t = get_threshold(input, &vof);
            if above_stake_threshold(t, key_stake) {
                Some(get_nonce(input, &vof))
            } else {
                None
            }
        }
    }
}

fn above_stake_threshold(threshold: Threshold, stake: PercentStake) -> bool {
    // TODO F is hardcoded here
    threshold >= phi(F::create(0.5), stake)
}

const DOMAIN_NONCE: &'static [u8] = b"NONCE";
const DOMAIN_THRESHOLD: &'static [u8] = b"TEST";

fn get_threshold(input: &Input, os: &vrf::OutputSeed) -> Threshold {
    let out = os.to_output::<Sha256>(&input.0, DOMAIN_THRESHOLD);
    // read as big endian 64 bits values from left to right.
    Threshold::from_u256(out.as_slice())
}

fn get_nonce(input: &Input, os: &vrf::OutputSeed) -> Nonce {
    let mut nonce = [0u8; 32];
    let out = os.to_output::<Sha256>(&input.0, DOMAIN_NONCE);
    nonce.copy_from_slice(out.as_slice());
    Nonce(nonce)
}
