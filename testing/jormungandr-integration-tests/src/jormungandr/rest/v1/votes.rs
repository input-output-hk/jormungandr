use chain_addr::Discrimination;
use chain_core::property::BlockDate;
use chain_impl_mockchain::certificate::VoteTallyPayload;
use chain_impl_mockchain::{certificate::VoteAction, fee::LinearFee, vote::Choice};
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::FragmentSenderSetup;
use jormungandr_testing_utils::testing::VotePlanBuilder;
use jormungandr_testing_utils::testing::{jormungandr::ConfigurationBuilder, startup};
use std::time::Duration;

#[test]
pub fn list_casted_votes_for_active_vote_plan() {
    let mut alice = startup::create_new_account_address();
    let bob = startup::create_new_account_address();
    let wait_time = Duration::from_secs(2);
    let discrimination = Discrimination::Test;

    let jormungandr = startup::start_bft(
        vec![&alice, &bob],
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
        .with_vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(20, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(30, 0))
        .public()
        .build();

    assert!(jormungandr
        .rest()
        .account_votes(vote_plan.to_id().into(), &alice)
        .is_err());

    let proposals_ids = vec![0u8, 1u8, 2u8];

    jormungandr
        .fragment_chain_sender(FragmentSenderSetup::no_verify())
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
        Some(proposals_ids),
        jormungandr
            .rest()
            .account_votes(vote_plan.to_id().into(), &alice)
            .unwrap()
    );
    assert_eq!(
        Some(vec![]),
        jormungandr
            .rest()
            .account_votes(vote_plan.to_id().into(), &bob)
            .unwrap()
    );
}

#[test]
pub fn list_casted_votes_for_already_finished_vote_plan() {
    let mut alice = startup::create_new_account_address();
    let wait_time = Duration::from_secs(2);
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
        .with_vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(2, 1))
        .public()
        .build();

    let proposals_ids = vec![0u8, 1u8, 2u8];

    jormungandr
        .fragment_chain_sender(FragmentSenderSetup::no_verify())
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
        Some(proposals_ids),
        jormungandr
            .rest()
            .account_votes(vote_plan.to_id().into(), &alice)
            .unwrap()
    );
}

#[test]
pub fn list_casted_votes_for_non_voted() {
    let alice = startup::create_new_account_address();
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
        .with_vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(20, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(30, 0))
        .public()
        .build();

    time::wait_for_epoch(2, jormungandr.rest());

    assert!(jormungandr
        .rest()
        .account_votes(vote_plan.to_id().into(), &alice)
        .is_err());
}
