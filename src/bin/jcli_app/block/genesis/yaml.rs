use chain_addr::{Address, Discrimination};
use chain_core::property::HasMessages as _;
use chain_crypto::{bech32::Bech32, Ed25519Extended, PublicKey};
use chain_impl_mockchain::{
    block::{Block, BlockBuilder, ConsensusVersion},
    fee::LinearFee,
    legacy::{self, OldAddress},
    message::{InitialEnts, Message},
    setting::UpdateProposal,
    transaction,
    value::Value,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct Genesis {
    /// the initial configuration of the blockchain
    ///
    /// * the start date of the block 0;
    /// * the discrimination;
    /// * ...
    ///
    /// All that is static and does not need to have any update
    /// mechanism.
    pub blockchain_configuration: BlockchainConfiguration,

    pub initial_setting: Update,

    pub initial_funds: Option<Vec<InitialUTxO>>,
    pub legacy_funds: Option<Vec<LegacyUTxO>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockchainConfiguration {
    #[serde(
        serialize_with = "jormungandr_utils::serde::time::serialize_system_time_in_sec",
        deserialize_with = "jormungandr_utils::serde::time::deserialize_system_time_in_sec"
    )]
    block0_date: SystemTime,
    #[serde(
        serialize_with = "jormungandr_utils::serde::address::serialize_discrimination",
        deserialize_with = "jormungandr_utils::serde::address::deserialize_discrimination"
    )]
    discrimination: Discrimination,
    #[serde(
        serialize_with = "jormungandr_utils::serde::block::serialize_consensus_version",
        deserialize_with = "jormungandr_utils::serde::block::deserialize_consensus_version"
    )]
    block0_consensus: ConsensusVersion,
}

/// the initial configuration of the blockchain
///
/// This is the data tha may be updated but which needs
/// to have an initial value in the blockchain (or not)
#[derive(Clone, Serialize, Deserialize)]
pub struct Update {
    max_number_of_transactions_per_block: Option<u32>,
    bootstrap_key_slots_percentage: Option<u8>,
    bft_leaders: Option<Vec<String>>,
    allow_account_creation: Option<bool>,
    linear_fee: Option<InitialLinearFee>,
    slot_duration: u8,
    epoch_stability_depth: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitialLinearFee {
    coefficient: u64,
    constant: u64,
    certificate: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InitialUTxO {
    #[serde(
        serialize_with = "jormungandr_utils::serde::address::serialize",
        deserialize_with = "jormungandr_utils::serde::address::deserialize"
    )]
    pub address: Address,
    #[serde(
        serialize_with = "jormungandr_utils::serde::value::serialize",
        deserialize_with = "jormungandr_utils::serde::value::deserialize"
    )]
    pub value: Value,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LegacyUTxO {
    pub address: OldAddress,
    #[serde(
        serialize_with = "jormungandr_utils::serde::value::serialize",
        deserialize_with = "jormungandr_utils::serde::value::deserialize"
    )]
    pub value: Value,
}

impl Genesis {
    pub fn from_block<'a>(block: &'a Block) -> Self {
        let mut messages = block.messages();

        let blockchain_configuration = if let Some(Message::Initial(initial)) = messages.next() {
            BlockchainConfiguration::from_ents(initial)
        } else {
            panic!("Expecting the second Message of the block 0 to be `Message::Initial`")
        };
        let initial_setting = if let Some(Message::Update(update)) = messages.next() {
            Update::from_message(update)
        } else {
            panic!("Expecting the second Message of the block 0 to be `Message::Update`")
        };

        let mut messages = messages.peekable();

        let initial_utxos = get_initial_utxos(&mut messages);
        let legacy_utxos = get_legacy_utxos(&mut messages);

        Genesis {
            blockchain_configuration,
            initial_setting: initial_setting,
            initial_funds: if initial_utxos.is_empty() {
                None
            } else {
                Some(initial_utxos)
            },
            legacy_funds: if legacy_utxos.is_empty() {
                None
            } else {
                Some(legacy_utxos)
            },
        }
    }

    pub fn to_block(&self) -> Block {
        let mut builder = BlockBuilder::new();

        builder.message(Message::Initial(
            self.blockchain_configuration.clone().to_ents(),
        ));
        builder.message(self.initial_setting.clone().to_message());

        builder.messages(
            self.to_initial_messages(
                self.initial_setting
                    .max_number_of_transactions_per_block
                    .unwrap_or(255) as usize,
            ),
        );
        builder.messages(
            self.to_legacy_messages(
                self.initial_setting
                    .max_number_of_transactions_per_block
                    .unwrap_or(255) as usize,
            ),
        );

        builder.make_genesis_block()
    }

    fn to_initial_messages(&self, max_output_per_message: usize) -> Vec<Message> {
        let mut messages = Vec::new();
        if let Some(initial_utxos) = &self.initial_funds {
            let mut utxo_iter = initial_utxos.iter();

            while let Some(utxo) = utxo_iter.next() {
                let mut outputs = Vec::with_capacity(max_output_per_message);
                outputs.push(transaction::Output {
                    address: utxo.address.clone(),
                    value: utxo.value,
                });

                while let Some(utxo) = utxo_iter.next() {
                    outputs.push(transaction::Output {
                        address: utxo.address.clone(),
                        value: utxo.value,
                    });
                    if outputs.len() == max_output_per_message {
                        break;
                    }
                }

                let transaction = transaction::AuthenticatedTransaction {
                    transaction: transaction::Transaction {
                        inputs: Vec::new(),
                        outputs: outputs,
                        extra: transaction::NoExtra,
                    },
                    witnesses: Vec::new(),
                };
                messages.push(Message::Transaction(transaction));
            }
        }

        messages
    }
    fn to_legacy_messages(&self, max_output_per_message: usize) -> Vec<Message> {
        let mut messages = Vec::new();
        if let Some(legacy_utxos) = &self.legacy_funds {
            let mut utxo_iter = legacy_utxos.iter();

            while let Some(utxo) = utxo_iter.next() {
                let mut outputs = Vec::with_capacity(max_output_per_message);
                outputs.push((utxo.address.clone(), utxo.value));

                while let Some(utxo) = utxo_iter.next() {
                    outputs.push((utxo.address.clone(), utxo.value));
                    if outputs.len() == max_output_per_message {
                        break;
                    }
                }

                let declaration = legacy::UtxoDeclaration { addrs: outputs };

                messages.push(Message::OldUtxoDeclaration(declaration));
            }
        }

        messages
    }
}

