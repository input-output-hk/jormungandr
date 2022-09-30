use crate::startup;
use chain_addr::Discrimination;
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    fee::LinearFee,
    tokens::minting_policy::MintingPolicy,
    vote::Choice,
};
use jormungandr_automation::{
    jormungandr::ConfigurationBuilder,
    testing::{time, VotePlanBuilder},
};
use jormungandr_lib::interfaces::{AccountVotes, InitialToken};
use std::{collections::HashMap, time::Duration};
use thor::FragmentSenderSetup;

#[test]
pub fn list_cast_votes_for_active_vote_plan() {
    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let wait_time = Duration::from_secs(2);
    let discrimination = Discrimination::Test;

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(20, 0))
        .tally_end(BlockDate::from_epoch_slot_id(30, 0))
        .public()
        .build();

    let jormungandr = startup::start_bft(
        vec![&alice, &bob],
        ConfigurationBuilder::new()
            .with_discrimination(discrimination)
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0))
            .with_token(InitialToken {
                token_id: vote_plan.voting_token().clone().into(),
                policy: MintingPolicy::new().into(),
                to: vec![alice.to_initial_token(1_000)],
            }),
    )
    .unwrap();

    assert!(jormungandr
        .rest()
        .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
        .is_err());
    assert_eq!(
        Some(vec![]),
        jormungandr.rest().account_votes(alice.address()).unwrap()
    );

    let proposals_ids = vec![0u8, 1u8, 2u8];

    thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    )
    .send_vote_plan(&mut alice, &vote_plan)
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .then_wait_for_epoch(1)
    .cast_vote(&mut alice, &vote_plan, proposals_ids[0], &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan, proposals_ids[1], &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan, proposals_ids[2], &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap();

    assert_eq!(
        Some(proposals_ids.clone()),
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
            .unwrap()
    );
    assert_eq!(
        Some(vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: proposals_ids
        }]),
        jormungandr.rest().account_votes(alice.address()).unwrap()
    );
    assert_eq!(
        Some(vec![]),
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), bob.address())
            .unwrap()
    );
    assert_eq!(
        Some(vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![]
        }]),
        jormungandr.rest().account_votes(bob.address()).unwrap()
    );
}

#[test]
pub fn list_cast_votes_for_already_finished_vote_plan() {
    let mut alice = thor::Wallet::default();
    let wait_time = Duration::from_secs(2);
    let discrimination = Discrimination::Test;

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 1))
        .public()
        .build();

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_discrimination(discrimination)
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0))
            .with_token(InitialToken {
                token_id: vote_plan.voting_token().clone().into(),
                policy: MintingPolicy::new().into(),
                to: vec![alice.to_initial_token(1_000_000)],
            }),
    )
    .unwrap();

    let proposals_ids = vec![0u8, 1u8, 2u8];

    thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    )
    .send_vote_plan(&mut alice, &vote_plan)
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .then_wait_for_epoch(1)
    .cast_vote(&mut alice, &vote_plan, 0, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan, 1, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan, 2, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .then_wait_for_epoch(2)
    .tally_vote(&mut alice, &vote_plan, VoteTallyPayload::Public)
    .unwrap()
    .then_wait_for_epoch(3);

    assert_eq!(
        Some(proposals_ids.clone()),
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
            .unwrap()
    );
    assert_eq!(
        Some(vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: proposals_ids
        }]),
        jormungandr.rest().account_votes(alice.address()).unwrap()
    );
}

#[test]
pub fn list_casted_votes_for_non_voted() {
    let alice = thor::Wallet::default();
    let discrimination = Discrimination::Test;

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_discrimination(discrimination)
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0)),
    )
    .unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(20, 0))
        .tally_end(BlockDate::from_epoch_slot_id(30, 0))
        .public()
        .build();

    time::wait_for_epoch(2, jormungandr.rest());

    assert!(jormungandr
        .rest()
        .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
        .is_err());
    assert_eq!(
        Some(vec![]),
        jormungandr.rest().account_votes(alice.address()).unwrap()
    );
}

#[test]
pub fn list_cast_votes_count() {
    let mut alice = thor::Wallet::default();
    let mut bob = thor::Wallet::default();
    let wait_time = Duration::from_secs(2);
    let discrimination = Discrimination::Test;

    let vote_plan_1 = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 1))
        .public()
        .build();

    let vote_plan_2 = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 1))
        .public()
        .build();

    let jormungandr = startup::start_bft(
        vec![&alice, &bob],
        ConfigurationBuilder::new()
            .with_discrimination(discrimination)
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0))
            .with_token(InitialToken {
                token_id: vote_plan_1.voting_token().clone().into(),
                policy: MintingPolicy::new().into(),
                to: vec![
                    alice.to_initial_token(1_000_000),
                    bob.to_initial_token(1_000_000),
                ],
            }),
    )
    .unwrap();

    thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    )
    .send_vote_plan(&mut alice, &vote_plan_1)
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .send_vote_plan(&mut alice, &vote_plan_2)
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .then_wait_for_epoch(1)
    .cast_vote(&mut alice, &vote_plan_1, 0, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan_1, 1, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut alice, &vote_plan_1, 2, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut bob, &vote_plan_2, 0, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut bob, &vote_plan_1, 1, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .cast_vote(&mut bob, &vote_plan_1, 2, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(wait_time)
    .unwrap()
    .then_wait_for_epoch(2)
    .tally_vote(&mut alice, &vote_plan_1, VoteTallyPayload::Public)
    .unwrap()
    .then_wait_for_epoch(3);

    let mut expected_votes_count = HashMap::new();
    expected_votes_count.insert(
        alice.public_key_bech32(),
        vec![AccountVotes {
            vote_plan_id: vote_plan_1.to_id().into(),
            votes: vec![0, 1, 2],
        }],
    );
    let mut votes = vec![
        AccountVotes {
            vote_plan_id: vote_plan_2.to_id().into(),
            votes: vec![0],
        },
        AccountVotes {
            vote_plan_id: vote_plan_1.to_id().into(),
            votes: vec![1, 2],
        },
    ];

    // sort votes by voteplan to ensure consistent results
    votes.sort_by_key(|v| v.vote_plan_id);
    expected_votes_count.insert(bob.public_key_bech32(), votes);

    let mut res = jormungandr.rest().account_votes_all().unwrap();
    for v in res.values_mut() {
        v.sort_by_key(|v| v.vote_plan_id)
    }
    assert_eq!(res, expected_votes_count);
}
