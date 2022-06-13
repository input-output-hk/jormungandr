use crate::startup;
use assert_fs::{prelude::*, TempDir};
use chain_impl_mockchain::{
    accounting::account::{DelegationRatio, DelegationType},
    block::BlockDate,
};
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder, testing::time};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use loki::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use std::time::Duration;
use thor::{BlockDateGenerator, FragmentSender, FragmentSenderSetup, StakePool, Wallet};

#[test]
pub fn test_all_fragments() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let mut faucet = thor::Wallet::default();
    let mut stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();
    let mut split_delegator = thor::Wallet::default();

    let stake_pool_owner_stake = 1_000;

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[faucet.clone()],
        &[full_delegator.clone(), split_delegator.clone()],
        ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let initial_stake_pool = stake_pools.get(0).unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate {
            epoch: 10,
            slot_id: 0,
        }
        .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_transaction(
            &mut faucet,
            &stake_pool_owner,
            &jormungandr,
            stake_pool_owner_stake.into(),
        )
        .unwrap();

    let stake_pool = StakePool::new(&stake_pool_owner);

    transaction_sender
        .send_pool_registration(&mut stake_pool_owner, &stake_pool, &jormungandr)
        .unwrap();

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        stake_pools_from_rest.contains(&stake_pool.id().to_string()),
        "newly created stake pools is not visible in node"
    );

    transaction_sender
        .send_owner_delegation(&mut stake_pool_owner, &stake_pool, &jormungandr)
        .unwrap();

    let stake_pool_owner_info = jcli.rest().v0().account_stats(
        stake_pool_owner.address().to_string(),
        jormungandr.rest_uri(),
    );
    let stake_pool_owner_delegation: DelegationType =
        stake_pool_owner_info.delegation().clone().into();
    assert_eq!(
        stake_pool_owner_delegation,
        DelegationType::Full(stake_pool.id())
    );

    transaction_sender
        .send_full_delegation(&mut full_delegator, &stake_pool, &jormungandr)
        .unwrap();

    let full_delegator_info = jcli
        .rest()
        .v0()
        .account_stats(full_delegator.address().to_string(), jormungandr.rest_uri());
    let full_delegator_delegation: DelegationType = full_delegator_info.delegation().clone().into();
    assert_eq!(
        full_delegator_delegation,
        DelegationType::Full(stake_pool.id())
    );

    transaction_sender
        .send_split_delegation(
            &mut split_delegator,
            &[(initial_stake_pool, 1u8), (&stake_pool, 1u8)],
            &jormungandr,
        )
        .unwrap();

    let split_delegator = jcli.rest().v0().account_stats(
        split_delegator.address().to_string(),
        jormungandr.rest_uri(),
    );
    let delegation_ratio = DelegationRatio::new(
        2,
        vec![(initial_stake_pool.id(), 1u8), (stake_pool.id(), 1u8)],
    )
    .unwrap();
    let split_delegator_delegation: DelegationType = split_delegator.delegation().clone().into();
    assert_eq!(
        split_delegator_delegation,
        DelegationType::Ratio(delegation_ratio)
    );

    let mut new_stake_pool = stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();
    stake_pool_info.serial = 100u128;

    time::wait_for_epoch(1, jormungandr.rest());

    transaction_sender
        .send_pool_update(
            &mut stake_pool_owner,
            &stake_pool,
            &new_stake_pool,
            &jormungandr,
        )
        .unwrap();

    transaction_sender
        .send_pool_retire(&mut stake_pool_owner, &stake_pool, &jormungandr)
        .unwrap();

    let stake_pools_from_rest = jormungandr
        .rest()
        .stake_pools()
        .expect("cannot retrieve stake pools id from rest");
    assert!(
        !stake_pools_from_rest.contains(&stake_pool.id().to_string()),
        "newly created stake pools is not visible in node"
    );
}

#[test]
pub fn test_all_adversary_fragments() {
    let temp_dir = TempDir::new().unwrap();

    let mut faucet = thor::Wallet::default();
    let stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();
    let split_delegator = thor::Wallet::default();

    let stake_pool_owner_stake = 1_000;

    let (jormungandr, stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[full_delegator.clone(), split_delegator, faucet.clone()],
        ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let initial_stake_pool = stake_pools.get(0).unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        FragmentSenderSetup::resend_3_times(),
    );

    let adversary_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        AdversaryFragmentSenderSetup::no_verify(),
    );
    let verifier = jormungandr
        .correct_state_verifier()
        .record_address_state(vec![&faucet.address(), &stake_pool_owner.address()]);
    adversary_sender
        .send_faulty_transactions_with_iteration_delay(
            10,
            &mut faucet,
            &stake_pool_owner,
            &jormungandr,
            Duration::from_secs(5),
        )
        .unwrap();
    adversary_sender
        .send_faulty_full_delegation(
            BlockDate::first().next_epoch(),
            &mut full_delegator,
            initial_stake_pool.id(),
            &jormungandr,
        )
        .unwrap();
    transaction_sender
        .send_transaction(
            &mut faucet,
            &stake_pool_owner,
            &jormungandr,
            stake_pool_owner_stake.into(),
        )
        .unwrap();

    verifier
        .value_moved_between_addresses(
            &faucet.address(),
            &stake_pool_owner.address(),
            stake_pool_owner_stake.into(),
        )
        .unwrap();
}

#[test]
pub fn test_increased_block_content_max_size() {
    let receivers: Vec<Wallet> = std::iter::from_fn(|| Some(thor::Wallet::default()))
        .take(98)
        .collect();
    let mut stake_pool_owner = thor::Wallet::default();

    let stake_pool_owner_stake = 1;

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_block_content_max_size(8192.into()),
    )
    .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDateGenerator::rolling(
            &settings,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            false,
        ),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_transaction_to_many(
            &mut stake_pool_owner,
            &receivers,
            &jormungandr,
            stake_pool_owner_stake.into(),
        )
        .unwrap();
}

#[test]
pub fn test_block_content_max_size_below_transaction_size() {
    let receivers: Vec<Wallet> = std::iter::from_fn(|| Some(thor::Wallet::default()))
        .take(98)
        .collect();
    let mut stake_pool_owner = thor::Wallet::default();

    let stake_pool_owner_stake = 1;

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[stake_pool_owner.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_block_content_max_size(4092.into()),
    )
    .unwrap();

    let fragment_sender = FragmentSender::from_with_setup(
        jormungandr.block0_configuration(),
        FragmentSenderSetup::should_stop_at_error(),
    );
    assert!(fragment_sender
        .send_transaction_to_many(
            &mut stake_pool_owner,
            &receivers,
            &jormungandr,
            stake_pool_owner_stake.into(),
        )
        .is_err());
}
