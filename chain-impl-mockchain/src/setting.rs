//! define the Blockchain settings
//!

use crate::leadership::genesis::ActiveSlotsCoeff;
use crate::milli::Milli;
use crate::update::Error;
use crate::{block::ConsensusVersion, config::ConfigParam, fee::LinearFee, leadership::bft};
use chain_time::era::TimeEra;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    pub era: TimeEra,
    pub consensus_version: ConsensusVersion,
    pub slots_per_epoch: u32,
    pub slot_duration: u8,
    pub epoch_stability_depth: u32,
    pub active_slots_coeff: ActiveSlotsCoeff,
    pub max_number_of_transactions_per_block: u32,
    pub bft_slots_ratio: Milli, // aka "d" parameter
    pub bft_leaders: Arc<Vec<bft::LeaderId>>,
    /// allow for the creation of accounts without the certificate
    pub allow_account_creation: bool,
    pub linear_fees: Arc<LinearFee>,
    /// The number of epochs that a proposal remains valid. To be
    /// precise, if a proposal is made at date (epoch_p, slot), then
    /// it expires at the start of epoch 'epoch_p +
    /// proposal_expiration + 1'. FIXME: make updateable.
    pub proposal_expiration: u32,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new(era: TimeEra) -> Self {
        Self {
            era: era,
            consensus_version: ConsensusVersion::Bft,
            slots_per_epoch: 1,
            slot_duration: 10,         // 10 sec
            epoch_stability_depth: 10, // num of block
            active_slots_coeff: ActiveSlotsCoeff::try_from(Milli::HALF).unwrap(),
            max_number_of_transactions_per_block: 100,
            bft_slots_ratio: Milli::ONE,
            bft_leaders: Arc::new(Vec::new()),
            allow_account_creation: false,
            linear_fees: Arc::new(LinearFee::new(0, 0, 0)),
            proposal_expiration: 100,
        }
    }

    pub fn allow_account_creation(&self) -> bool {
        self.allow_account_creation
    }

    pub fn linear_fees(&self) -> LinearFee {
        *self.linear_fees
    }

    pub fn apply(&self, changes: &crate::message::config::ConfigParams) -> Result<Self, Error> {
        let mut new_state = self.clone();

        for param in changes.iter() {
            match param {
                ConfigParam::Block0Date(_) | ConfigParam::Discrimination(_) => {
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
                    new_state.active_slots_coeff = ActiveSlotsCoeff(*d);
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
                ConfigParam::AllowAccountCreation(d) => {
                    new_state.allow_account_creation = *d;
                }
                ConfigParam::LinearFee(d) => {
                    new_state.linear_fees = Arc::new(*d);
                }
                ConfigParam::ProposalExpiration(d) => {
                    new_state.proposal_expiration = *d;
                }
            }
        }

        Ok(new_state)
    }
}
