use chain_addr::{Address, Discrimination};
use chain_crypto::*;
use chain_impl_mockchain::block::ConsensusVersion;
use chain_impl_mockchain::block::HeaderHash;
use chain_impl_mockchain::config::ConfigParam;
use chain_impl_mockchain::ledger::Ledger;
use chain_impl_mockchain::message::config::ConfigParams;
use chain_impl_mockchain::message::Message;
use chain_impl_mockchain::milli::Milli;
use chain_impl_mockchain::transaction::*;
use std::vec::Vec;

use crate::common::tx_builder::TransactionBuilder;

pub struct ConfigBuilder {
    slot_duration: u8,
    slots_per_epoch: u32,
    active_slots_coeff: Milli,
    discrimination: Discrimination,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            slot_duration: 20,
            slots_per_epoch: 21600,
            active_slots_coeff: Milli::HALF,
            discrimination: Discrimination::Test,
        }
    }

    pub fn with_slot_duration<'a>(&'a mut self, slot_duration: u8) -> &'a mut Self {
        self.slot_duration = slot_duration;
        self
    }

    pub fn with_slots_per_epoch<'a>(&'a mut self, slots_per_epoch: u32) -> &'a mut Self {
        self.slots_per_epoch = slots_per_epoch;
        self
    }

    pub fn with_active_slots_coeff<'a>(&'a mut self, active_slots_coeff: Milli) -> &'a mut Self {
        self.active_slots_coeff = active_slots_coeff;
        self
    }

    pub fn build(&self) -> ConfigParams {
        let mut ie = ConfigParams::new();
        ie.push(ConfigParam::Discrimination(self.discrimination));
        ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));

        // TODO remove rng: make this creation deterministic
        let leader_prv_key: SecretKey<Ed25519Extended> = SecretKey::generate(rand::thread_rng());
        let leader_pub_key = leader_prv_key.to_public();
        ie.push(ConfigParam::AddBftLeader(leader_pub_key.into()));
        ie.push(ConfigParam::Block0Date(
            chain_impl_mockchain::config::Block0Date(0),
        ));
        ie.push(ConfigParam::SlotDuration(self.slot_duration));
        ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            self.active_slots_coeff,
        ));
        ie.push(ConfigParam::SlotsPerEpoch(self.slots_per_epoch));
        ie.push(ConfigParam::KESUpdateSpeed(3600 * 12));
        ie
    }
}

// create an initial fake ledger with the non-optional parameter setup
pub fn create_initial_fake_ledger(
    initial_msgs: &[Message],
    config_Params: ConfigParams,
) -> (HeaderHash, Ledger) {
    let block0_hash = HeaderHash::hash_bytes(&[1, 2, 3]);

    let mut messages = Vec::new();
    messages.push(Message::Initial(config_Params));
    messages.extend_from_slice(initial_msgs);
    let ledger = Ledger::new(block0_hash, &messages).expect("create initial fake ledger failed");
    (block0_hash, ledger)
}

pub fn create_initial_transaction(output: Output<Address>) -> (Message, Vec<UtxoPointer>) {
    let mut builder = TransactionBuilder::new();
    builder.with_output(output).finalize();
    (builder.as_message(), builder.as_utxos())
}
