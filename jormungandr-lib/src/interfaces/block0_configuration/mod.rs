mod active_slot_coefficient;
mod bft_slots_ratio;
mod initial_config;
mod initial_fragment;
mod kes_update_speed;
mod leader_id;
mod number_of_slots_per_epoch;
mod slots_duration;

pub use self::active_slot_coefficient::ActiveSlotCoefficient;
pub use self::bft_slots_ratio::BFTSlotsRatio;
pub use self::initial_config::BlockchainConfiguration;
pub use self::initial_fragment::{Certificate, Initial, InitialUTxO, LegacyUTxO};
pub use self::kes_update_speed::KESUpdateSpeed;
pub use self::leader_id::ConsensusLeaderId;
pub use self::number_of_slots_per_epoch::NumberOfSlotsPerEpoch;
pub use self::slots_duration::SlotDuration;
use chain_core::property::HasMessages as _;
use chain_impl_mockchain::{
    block::{Block, BlockBuilder},
    message::Message,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom as _;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block0Configuration {
    /// the initial configuration of the blockchain
    ///
    /// * the start date of the block 0;
    /// * the discrimination;
    /// * ...
    ///
    /// All that is static and does not need to have any update
    /// mechanism.
    pub blockchain_configuration: BlockchainConfiguration,

    /// the initial fragments of the blockchain:
    ///
    /// * initial funds
    /// * initial certificates (delegation, stake pool...)
    #[serde(default)]
    pub initial: Vec<Initial>,
}

custom_error! {pub Block0ConfigurationError
    FirstBlock0MessageNotInit = "Invalid block, expecting the first block fragment to be an special Init fragment",
    BlockchainConfiguration { source: initial_config::FromConfigParamsError } = "blockchain configuration is invalid",
    InitialFragments { source: initial_fragment::Error } = "Invalid fragments"
}

impl Block0Configuration {
    pub fn from_block(block: &Block) -> Result<Self, Block0ConfigurationError> {
        let mut messages = block.messages();

        let blockchain_configuration = match messages.next() {
            Some(Message::Initial(initial)) => BlockchainConfiguration::try_from(initial.clone())?,
            _ => return Err(Block0ConfigurationError::FirstBlock0MessageNotInit),
        };

        Ok(Block0Configuration {
            blockchain_configuration,
            initial: initial_fragment::try_initials_vec_from_messages(messages)?,
        })
    }

    pub fn to_block(&self) -> Block {
        let mut builder = BlockBuilder::new();
        builder.message(Message::Initial(
            self.blockchain_configuration.clone().into(),
        ));
        builder.messages(self.initial.iter().map(Message::from));
        builder.make_genesis_block()
    }
}

pub fn block0_configuration_documented_example() -> String {
    use chain_crypto::{bech32::Bech32 as _, Ed25519, KeyPair, PublicKey, SecretKey};
    use rand_chacha::ChaChaRng;
    use rand_core::SeedableRng as _;

    let mut rng = ChaChaRng::from_seed([0; 32]);

    const DISCRIMINATION: chain_addr::Discrimination = chain_addr::Discrimination::Test;

    let sk: SecretKey<Ed25519> = SecretKey::generate(&mut rng);
    let pk: PublicKey<Ed25519> = sk.to_public();
    let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut rng);
    let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut rng);

    let initial_funds_address = chain_addr::Address(DISCRIMINATION, chain_addr::Kind::Single(pk));
    let initial_funds_address = crate::interfaces::Address::from(initial_funds_address);
    let leader_1_pk = leader_1.public_key().to_bech32_str();
    let leader_2_pk = leader_2.public_key().to_bech32_str();

    format!(
        include_str!("DOCUMENTED_EXAMPLE.yaml"),
        discrimination = DISCRIMINATION,
        default_block0_date = crate::time::SecondsSinceUnixEpoch::default(),
        default_slots_per_epoch = NumberOfSlotsPerEpoch::default(),
        default_slot_duration = SlotDuration::default(),
        default_bft_slots_ratio = BFTSlotsRatio::default(),
        default_consensus_genesis_praos_active_slot_coeff = ActiveSlotCoefficient::default(),
        default_kes_update_speed = KESUpdateSpeed::default(),
        leader_1 = leader_1_pk,
        leader_2 = leader_2_pk,
        initial_funds_address = initial_funds_address
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Block0Configuration {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            const MAX_NUMBER_INITIALS: usize = 64;
            let number_initial = usize::arbitrary(g) % MAX_NUMBER_INITIALS;
            Block0Configuration {
                blockchain_configuration: Arbitrary::arbitrary(g),
                initial: std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(number_initial)
                    .collect(),
            }
        }
    }

    #[test]
    fn documented_example_decodes() {
        let _: Block0Configuration =
            serde_yaml::from_str(&block0_configuration_documented_example()).unwrap();
    }

    quickcheck! {
        fn block0_configuration_serde_human_readable_encode_decode(block0_configuration: Block0Configuration) -> TestResult {
            let s = serde_yaml::to_string(&block0_configuration).unwrap();
            let block0_configuration_dec: Block0Configuration = serde_yaml::from_str(&s).unwrap();

            TestResult::from_bool(block0_configuration == block0_configuration_dec)
        }

        fn block0_configuration_to_block_from_block(block0_configuration: Block0Configuration) -> TestResult {
            let block = block0_configuration.to_block();
            let block0_configuration_dec = Block0Configuration::from_block(&block).unwrap();

            TestResult::from_bool(block0_configuration == block0_configuration_dec)
        }
    }
}
