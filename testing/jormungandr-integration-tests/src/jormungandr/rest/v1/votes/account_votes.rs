use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    chaintypes::ConsensusType,
    fee::LinearFee,
    ledger::governance::TreasuryGovernanceAction,
    milli::Milli,
    testing::VoteTestGen,
    value::Value,
    vote::{Choice, CommitteeId},
};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{
        ActiveSlotCoefficient, BlockDate as BlockDateDto, CommitteeIdDef, FeesGoTo, KesUpdateSpeed,
        Tally, VotePlanStatus,
    },
};
use jormungandr_testing_utils::testing::VotePlanBuilder;
use jormungandr_testing_utils::testing::asserts::VotePlanStatusAssert;
use jormungandr_testing_utils::testing::startup::start_stake_pool;
use jormungandr_testing_utils::testing::VotePlanExtension;
use jormungandr_testing_utils::testing::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};
use jormungandr_testing_utils::{
    testing::{
        node::time::{self, wait_for_epoch},
        vote_plan_cert, FragmentSender, FragmentSenderSetup,
    },
    wallet::Wallet,
};
use rand::rngs::OsRng;
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

    let alice_account_state = jormungandr.rest().account_state(&alice).unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .with_vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(3, 0))
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


    println!("{:?}",jormungandr.rest().account_votes(vote_plan.to_id().into(),&alice));
    /* 

        .then_wait_for_epoch(2)
        .tally_vote(&mut alice, &vote_plan, VoteTallyPayload::Public)
        .unwrap()
        .and_verify_is_in_block(Duration::from_secs(2))
        .unwrap();*/
}