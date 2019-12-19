use crate::common::{
    configuration::JormungandrConfig, jcli_wrapper, jormungandr::ConfigurationBuilder,
    process_utils, startup,
};

use chain_impl_mockchain::value::Value;

#[test]
pub fn collect_reward() {
    let actor_account = startup::create_new_account_address();
    let (jormungandr, stake_pool_id) = startup::start_stake_pool(
        &actor_account,
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff("0.999")
            .with_slot_duration(1),
    )
    .unwrap();
    sleep_till_next_epoch(&jormungandr.config);
    let stake_pool_data =
        jcli_wrapper::assert_rest_get_stake_pool(&stake_pool_id, &jormungandr.rest_address());

    assert!(stake_pool_data.rewards.epoch != 0, "zero epoch");
    assert!(
        stake_pool_data.rewards.value_for_stakers != Value::zero(),
        "zero value_for_stakers epoch"
    );
    assert!(
        stake_pool_data.rewards.value_taxed != Value::zero(),
        "zero value_taxed epoch"
    );
}

fn sleep_till_next_epoch(config: &JormungandrConfig) {
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
    let grace_period = 30;
    let wait_time = (slots_per_epoch * slot_duration) + grace_period;
    process_utils::sleep(wait_time.into());
}
