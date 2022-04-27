use crate::startup;
use chain_impl_mockchain::rewards::{self, CompoundingType, Parameters, Limit};
use jormungandr_automation::jormungandr::ConfigurationBuilder;
use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_lib::interfaces::{SettingsDto, RewardParams, BlockchainConfiguration};
use jormungandr_lib::time::SystemTime;
use rstest::*;

#[fixture]
fn world() -> JormungandrProcess {
    let alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    
    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    jormungandr
}

fn get_parameters_from_blockchain_config(blockchain_configuration: &BlockchainConfiguration) -> Option<Parameters> {   
    let reward_param = 
        match blockchain_configuration.reward_parameters{
            None => return None,
            Some(r) => r
        };
    
    let reward_drawing = 
         match blockchain_configuration.reward_constraints.reward_drawing_limit_max{
            None => Limit::None,
            Some(r) => Limit::ByStakeAbsolute(rewards::Ratio::from(r))
        };

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
            compounding_ratio: rewards::Ratio::from(ratio),
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
            compounding_ratio: rewards::Ratio::from(ratio),
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
        block0_time: SystemTime::from(blockchain_configuration.block0_date),
        //TODO use map?
        curr_slot_start_time: Some(SystemTime::from(blockchain_configuration.block0_date)),
        consensus_version: blockchain_configuration.block0_consensus.to_string(),
        fees: blockchain_configuration.linear_fees,
        block_content_max_size: u32::from(blockchain_configuration.block_content_max_size),
        epoch_stability_depth: u32::from(blockchain_configuration.epoch_stability_depth),
        slot_duration: u8::from(blockchain_configuration.slot_duration) as u64,
        slots_per_epoch: u32::from(blockchain_configuration.slots_per_epoch),
        //TODO handle the unwarp
        treasury_tax: rewards::TaxType::from(blockchain_configuration.treasury_parameters.unwrap()),
        reward_params: get_parameters_from_blockchain_config(blockchain_configuration).unwrap(),
        discrimination: blockchain_configuration.discrimination,
        tx_max_expiry_epochs: blockchain_configuration.tx_max_expiry_epochs.unwrap(),
    }

}

#[rstest]
pub fn test_valid_settings (world: JormungandrProcess) {
    let jormungandr = world;
    let rest_settings = jormungandr.rest().settings().expect("Rest settings error");
    let block0_configuration = jormungandr.block0_configuration();
    let block0_settings = get_settings_from_block0_configuration(block0_configuration);
    assert_eq!(rest_settings,block0_settings);

}
