use chain_core::property::BlockDate;
use chain_impl_mockchain::{certificate::VoteAction, fee::LinearFee, vote::Choice};
use jormungandr_testing_utils::testing::FragmentSenderSetup;
use jormungandr_testing_utils::testing::VotePlanBuilder;
use jormungandr_testing_utils::testing::{jormungandr::ConfigurationBuilder, startup};
use std::time::Duration;

#[test]
pub fn list_casted_votes_for_active_vote_plan() {
    let mut alice = startup::create_new_account_address();
    let wait_time = Duration::from_secs(2);

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
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
        .unwrap();

    println!(
        "{:?}",
        jormungandr
            .rest()
            .account_votes(vote_plan.to_id().into(), &alice)
    );
}
