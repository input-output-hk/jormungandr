use std::num::{NonZeroU64, NonZeroU32};
use crate::startup;
use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use chain_impl_mockchain::rewards::{CompoundingType, Parameters, Limit};
use jormungandr_automation::jormungandr::ConfigurationBuilder;
use jormungandr_lib::interfaces::{Block0Configuration, TaxType, Ratio};
use jormungandr_lib::interfaces::{SettingsDto, RewardParams, BlockchainConfiguration};
use jormungandr_lib::time::SystemTime;
use rstest::*;

fn get_parameters_from_blockchain_config(blockchain_configuration: &BlockchainConfiguration) -> Option<Parameters> {
    let reward_param =
        match blockchain_configuration.reward_parameters{
            None => return None,
            Some(r) => r
        };

    let reward_drawing =
         match blockchain_configuration.reward_constraints.reward_drawing_limit_max{
            None => Limit::None,
            Some(r) => Limit::ByStakeAbsolute(r.into())
        };
    //TODO use map?
    let pool_participation =
        match blockchain_configuration.reward_constraints.pool_participation_capping
         {
            None => None,
            Some(p) => Some((p.min, p.max))
        };

    match reward_param {
        RewardParams::Linear {
            constant,
            ratio,
            epoch_start,
            epoch_rate,
        } => Some(Parameters {
            initial_value: constant,
            compounding_ratio: ratio.into(),
            compounding_type: CompoundingType::Linear,
            epoch_rate,
            epoch_start,
            reward_drawing_limit_max: reward_drawing,
            pool_participation_capping: pool_participation,
        }),
        RewardParams::Halving {
            constant,
            ratio,
            epoch_start,
            epoch_rate,
        } => Some(Parameters {
            initial_value: constant,
            compounding_ratio: ratio.into(),
            compounding_type: CompoundingType::Halvening,
            epoch_rate,
            epoch_start,
            reward_drawing_limit_max: reward_drawing,
            pool_participation_capping: pool_participation,
        }),
    }
}

fn get_settings_from_block0_configuration(block0_configuration: &Block0Configuration) -> SettingsDto{
    let blockchain_configuration = &block0_configuration.blockchain_configuration;
    SettingsDto {
        block0_hash: block0_configuration.to_block().header().id().to_string(),
        block0_time: blockchain_configuration.block0_date.into(),
        //TODO use map?
        curr_slot_start_time: Some(SystemTime::from(blockchain_configuration.block0_date)),
        consensus_version: blockchain_configuration.block0_consensus.to_string(),
        fees: blockchain_configuration.linear_fees,
        block_content_max_size: blockchain_configuration.block_content_max_size.into(),
        epoch_stability_depth: blockchain_configuration.epoch_stability_depth.into(),
        slot_duration: u8::from(blockchain_configuration.slot_duration).into(),
        slots_per_epoch: blockchain_configuration.slots_per_epoch.into(),
        //TODO handle the unwarp
        treasury_tax: blockchain_configuration.treasury_parameters.unwrap().into(),
        reward_params: get_parameters_from_blockchain_config(blockchain_configuration).unwrap(),
        discrimination: blockchain_configuration.discrimination,
        tx_max_expiry_epochs: blockchain_configuration.tx_max_expiry_epochs.unwrap(),
    }

}

#[rstest]
pub fn test_default_settings () {
    let alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    ).unwrap();

    let rest_settings = jormungandr.rest().settings().expect("Rest settings error");
    let block0_settings = get_settings_from_block0_configuration(jormungandr.block0_configuration());
    assert_eq!(rest_settings,block0_settings);
}

#[rstest]
pub fn test_custom_settings () {
    let alice = thor::Wallet::default();

    let mut linear_fees = LinearFee::new(1, 2, 1);
    linear_fees.per_certificate_fees(PerCertificateFee::new(NonZeroU64::new(2), NonZeroU64::new(3), NonZeroU64::new(1)));
    linear_fees.per_vote_certificate_fees(PerVoteCertificateFee::new(NonZeroU64::new(3), NonZeroU64::new(3)));

    let treasury_parameters = TaxType {
        fixed: 200.into(),
        ratio: Ratio::new_checked(10, 500).unwrap(),
        max_limit: NonZeroU64::new(200),
    };

    let reward_parameters = RewardParams::Linear {
        constant: 500_000,
        ratio: Ratio::new_checked(5, 2_00).unwrap(),
        epoch_start: 2,
        epoch_rate: NonZeroU32::new(4).unwrap(),
    };

    let jormungandr = startup::start_bft(
        vec![&alice],
        &mut ConfigurationBuilder::new()
        .with_linear_fees(linear_fees)
        .with_block_content_max_size(2000.into())
        .with_epoch_stability_depth(2000)
        .with_slot_duration(1)
        .with_slots_per_epoch(6)
        .with_treasury_parameters(treasury_parameters)
        .with_reward_parameters(reward_parameters)
        .with_tx_max_expiry_epochs(50),
    ).unwrap();

    let rest_settings = jormungandr.rest().settings().expect("Rest settings error");
    let block0_settings = get_settings_from_block0_configuration(jormungandr.block0_configuration());
    assert_eq!(rest_settings,block0_settings);
}
