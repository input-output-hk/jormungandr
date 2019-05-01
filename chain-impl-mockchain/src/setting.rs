//! define the Blockchain settings
//!

use crate::update;
use crate::{block::ConsensusVersion, fee::LinearFee, leadership::bft};
use crate::leadership::{genesis::ActiveSlotsCoeff};
use crate::milli::Milli;
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    pub max_number_of_transactions_per_block: u32,
    pub bootstrap_key_slots_percentage: u8, // == d * 100
    pub consensus_version: ConsensusVersion,
    pub bft_leaders: Arc<Vec<bft::LeaderId>>,
    /// allow for the creation of accounts without the certificate
    pub allow_account_creation: bool,
    pub linear_fees: Arc<LinearFee>,
    pub slot_duration: u8,
    pub epoch_stability_depth: u32,
    /// The number of epochs that a proposal remains valid. To be
    /// precise, if a proposal is made at date (epoch_p, slot), then
    /// it expires at the start of epoch 'epoch_p +
    /// proposal_expiration + 1'. FIXME: make updateable.
    pub proposal_expiration: u32,
    pub active_slots_coeff: ActiveSlotsCoeff,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new() -> Self {
        Self {
            max_number_of_transactions_per_block: 100,
            bootstrap_key_slots_percentage: SLOTS_PERCENTAGE_RANGE,
            consensus_version: ConsensusVersion::Bft,
            bft_leaders: Arc::new(Vec::new()),
            allow_account_creation: false,
            linear_fees: Arc::new(LinearFee::new(0, 0, 0)),
            slot_duration: 10,         // 10 sec
            epoch_stability_depth: 10, // num of block
            proposal_expiration: 100,
            active_slots_coeff: ActiveSlotsCoeff::try_from(Milli::HALF).unwrap(),
        }
    }

    pub fn allow_account_creation(&self) -> bool {
        self.allow_account_creation
    }

    pub fn linear_fees(&self) -> LinearFee {
        *self.linear_fees
    }

    pub fn apply(&self, update: &update::UpdateProposal) -> Self {
        let mut new_state = self.clone();
        if let Some(max_number_of_transactions_per_block) =
            update.max_number_of_transactions_per_block
        {
            new_state.max_number_of_transactions_per_block = max_number_of_transactions_per_block;
        }
        if let Some(bootstrap_key_slots_percentage) = update.bootstrap_key_slots_percentage {
            new_state.bootstrap_key_slots_percentage = bootstrap_key_slots_percentage;
        }
        if let Some(consensus_version) = update.consensus_version {
            new_state.consensus_version = consensus_version;
        }
        if let Some(ref leaders) = update.bft_leaders {
            new_state.bft_leaders = Arc::new(leaders.clone());
        }
        if let Some(allow_account_creation) = update.allow_account_creation {
            new_state.allow_account_creation = allow_account_creation;
        }
        if let Some(linear_fees) = update.linear_fees {
            new_state.linear_fees = Arc::new(linear_fees);
        }
        if let Some(slot_duration) = update.slot_duration {
            new_state.slot_duration = slot_duration;
        }
        if let Some(epoch_stability_depth) = update.epoch_stability_depth {
            new_state.epoch_stability_depth = epoch_stability_depth;
        }
        new_state
    }
}
