use chain_addr::{Address, Discrimination};
use chain_addr::{AddressReadable, Kind};
use chain_core::property::HasMessages;
use chain_crypto::bech32::Bech32;
use chain_crypto::{Ed25519, Ed25519Extended, KeyPair, PublicKey, SecretKey};
use chain_impl_mockchain::{
    block::{Block, BlockBuilder, ConsensusVersion},
    certificate::Certificate,
    config::{Block0Date, ConfigParam},
    fee::LinearFee,
    legacy::{OldAddress, UtxoDeclaration},
    message::{ConfigParams, Message},
    milli::Milli,
    transaction::{AuthenticatedTransaction, NoExtra, Output, Transaction},
    value::Value,
};
use jormungandr_utils::serde::{self, SerdeAsString, SerdeLeaderId};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Genesis {
    /// the initial configuration of the blockchain
    ///
    /// * the start date of the block 0;
    /// * the discrimination;
    /// * ...
    ///
    /// All that is static and does not need to have any update
    /// mechanism.
    blockchain_configuration: BlockchainConfiguration,
    #[serde(default)]
    initial: Vec<Initial>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockchainConfiguration {
    block0_date: u64,
    #[serde(with = "serde::as_string")]
    discrimination: Discrimination,
    #[serde(with = "serde::as_string")]
    block0_consensus: ConsensusVersion,
    slots_per_epoch: Option<u32>,
    slot_duration: u8,
    epoch_stability_depth: Option<u32>,
    #[serde(default)]
    consensus_leader_ids: Vec<SerdeLeaderId>,
    #[serde(with = "serde::as_string")]
    consensus_genesis_praos_active_slot_coeff: Milli,
    max_number_of_transactions_per_block: Option<u32>,
    bft_slots_ratio: Option<SerdeAsString<Milli>>,
    linear_fees: Option<InitialLinearFee>,
    kes_update_speed: u32,
}

// FIXME: duplicates LinearFee, can we get rid of this?
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialLinearFee {
    coefficient: u64,
    constant: u64,
    certificate: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Initial {
    Fund(InitialUTxO),
    Cert(InitialCertificate),
    LegacyFund(LegacyUTxO),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent, deny_unknown_fields)]
struct InitialCertificate(#[serde(with = "serde::certificate")] Certificate);

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialUTxO {
    #[serde(with = "serde::address")]
    pub address: Address,
    #[serde(with = "serde::value")]
    pub value: Value,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyUTxO {
    //pub address: OldAddress,
    #[serde(with = "serde::value")]
    pub value: Value,
}

type StaticStr = &'static str;

custom_error! {pub Error
    FirstBlock0MessageNotInit = "first message of block 0 is not initial",
    Block0MessageUnexpected  = "non-first message of block 0 has unexpected type",
    InitUtxoHasInput = "initial UTXO has input",
    InitConfigParamMissing { name: StaticStr } = "initial message misses parameter {name}",
    InitConfigParamDuplicate { name: StaticStr } = "initial message contains duplicate parameter {name}",
}

impl Genesis {
    pub fn from_block(block: &Block) -> Result<Self, Error> {
        let mut messages = block.messages();

        let blockchain_configuration = match messages.next() {
            Some(Message::Initial(initial)) => BlockchainConfiguration::from_ents(initial),
            _ => Err(Error::FirstBlock0MessageNotInit),
        }?;

        Ok(Genesis {
            blockchain_configuration,
            initial: try_initials_vec_from_messages(messages)?,
        })
    }

    pub fn to_block(&self) -> Block {
        let mut builder = BlockBuilder::new();
        builder.message(Message::Initial(
            self.blockchain_configuration.clone().to_ents(),
        ));
        builder.messages(self.initial.iter().map(Message::from));
        builder.make_genesis_block()
    }
}

impl BlockchainConfiguration {
    fn from_ents(ents: &ConfigParams) -> Result<Self, Error> {
        let mut block0_date = None;
        let mut discrimination = None;
        let mut block0_consensus = None;
        let mut slots_per_epoch = None;
        let mut slot_duration = None;
        let mut epoch_stability_depth = None;
        let mut consensus_leader_ids = vec![];
        let mut consensus_genesis_praos_active_slot_coeff = None;
        let mut max_number_of_transactions_per_block = None;
        let mut bft_slots_ratio = None;
        let mut linear_fees = None;
        let mut kes_update_speed = None;

        for ent in ents.iter() {
            match ent {
                ConfigParam::Block0Date(param) => {
                    block0_date.replace(param.0).map(|_| "Block0Date")
                }
                ConfigParam::ConsensusVersion(param) => {
                    block0_consensus.replace(*param).map(|_| "ConsensusVersion")
                }
                ConfigParam::Discrimination(param) => {
                    discrimination.replace(*param).map(|_| "Discrimination")
                }
                ConfigParam::SlotsPerEpoch(param) => {
                    slots_per_epoch.replace(*param).map(|_| "SlotsPerEpoch")
                }
                ConfigParam::SlotDuration(param) => {
                    slot_duration.replace(*param).map(|_| "SlotDuration")
                }
                ConfigParam::EpochStabilityDepth(param) => epoch_stability_depth
                    .replace(*param)
                    .map(|_| "EpochStabilityDepth"),
                ConfigParam::AddBftLeader(param) => {
                    consensus_leader_ids.push(SerdeLeaderId(param.clone()));
                    None
                }
                ConfigParam::RemoveBftLeader(_) => {
                    panic!("block 0 attempts to remove a BFT leader")
                }
                ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(param) => {
                    consensus_genesis_praos_active_slot_coeff
                        .replace(*param)
                        .map(|_| "ConsensusGenesisPraosActiveSlotsCoeff")
                }
                ConfigParam::MaxNumberOfTransactionsPerBlock(param) => {
                    max_number_of_transactions_per_block
                        .replace(*param)
                        .map(|_| "MaxNumberOfTransactionsPerBlock")
                }
                ConfigParam::BftSlotsRatio(param) => bft_slots_ratio
                    .replace(SerdeAsString(*param))
                    .map(|_| "BftSlotsRatio"),
                ConfigParam::LinearFee(param) => linear_fees
                    .replace(InitialLinearFee {
                        constant: param.constant,
                        coefficient: param.coefficient,
                        certificate: param.certificate,
                    })
                    .map(|_| "LinearFee"),
                ConfigParam::ProposalExpiration(_param) => unimplemented!(),
                ConfigParam::KESUpdateSpeed(v) => {
                    kes_update_speed.replace(*v).map(|_| "KESUpdateSpeed")
                }
            }
            .map(|name| Err(Error::InitConfigParamDuplicate { name }))
            .unwrap_or(Ok(()))?;
        }

        Ok(BlockchainConfiguration {
            block0_date: block0_date.ok_or(param_missing_error("Block0Date"))?,
            discrimination: discrimination.ok_or(param_missing_error("Discrimination"))?,
            block0_consensus: block0_consensus.ok_or(param_missing_error("Block0Consensus"))?,
            slots_per_epoch,
            slot_duration: slot_duration.ok_or(param_missing_error("SlotDuration"))?,
            epoch_stability_depth,
            consensus_leader_ids,
            consensus_genesis_praos_active_slot_coeff: consensus_genesis_praos_active_slot_coeff
                .ok_or(param_missing_error("ActiveSlotCoeff"))?,
            max_number_of_transactions_per_block,
            bft_slots_ratio,
            linear_fees,
            kes_update_speed: kes_update_speed.ok_or(param_missing_error("KESUpdateSpeed"))?,
        })
    }

    fn to_ents(self) -> ConfigParams {
        // Generate warning or error for each unused field
        let BlockchainConfiguration {
            block0_date,
            discrimination,
            block0_consensus,
            slots_per_epoch,
            slot_duration,
            epoch_stability_depth,
            consensus_leader_ids,
            consensus_genesis_praos_active_slot_coeff,
            max_number_of_transactions_per_block,
            bft_slots_ratio,
            linear_fees,
            kes_update_speed,
        } = self;
        let mut initial_ents = ConfigParams::new();
        initial_ents.push(ConfigParam::Block0Date(Block0Date(block0_date)));
        initial_ents.push(ConfigParam::Discrimination(discrimination));
        initial_ents.push(ConfigParam::ConsensusVersion(block0_consensus));
        if let Some(slots_per_epoch) = slots_per_epoch {
            initial_ents.push(ConfigParam::SlotsPerEpoch(slots_per_epoch))
        }
        initial_ents.push(ConfigParam::SlotDuration(slot_duration));
        if let Some(epoch_stability_depth) = epoch_stability_depth {
            initial_ents.push(ConfigParam::EpochStabilityDepth(epoch_stability_depth))
        }
        for leader_id in consensus_leader_ids {
            initial_ents.push(ConfigParam::AddBftLeader(leader_id.0))
        }
        initial_ents.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            consensus_genesis_praos_active_slot_coeff,
        ));
        if let Some(d) = max_number_of_transactions_per_block {
            initial_ents.push(ConfigParam::MaxNumberOfTransactionsPerBlock(d))
        }
        if let Some(d) = bft_slots_ratio {
            initial_ents.push(ConfigParam::BftSlotsRatio(d.0))
        }
        if let Some(d) = linear_fees {
            initial_ents.push(ConfigParam::LinearFee(LinearFee {
                constant: d.constant,
                coefficient: d.coefficient,
                certificate: d.certificate,
            }))
        }
        initial_ents.push(ConfigParam::KESUpdateSpeed(kes_update_speed));
        initial_ents
    }
}

fn param_missing_error(name: &'static str) -> Error {
    Error::InitConfigParamMissing { name }
}

fn try_initials_vec_from_messages<'a>(
    mut messages: impl Iterator<Item = &'a Message>,
) -> Result<Vec<Initial>, Error> {
    let mut inits = Vec::new();
    for message in messages {
        match message {
            Message::Transaction(tx) => try_extend_inits_with_tx(&mut inits, tx)?,
            Message::Certificate(tx) => extend_inits_with_cert(&mut inits, tx),
            Message::OldUtxoDeclaration(utxo) => extend_inits_with_legacy_utxo(&mut inits, utxo),
            _ => return Err(Error::Block0MessageUnexpected),
        }
    }
    Ok(inits)
}

