//! define the Blockchain settings
//!

use crate::fragment::config::ConfigParams;
use crate::leadership::genesis::ActiveSlotsCoeff;
use crate::milli::Milli;
use crate::update::Error;
use crate::{
    block::ConsensusVersion,
    config::{ConfigParam, RewardParams},
    fee::LinearFee,
    leadership::{bft, genesis},
    rewards,
};
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    pub consensus_version: ConsensusVersion,
    pub consensus_nonce: genesis::Nonce,
    pub slots_per_epoch: u32,
    pub slot_duration: u8,
    pub epoch_stability_depth: u32,
    pub active_slots_coeff: ActiveSlotsCoeff,
    pub max_number_of_transactions_per_block: u32,
    pub bft_slots_ratio: Milli, // aka "d" parameter
    pub bft_leaders: Arc<Vec<bft::LeaderId>>,
    pub linear_fees: Arc<LinearFee>,
    /// The number of epochs that a proposal remains valid. To be
    /// precise, if a proposal is made at date (epoch_p, slot), then
    /// it expires at the start of epoch 'epoch_p +
    /// proposal_expiration + 1'. FIXME: make updateable.
    pub proposal_expiration: u32,
    pub reward_params: Option<RewardParams>,
    pub treasury_params: Option<rewards::TaxType>,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new() -> Self {
        Self {
            consensus_version: ConsensusVersion::Bft,
            consensus_nonce: genesis::Nonce::zero(),
            slots_per_epoch: 1,
            slot_duration: 10,         // 10 sec
            epoch_stability_depth: 10, // num of block
            active_slots_coeff: ActiveSlotsCoeff::try_from(Milli::HALF).unwrap(),
            max_number_of_transactions_per_block: 100,
            bft_slots_ratio: Milli::ONE,
            bft_leaders: Arc::new(Vec::new()),
            linear_fees: Arc::new(LinearFee::new(0, 0, 0)),
            proposal_expiration: 100,
            reward_params: None,
            treasury_params: None,
        }
    }

    pub fn linear_fees(&self) -> LinearFee {
        *self.linear_fees
    }

    pub fn apply(&self, changes: &ConfigParams) -> Result<Self, Error> {
        let mut new_state = self.clone();

        for param in changes.iter() {
            match param {
                ConfigParam::Block0Date(_)
                | ConfigParam::Discrimination(_)
                | ConfigParam::TreasuryAdd(_)
                | ConfigParam::RewardPot(_)
                | ConfigParam::KESUpdateSpeed(_) => {
                    return Err(Error::ReadOnlySetting);
                }
                ConfigParam::ConsensusVersion(d) => {
                    new_state.consensus_version = *d;
                }
                ConfigParam::SlotsPerEpoch(d) => {
                    new_state.slots_per_epoch = *d;
                }
                ConfigParam::SlotDuration(d) => {
                    new_state.slot_duration = *d;
                }
                ConfigParam::EpochStabilityDepth(d) => {
                    new_state.epoch_stability_depth = *d;
                }
                ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(d) => {
                    new_state.active_slots_coeff = ActiveSlotsCoeff::try_from(*d)?;
                }
                ConfigParam::MaxNumberOfTransactionsPerBlock(d) => {
                    new_state.max_number_of_transactions_per_block = *d;
                }
                ConfigParam::BftSlotsRatio(d) => {
                    if *d > Milli::ONE {
                        return Err(Error::BadBftSlotsRatio(*d));
                    }
                    new_state.bft_slots_ratio = *d;
                }
                ConfigParam::AddBftLeader(d) => {
                    // FIXME: O(n)
                    let mut v = new_state.bft_leaders.to_vec();
                    v.push(d.clone());
                    new_state.bft_leaders = Arc::new(v);
                }
                ConfigParam::RemoveBftLeader(d) => {
                    new_state.bft_leaders = Arc::new(
                        new_state
                            .bft_leaders
                            .iter()
                            .filter(|leader| *leader != d)
                            .cloned()
                            .collect(),
                    );
                }
                ConfigParam::LinearFee(d) => {
                    new_state.linear_fees = Arc::new(*d);
                }
                ConfigParam::ProposalExpiration(d) => {
                    new_state.proposal_expiration = *d;
                }
                ConfigParam::RewardParams(rp) => {
                    new_state.reward_params = Some(rp.clone());
                }
                ConfigParam::TreasuryParams(rp) => {
                    new_state.treasury_params = Some(rp.clone());
                }
            }
        }

        Ok(new_state)
    }

    pub fn to_config_params(&self) -> ConfigParams {
        let mut params = ConfigParams::new();

        params.push(ConfigParam::ConsensusVersion(self.consensus_version));
        params.push(ConfigParam::SlotsPerEpoch(self.slots_per_epoch));
        params.push(ConfigParam::SlotDuration(self.slot_duration));
        params.push(ConfigParam::EpochStabilityDepth(self.epoch_stability_depth));
        params.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            self.active_slots_coeff.into(),
        ));
        params.push(ConfigParam::MaxNumberOfTransactionsPerBlock(
            self.max_number_of_transactions_per_block,
        ));
        params.push(ConfigParam::BftSlotsRatio(self.bft_slots_ratio));
        for bft_leader in self.bft_leaders.iter() {
            params.push(ConfigParam::AddBftLeader(bft_leader.clone()));
        }
        params.push(ConfigParam::LinearFee(*self.linear_fees));
        params.push(ConfigParam::ProposalExpiration(self.proposal_expiration));

        match &self.reward_params {
            Some(p) => params.push(ConfigParam::RewardParams(p.clone())),
            None => (),
        };
        match &self.treasury_params {
            Some(p) => params.push(ConfigParam::TreasuryParams(p.clone())),
            None => (),
        };

        debug_assert_eq!(self, &Settings::new().apply(&params).unwrap());

        params
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Settings {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Settings::new()
        }
    }
}
