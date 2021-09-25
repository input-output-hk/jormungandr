use assert_fs::TempDir;
use chain_core::property::BlockDate;
use chain_impl_mockchain::vote::Choice;
use jormungandr_testing_utils::testing::{
    jcli::JCli, jormungandr::starter::Starter, jormungandr::ConfigurationBuilder,
};
use jormungandr_testing_utils::{
    testing::{vote_plan_cert, FragmentSender, FragmentSenderSetup, VotePlanBuilder},
    wallet::Wallet,
};

#[test]
pub fn test_correct_proposal_number_is_returned() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = Wallet::new_account(&mut rand::thread_rng());

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(
        &alice,
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        &vote_plan,
    )
    .into();
    let wallets = [&alice];
    let config = ConfigurationBuilder::new()
        .with_funds(wallets.iter().map(|x| x.to_initial_fund(1000)).collect())
        .with_committees(&wallets)
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .config(config)
        .temp_dir(temp_dir)
        .start()
        .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        }
        .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &Choice::new(0), &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 2, &Choice::new(0), &jormungandr)
        .unwrap();

    let jcli: JCli = Default::default();
    let rest_uri = jormungandr.rest_uri();
    let account_votes = jcli.rest().v1().vote().account_votes(
        alice.address().to_string(),
        vote_plan.to_id().to_string(),
        rest_uri,
    );
    assert_eq!(&account_votes, &[0, 2]);
}