fn try_extend_inits_with_tx(
    initials: &mut Vec<Initial>,
    tx: &AuthenticatedTransaction<Address, NoExtra>,
) -> Result<(), Error> {
    if !tx.transaction.inputs.is_empty() {
        return Err(Error::InitUtxoHasInput);
    }
    let inits_iter = tx
        .transaction
        .outputs
        .iter()
        .map(|output| InitialUTxO {
            address: output.address.clone(),
            value: output.value,
        })
        .map(Initial::Fund);
    initials.extend(inits_iter);
    Ok(())
}

fn extend_inits_with_cert(
    initials: &mut Vec<Initial>,
    tx: &AuthenticatedTransaction<Address, Certificate>,
) {
    let cert = InitialCertificate(tx.transaction.extra.clone());
    initials.push(Initial::Cert(cert))
}

fn extend_inits_with_legacy_utxo(initials: &mut Vec<Initial>, utxo_decl: &UtxoDeclaration) {
    let inits_iter = utxo_decl
        .addrs
        .iter()
        .map(|(_address, value)| LegacyUTxO {
            //address: address.clone(),
            value: value.clone(),
        })
        .map(Initial::LegacyFund);
    initials.extend(inits_iter)
}

impl<'a> From<&'a Initial> for Message {
    fn from(initial: &'a Initial) -> Message {
        match initial {
            Initial::Fund(utxo) => utxo.into(),
            Initial::Cert(cert) => cert.into(),
            Initial::LegacyFund(utxo) => utxo.into(),
        }
    }
}

