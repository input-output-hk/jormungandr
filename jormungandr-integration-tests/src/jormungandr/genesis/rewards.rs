use crate::common::{jcli_wrapper, jormungandr::ConfigurationBuilder, startup};

use chain_impl_mockchain::value::Value;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, EpochRewardsInfo, StakePoolStats, Value as LibValue},
};
use std::str::FromStr;

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
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3),
    )
    .unwrap();
    startup::sleep_till_next_epoch(10, &jormungandr.config);

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

#[test]
pub fn reward_history() {
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
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_rewards_history()
            .with_slot_duration(3),
    )
    .unwrap();

    let empty_vec: Vec<EpochRewardsInfo> = Vec::new();

    assert_eq!(
        empty_vec,
        jormungandr.rest().reward_history(1).unwrap(),
        "reward history for epoch in the future should be empty"
    );
    assert!(
        jormungandr.rest().epoch_reward_history(1).is_err(),
        "reward per epoch for epoch in the future should return error"
    );

    assert_eq!(
        empty_vec,
        jormungandr.rest().reward_history(0).unwrap(),
        "reward history for current epoch should be empty"
    );
    assert!(
        jormungandr.rest().epoch_reward_history(0).is_err(),
        "reward per epoch for current epoch in the future should return error"
    );

    startup::sleep_till_next_epoch(10, &jormungandr.config);

    let history = jormungandr.rest().reward_history(1).unwrap();
    let epoch_reward_info_from_history = history.get(0).unwrap();

    let epoch_reward_info_from_epoch = jormungandr
        .rest()
        .epoch_reward_history(epoch_reward_info_from_history.clone().epoch().into())
        .unwrap();
    assert_eq!(
        *epoch_reward_info_from_history, epoch_reward_info_from_epoch,
        "reward history is not equal to reward by epoch"
    );

    let stake_pools_data: Vec<(Hash, StakePoolStats)> = stake_pool_ids
        .iter()
        .map(|x| {
            (
                Hash::from_str(x).unwrap(),
                jcli_wrapper::assert_rest_get_stake_pool(x, &jormungandr.rest_address()),
            )
        })
        .collect();

    for (stake_pool_hash, (value_taxed, _)) in epoch_reward_info_from_epoch.stake_pools() {
        let (_, stake_pool_data) = stake_pools_data
            .iter()
            .find(|(x, _)| x == stake_pool_hash)
            .unwrap();
        let actual_value_taxed: LibValue = stake_pool_data.rewards.value_taxed.into();
        let value_for_stakers: LibValue = stake_pool_data.rewards.value_for_stakers.into();
        assert_eq!(value_taxed.clone(), actual_value_taxed, "value taxed");
        assert_eq!(
            value_for_stakers.clone(),
            value_for_stakers,
            "value for stakers"
        );
    }
}