fn get_initial_utxos<'a>(
    messages: &mut std::iter::Peekable<
        std::boxed::Box<
            (dyn std::iter::Iterator<Item = &'a chain_impl_mockchain::message::Message> + 'a),
        >,
    >,
) -> Vec<InitialUTxO> {
    let mut vec = Vec::new();

    while let Some(Message::Transaction(transaction)) = messages.peek() {
        messages.next();
        if !transaction.transaction.inputs.is_empty() {
            panic!("Expected every transaction to not have any inputs");
        }

        for output in transaction.transaction.outputs.iter() {
            let initial_utxo = InitialUTxO {
                address: output.address.clone(),
                value: output.value,
            };

            vec.push(initial_utxo);
        }
    }

    vec
}
fn get_legacy_utxos<'a>(
    messages: &mut std::iter::Peekable<
        std::boxed::Box<
            (dyn std::iter::Iterator<Item = &'a chain_impl_mockchain::message::Message> + 'a),
        >,
    >,
) -> Vec<LegacyUTxO> {
    let mut vec = Vec::new();

    while let Some(Message::OldUtxoDeclaration(old_decls)) = messages.peek() {
        messages.next();
        for (address, value) in old_decls.addrs.iter() {
            let legacy_utxo = LegacyUTxO {
                address: address.clone(),
                value: value.clone(),
            };

            vec.push(legacy_utxo);
        }
    }

    vec
}

impl BlockchainConfiguration {
    fn from_ents(ents: &InitialEnts) -> Self {
        use chain_impl_mockchain::config::ConfigParam;
        let mut block0_date = None;
        let mut block0_consensus = None;
        let mut discrimination = None;

        for ent in ents.iter() {
            match ent {
                ConfigParam::Block0Date(date) => {
                    block0_date =
                        Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(date.0))
                }
                ConfigParam::ConsensusVersion(version) => block0_consensus = Some(version.clone()),
                ConfigParam::Discrimination(d) => discrimination = Some(d.clone()),
            }
        }

        BlockchainConfiguration {
            block0_date: block0_date.unwrap(),
            block0_consensus: block0_consensus.unwrap(),
            discrimination: discrimination.unwrap(),
        }
    }

    fn to_ents(self) -> InitialEnts {
        use chain_impl_mockchain::config::{Block0Date, ConfigParam};
        let mut initial_ents = InitialEnts::new();

        initial_ents.push(ConfigParam::Block0Date(Block0Date(
            self.block0_date
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )));
        initial_ents.push(ConfigParam::ConsensusVersion(self.block0_consensus));
        initial_ents.push(ConfigParam::Discrimination(self.discrimination));

        initial_ents
    }
}

impl Update {
    pub fn to_message(self) -> Message {
        let update = UpdateProposal {
            max_number_of_transactions_per_block: self.max_number_of_transactions_per_block,
            bootstrap_key_slots_percentage: self.bootstrap_key_slots_percentage,
            consensus_version: None,
            bft_leaders: self.bft_leaders.clone().map(|leaders| {
                leaders
                    .iter()
                    .map(|leader| {
                        <PublicKey<Ed25519Extended> as Bech32>::try_from_bech32_str(&leader)
                            .unwrap()
                            .into()
                    })
                    .collect()
            }),
            allow_account_creation: self.allow_account_creation,
            linear_fees: self.linear_fee.map(|linear_fee| LinearFee {
                constant: linear_fee.constant,
                coefficient: linear_fee.coefficient,
                certificate: linear_fee.certificate,
            }),
            slot_duration: Some(self.slot_duration),
            epoch_stability_depth: Some(self.epoch_stability_depth as u32),
        };
        Message::Update(update)
    }
    pub fn from_message(update_proposal: &UpdateProposal) -> Self {
        Update {
            max_number_of_transactions_per_block: update_proposal
                .max_number_of_transactions_per_block,
            bootstrap_key_slots_percentage: update_proposal.bootstrap_key_slots_percentage,
            bft_leaders: update_proposal.bft_leaders.clone().map(|leaders| {
                leaders
                    .iter()
                    .map(|leader| leader.as_public_key().to_bech32_str())
                    .collect()
            }),
            allow_account_creation: update_proposal.allow_account_creation,
            linear_fee: update_proposal
                .linear_fees
                .map(|linear_fee| InitialLinearFee {
                    constant: linear_fee.constant,
                    coefficient: linear_fee.coefficient,
                    certificate: linear_fee.certificate,
                }),
            slot_duration: update_proposal
                .slot_duration
                .expect("slot_duration is mandatory"),
            epoch_stability_depth: update_proposal
                .epoch_stability_depth
                .expect("epoch_stability_depth is mandatory"),
        }
    }
}

pub fn documented_example<W>(mut writer: W, now: std::time::SystemTime) -> std::io::Result<()>
where
    W: std::io::Write,
{
    writeln!(
        writer,
        include_str!("DOCUMENTED_EXAMPLE.yaml"),
        now = now
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    )
}