impl<'a> From<&'a InitialUTxO> for Message {
    fn from(utxo: &'a InitialUTxO) -> Message {
        Message::Transaction(AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![Output {
                    address: utxo.address.clone(),
                    value: utxo.value,
                }],
                extra: NoExtra,
            },
            witnesses: vec![],
        })
    }
}

impl<'a> From<&'a InitialCertificate> for Message {
    fn from(utxo: &'a InitialCertificate) -> Message {
        Message::Certificate(AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![],
                extra: utxo.0.clone(),
            },
            witnesses: vec![],
        })
    }
}

impl<'a> From<&'a LegacyUTxO> for Message {
    fn from(utxo: &'a LegacyUTxO) -> Message {
        Message::OldUtxoDeclaration(UtxoDeclaration {
            addrs: vec![], // vec![(utxo.address.clone(), utxo.value)],
        })
    }
}

pub fn documented_example(now: std::time::SystemTime) -> String {
    let secs = now
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sk: SecretKey<Ed25519Extended> = SecretKey::generate(&mut ChaChaRng::from_seed([0; 32]));
    let pk: PublicKey<Ed25519> = sk.to_public();
    let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
    let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));

    let initial_funds_address = Address(Discrimination::Test, Kind::Single(pk));
    let initial_funds_address = AddressReadable::from_address(&initial_funds_address).to_string();
    let leader_1_pk = leader_1.public_key().to_bech32_str();
    let leader_2_pk = leader_2.public_key().to_bech32_str();
    format!(
        include_str!("DOCUMENTED_EXAMPLE.yaml"),
        now = secs,
        leader_1 = leader_1_pk,
        leader_2 = leader_2_pk,
        initial_funds_address = initial_funds_address
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml;

    #[test]
    fn conversion_to_and_from_message_preserves_data() {
        let sk: SecretKey<Ed25519Extended> =
            SecretKey::generate(&mut ChaChaRng::from_seed([0; 32]));
        let pk: PublicKey<Ed25519> = sk.to_public();

        let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([1; 32]));
        let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut ChaChaRng::from_seed([2; 32]));

        let initial_funds_address = Address(Discrimination::Test, Kind::Single(pk));
        let initial_funds_address =
            AddressReadable::from_address(&initial_funds_address).to_string();

        let leader_1_pk = leader_1.public_key().to_bech32_str();
        let leader_2_pk = leader_2.public_key().to_bech32_str();

        let genesis_yaml = format!(r#"
---
blockchain_configuration:
  block0_date: 123456789
  discrimination: test
  block0_consensus: bft
  slots_per_epoch: 5
  slot_duration: 15
  epoch_stability_depth: 10
  consensus_leader_ids:
    - {}
    - {}
  consensus_genesis_praos_active_slot_coeff: "0.444"
  max_number_of_transactions_per_block: 255
  bft_slots_ratio: "0.222"
  linear_fees:
    coefficient: 1
    constant: 2
    certificate: 4
  kes_update_speed: 43200
initial:
  - cert: cert1qgqqqqqqqqqqqqqqqqqqq0p5avfqqmgurpe7s9k7933q0wj420jl5xqvx8lywcu5jcr7fwqa9qmdn93q4nm7c4fsay3mzeqgq3c0slnut9kns08yn2qn80famup7nvgtfuyszqzqrd4lxlt5ylplfu76p8f6ks0ggprzatp2c8rn6ev3hn9dgr38tzful4h0udlwa0536vyrrug7af9ujmrr869afs0yw9gj5x7z24l8sps3zzcmv
  - fund:
      address: {}
      value: 10000"#, leader_1_pk, leader_2_pk, initial_funds_address);
        let genesis: Genesis =
            serde_yaml::from_str(genesis_yaml.as_str()).expect("Failed to deserialize YAML");

        let block = genesis.to_block();
        let new_genesis = Genesis::from_block(&block).expect("Failed to build genesis");

        let new_genesis_yaml =
            serde_yaml::to_string(&new_genesis).expect("Failed to serialize YAML");
        assert_eq!(
            genesis_yaml.trim(),
            new_genesis_yaml,
            "\nGenesis YAML has changed after conversions:\n{}\n",
            new_genesis_yaml
        );
    }
}
