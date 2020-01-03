use crate::common::{
    configuration::JormungandrConfig, jcli_wrapper, jormungandr::ConfigurationBuilder,
    process_utils, startup,
};
use chain_impl_mockchain::value::Value;
use jormungandr_lib::interfaces::StakePoolStats;

#[ignore]
#[test]
pub fn collect_reward() {
    let stake_pool_owners = [
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
        startup::create_new_account_address(),
    ];
    let (jormungandr, stake_pool_ids) = startup::start_stake_pool(
        &stake_pool_owners,
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff("0.999")
            .with_slot_duration(3),
    )
    .unwrap();
    sleep_till_next_epoch(10, &jormungandr.config);

    let stake_pools_data: Vec<StakePoolStats> = stake_pool_ids
        .iter()
        .map(|x| jcli_wrapper::assert_rest_get_stake_pool(x, &jormungandr.rest_address()))
        .collect();

    // at least one stake pool has reward
    assert!(
        stake_pools_data.iter().any(|x| x.rewards.epoch != 0),
        "zero epoch"
    );
    assert!(
        stake_pools_data
            .iter()
            .any(|x| x.rewards.value_for_stakers != Value::zero()),
        "zero value_for_stakers epoch"
    );
    assert!(
        stake_pools_data
            .iter()
            .any(|x| x.rewards.value_taxed != Value::zero()),
        "zero value_taxed epoch"
    );
}

fn sleep_till_next_epoch(grace_period: u32, config: &JormungandrConfig) {
    let slots_per_epoch = config
        .genesis_yaml
        .blockchain_configuration
        .slots_per_epoch
        .unwrap();
    let slot_duration = config
        .genesis_yaml
        .blockchain_configuration
        .slot_duration
        .unwrap();
    let wait_time = ((slots_per_epoch * slot_duration) * 2) + grace_period;
    process_utils::sleep(wait_time.into());
}
