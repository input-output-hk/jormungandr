use crate::startup;
use chain_impl_mockchain::value::Value;
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder, testing::time};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, EpochRewardsInfo, StakePoolStats, Value as LibValue},
};
use std::str::FromStr;

#[test]
pub fn collect_reward() {
    let jcli: JCli = Default::default();
    let stake_pool_owners = [
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
    ];
    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &stake_pool_owners,
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_total_rewards_supply(1_000_000.into()),
    )
    .unwrap();

    time::wait_for_epoch(2, jormungandr.rest());

    let stake_pools_data: Vec<StakePoolStats> = stake_pools
        .iter()
        .map(|x| {
            jcli.rest()
                .v0()
                .stake_pool(x.id().to_string(), jormungandr.rest_uri())
        })
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
    let jcli: JCli = Default::default();

    let stake_pool_owners = [
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
        thor::Wallet::default(),
    ];
    let (jormungandr, stake_pools) = startup::start_stake_pool(
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

    time::wait_for_epoch(2, jormungandr.rest());

    let history = jormungandr.rest().reward_history(1).unwrap();
    let epoch_reward_info_from_history = history.get(0).unwrap();

    let epoch_reward_info_from_epoch = jormungandr
        .rest()
        .epoch_reward_history(epoch_reward_info_from_history.epoch())
        .unwrap();
    assert_eq!(
        *epoch_reward_info_from_history, epoch_reward_info_from_epoch,
        "reward history is not equal to reward by epoch"
    );

    let stake_pools_data: Vec<(Hash, StakePoolStats)> = stake_pools
        .iter()
        .map(|x| {
            (
                Hash::from_str(&x.id().to_string()).unwrap(),
                jcli.rest()
                    .v0()
                    .stake_pool(x.id().to_string(), jormungandr.rest_uri()),
            )
        })
        .collect();

    for (stake_pool_hash, (value_taxed, _value_for_stakers)) in
        epoch_reward_info_from_epoch.stake_pools()
    {
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
