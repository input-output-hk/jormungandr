mod active_slot_coefficient;
mod block_content_max_size;
mod default_values;
mod epoch_stability_depth;
mod fees_go_to;
mod initial_config;
mod initial_fragment;
mod kes_update_speed;
mod leader_id;
mod number_of_slots_per_epoch;
mod proposal_expiration;
mod reward_constraint;
mod slots_duration;

pub use self::{
    active_slot_coefficient::{ActiveSlotCoefficient, TryFromActiveSlotCoefficientError},
    block_content_max_size::BlockContentMaxSize,
    default_values::*,
    epoch_stability_depth::EpochStabilityDepth,
    fees_go_to::{FeesGoTo, TryFromFeesGoToError},
    initial_config::{BlockchainConfiguration, ConsensusVersionDef, DiscriminationDef},
    initial_fragment::{
        try_initial_fragment_from_message, Destination, Initial, InitialToken, InitialUTxO,
        LegacyUTxO,
    },
    kes_update_speed::{KesUpdateSpeed, TryFromKesUpdateSpeedError},
    leader_id::ConsensusLeaderId,
    number_of_slots_per_epoch::{NumberOfSlotsPerEpoch, TryFromNumberOfSlotsPerEpochError},
    proposal_expiration::ProposalExpiration,
    reward_constraint::{PoolParticipationCapping, RewardConstraints},
    slots_duration::{SlotDuration, TryFromSlotDurationError},
};
use chain_impl_mockchain::{
    block::{self, Block},
    fragment::{ContentsBuilder, Fragment},
    header::{BlockDate, BlockVersion, Header},
};
use serde::{Deserialize, Serialize};
use std::convert::{Infallible, TryFrom as _};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Error)]
pub enum Block0ConfigurationError {
    #[error("Invalid block, expecting the first block fragment to be an special Init fragment")]
    FirstBlock0MessageNotInit,
    #[error("blockchain configuration is invalid")]
    BlockchainConfiguration(#[from] initial_config::FromConfigParamsError),
    #[error("Invalid fragments")]
    InitialFragments(#[from] initial_fragment::Error),
}

impl Block0Configuration {
    pub fn from_block(block: &Block) -> Result<Self, Block0ConfigurationError> {
        let mut messages = block.fragments();

        let blockchain_configuration = match messages.next() {
            Some(Fragment::Initial(initial)) => BlockchainConfiguration::try_from(initial.clone())?,
            _ => return Err(Block0ConfigurationError::FirstBlock0MessageNotInit),
        };

        let discrimination = blockchain_configuration.discrimination;

        Ok(Block0Configuration {
            blockchain_configuration,
            initial: messages
                .map(|message| try_initial_fragment_from_message(discrimination, message))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn to_block(&self) -> Block {
        let mut content_builder = ContentsBuilder::new();
        content_builder.push(Fragment::Initial(
            self.blockchain_configuration.clone().into(),
        ));
        for fragments in self.initial.iter().map(<Vec<Fragment>>::try_from) {
            content_builder.push_many(fragments.unwrap());
        }
        let content = content_builder.into();
        block::builder(BlockVersion::Genesis, content, |hdr| {
            let r: Result<Header, Infallible> = Ok(hdr
                .set_genesis()
                .set_date(BlockDate::first())
                .into_unsigned_header()
                .expect("internal error cannot build unsigned block")
                .generalize());
            r
        })
        .expect("internal error: block builder cannot return error")
    }
}

pub fn block0_configuration_documented_example() -> String {
    use chain_crypto::{bech32::Bech32 as _, Ed25519, KeyPair, PublicKey, SecretKey};
    use rand_chacha::ChaChaRng;
    use rand_core::SeedableRng as _;

    let mut rng = ChaChaRng::from_seed([0; 32]);

    const DISCRIMINATION: chain_addr::Discrimination = chain_addr::Discrimination::Test;

    let sk1: SecretKey<Ed25519> = SecretKey::generate(&mut rng);
    let pk1: PublicKey<Ed25519> = sk1.to_public();

    let sk2: SecretKey<Ed25519> = SecretKey::generate(&mut rng);
    let pk2: PublicKey<Ed25519> = sk2.to_public();

    let leader_1: KeyPair<Ed25519> = KeyPair::generate(&mut rng);
    let leader_2: KeyPair<Ed25519> = KeyPair::generate(&mut rng);

    let initial_funds_address_1 =
        chain_addr::Address(DISCRIMINATION, chain_addr::Kind::Account(pk1));
    let initial_funds_address_1 = crate::interfaces::Address::from(initial_funds_address_1);

    let initial_funds_address_2 =
        chain_addr::Address(DISCRIMINATION, chain_addr::Kind::Account(pk2));
    let initial_funds_address_2 = crate::interfaces::Address::from(initial_funds_address_2);

    let leader_1_pk = leader_1.public_key().to_bech32_str();
    let leader_2_pk = leader_2.public_key().to_bech32_str();

    format!(
        include_str!("BLOCKCHAIN_CONFIGURATION_DOCUMENTED_EXAMPLE.yaml"),
        discrimination = DISCRIMINATION,
        default_block0_date = crate::time::SecondsSinceUnixEpoch::default(),
        default_slots_per_epoch = NumberOfSlotsPerEpoch::default(),
        default_slot_duration = SlotDuration::default(),
        default_consensus_genesis_praos_active_slot_coeff = ActiveSlotCoefficient::default(),
        default_kes_update_speed = KesUpdateSpeed::default(),
        default_block_content_max_size = BlockContentMaxSize::default(),
        default_epoch_stability_depth = EpochStabilityDepth::default(),
        default_proposal_expiration = ProposalExpiration::default(),
        leader_1 = leader_1_pk,
        leader_2 = leader_2_pk,
        initial_funds_address_1 = initial_funds_address_1,
        initial_funds_address_2 = initial_funds_address_2,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interfaces::ARBITRARY_MAX_NUMBER_INITIAL_FRAGMENTS;
    use chain_core::packer::Codec;
    use quickcheck::{Arbitrary, Gen, TestResult};

    impl Arbitrary for Block0Configuration {
        fn arbitrary<G>(g: &mut G) -> Self
        where
            G: Gen,
        {
            let number_initial = usize::arbitrary(g) % ARBITRARY_MAX_NUMBER_INITIAL_FRAGMENTS;
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

        fn block0_configuration_to_serialize(block0_configuration: Block0Configuration) -> TestResult {
            use chain_core::property::{Serialize as _, Deserialize as _};

            let block = block0_configuration.to_block();

            let bytes = block.serialize_as_vec().unwrap();
            let reader = std::io::Cursor::new(&bytes);
            let decoded = Block::deserialize(&mut Codec::new(reader)).unwrap();

            let block0_configuration_dec = Block0Configuration::from_block(&decoded).unwrap();

            TestResult::from_bool(block0_configuration == block0_configuration_dec)
        }
    }
}
