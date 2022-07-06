use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee};
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder, testing::time};
use jormungandr_lib::{
    crypto::{account::Identifier as AccountIdentifier, hash::Hash},
    interfaces::{ActiveSlotCoefficient, Stake, StakeDistributionDto},
};
use std::str::FromStr;
use thor::TransactionHash;

#[test]
pub fn stake_distribution() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let stake_pool_owner_1 = thor::Wallet::default();
    let fee = LinearFee::new(1, 1, 1);
    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner_1.clone()],
        &[sender.clone(), receiver],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_rewards_history()
            .with_linear_fees(fee.clone())
            .with_total_rewards_supply(1_000_000.into())
            .with_slot_duration(3),
    )
    .unwrap();

    assert!(
        jormungandr.rest().stake_distribution_at(1).is_err(),
        "stake distribution for epoch in future should return error"
    );

    let transaction_fee: u64 = fee.constant + fee.coefficient * 2;
    let transaction_amount = 1_000;
    let initial_funds_per_account = 1_000_000_000;
    let stake_pool_id = Hash::from_str(&stake_pools.get(0).unwrap().id().to_string()).unwrap();

    assert_distribution(
        initial_funds_per_account * 2,
        0,
        (stake_pool_id, initial_funds_per_account),
        jormungandr.rest().stake_distribution().unwrap(),
    );

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(
        &sender,
        stake_pool_owner_1.address(),
        transaction_amount.into(),
    )
    .unwrap()
    .encode();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction)
        .assert_in_block();

    time::wait_for_epoch(2, jormungandr.rest());

    let identifier: AccountIdentifier = stake_pool_owner_1.identifier().into();
    let reward: u64 = (*jormungandr
        .rest()
        .epoch_reward_history(1)
        .unwrap()
        .accounts()
        .get(&identifier)
        .unwrap())
    .into();

    jcli.rest().v0().account_stats(
        stake_pool_owner_1.address().to_string(),
        jormungandr.rest_uri(),
    );

    time::wait_for_epoch(3, jormungandr.rest());

    jcli.rest().v0().account_stats(
        stake_pool_owner_1.address().to_string(),
        jormungandr.rest_uri(),
    );

    assert_distribution(
        initial_funds_per_account * 2 - transaction_fee - transaction_amount,
        0,
        (
            stake_pool_id,
            initial_funds_per_account + transaction_amount + reward,
        ),
        jormungandr.rest().stake_distribution_at(3).unwrap(),
    );
}

fn assert_distribution(
    unassigned: u64,
    dangling: u64,
    pool_stake: (Hash, u64),
    stake_distribution_dto: StakeDistributionDto,
) {
    let stake_distribution = stake_distribution_dto.stake;
    assert_eq!(
        Stake::from(unassigned),
        stake_distribution.unassigned,
        "unassigned"
    );
    assert_eq!(
        Stake::from(dangling),
        stake_distribution.dangling,
        "dangling"
    );
    let stake_pool_stake: Stake = *stake_distribution
        .pools
        .iter()
        .find(|(key, _)| *key == pool_stake.0)
        .map(|(_, stake)| stake)
        .unwrap();
    assert_eq!(
        Stake::from(pool_stake.1),
        stake_pool_stake,
        "stake pool {} stake",
        pool_stake.0
    );
}
