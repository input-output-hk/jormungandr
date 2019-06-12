mod vrfeval;

use crate::{
    block::{BlockDate, Header, Proof},
    date::Epoch,
    key::verify_signature,
    leadership::{Error, ErrorKind, Verification},
    ledger::Ledger,
    stake::{self, StakeDistribution, StakePoolId},
    value::Value,
};
use chain_crypto::Verification as SigningVerification;
use chain_crypto::{Curve25519_2HashDH, PublicKey, SecretKey, SumEd25519_12};
pub(crate) use vrfeval::witness_to_nonce;
pub use vrfeval::{ActiveSlotsCoeff, ActiveSlotsCoeffError, Nonce, Witness, WitnessOutput};
use vrfeval::{PercentStake, VrfEvaluator};

/// Praos Leader consisting of the KES public key and VRF public key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenesisPraosLeader {
    pub kes_public_key: PublicKey<SumEd25519_12>,
    pub vrf_public_key: PublicKey<Curve25519_2HashDH>,
}

pub struct GenesisLeaderSelection {
    epoch_nonce: Nonce,
    nodes: stake::PoolTable,
    distribution: StakeDistribution,
    // the epoch this leader selection is valid for
    epoch: Epoch,
    active_slots_coeff: ActiveSlotsCoeff,
}

custom_error! {GenesisError
    InvalidEpoch { expected: Epoch, actual: Epoch } = "Wrong epoch, expected epoch {expected} but received block at epoch {actual}",
    TotalStakeIsZero = "Total stake is null",
}

impl GenesisLeaderSelection {
    pub fn new(epoch: Epoch, ledger: &Ledger) -> Self {
        GenesisLeaderSelection {
            epoch_nonce: ledger.settings.consensus_nonce.clone(),
            nodes: ledger.delegation.stake_pools.clone(),
            distribution: ledger.get_stake_distribution(),
            epoch,
            active_slots_coeff: ledger.settings.active_slots_coeff,
        }
    }

    pub fn distribution(&mut self) -> &mut StakeDistribution {
        &mut self.distribution
    }

    pub fn leader(
        &self,
        pool_id: &StakePoolId,
        vrf_key: &SecretKey<Curve25519_2HashDH>,
        date: BlockDate,
    ) -> Result<Option<Witness>, Error> {
        if date.epoch != self.epoch {
            return Err(Error::new_(
                ErrorKind::Failure,
                GenesisError::InvalidEpoch {
                    actual: date.epoch,
                    expected: self.epoch,
                },
            ));
        }

        let stake_snapshot = &self.distribution;

        match stake_snapshot.get_stake_for(&pool_id) {
            None => Ok(None),
            Some(stake) => {
                // Calculate the total stake.
                let total_stake: Value = stake_snapshot.total_stake();

                if total_stake == Value::zero() {
                    return Err(Error::new_(
                        ErrorKind::Failure,
                        GenesisError::TotalStakeIsZero,
                    ));
                }

                let percent_stake = PercentStake {
                    stake: stake,
                    total: total_stake,
                };
                let evaluator = VrfEvaluator {
                    stake: percent_stake,
                    nonce: &self.epoch_nonce,
                    slot_id: date.slot_id,
                    active_slots_coeff: self.active_slots_coeff,
                };
                Ok(evaluator.evaluate(vrf_key))
            }
        }
    }

    pub(crate) fn verify(&self, block_header: &Header) -> Verification {
        if block_header.block_date().epoch != self.epoch {
            return Verification::Failure(Error::new_(
                ErrorKind::Failure,
                GenesisError::InvalidEpoch {
                    expected: self.epoch,
                    actual: block_header.block_date().epoch,
                },
            ));
        }

        let stake_snapshot = &self.distribution;

        match &block_header.proof() {
            Proof::GenesisPraos(ref genesis_praos_proof) => {
                let node_id = &genesis_praos_proof.node_id;
                match (
                    stake_snapshot.get_stake_for(node_id),
                    self.nodes.lookup(node_id),
                ) {
                    (Some(stake), Some(pool_info)) => {
                        // Calculate the total stake.
                        let total_stake: Value = stake_snapshot.total_stake();

                        let percent_stake = PercentStake {
                            stake: stake,
                            total: total_stake,
                        };

                        let _ = VrfEvaluator {
                            stake: percent_stake,
                            nonce: &self.epoch_nonce,
                            slot_id: block_header.block_date().slot_id,
                            active_slots_coeff: self.active_slots_coeff,
                        }
                        .verify(
                            &pool_info.initial_key.vrf_public_key,
                            &genesis_praos_proof.vrf_proof,
                        );

                        let valid = verify_signature(
                            &genesis_praos_proof.kes_proof.0,
                            &pool_info.initial_key.kes_public_key,
                            &block_header.common,
                        );

                        if valid == SigningVerification::Failed {
                            Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature))
                        } else {
                            Verification::Success
                        }
                    }
                    (_, _) => Verification::Failure(Error::new(ErrorKind::InvalidBlockMessage)),
                }
            }
            _ => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
        }
    }
}
