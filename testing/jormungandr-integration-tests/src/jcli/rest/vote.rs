use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_core::property::BlockDate;
use chain_impl_mockchain::{tokens::minting_policy::MintingPolicy, vote::Choice};
use jormungandr_automation::{
    jcli::JCli, jormungandr::Block0ConfigurationBuilder, testing::VotePlanBuilder,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Initial, InitialToken, NumberOfSlotsPerEpoch},
};
use std::str::FromStr;
use thor::{vote_plan_cert, FragmentSender, FragmentSenderSetup, Wallet};

#[test]
pub fn test_correct_proposal_number_is_returned() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
        .build();

    let vote_plan_cert = Initial::Cert(
        vote_plan_cert(
            &alice,
            chain_impl_mockchain::block::BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            &vote_plan,
        )
        .into(),
    );
    let wallets = [&alice];
    let block0_builder = Block0ConfigurationBuilder::default()
        .with_utxos(wallets.iter().map(|x| x.to_initial_fund(1000)).collect())
        .with_token(InitialToken {
            token_id: vote_plan.voting_token().clone().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1000)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(NumberOfSlotsPerEpoch::new(60).unwrap())
        .with_certs(vec![vote_plan_cert])
        .with_treasury(1_000.into());

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(block0_builder)
        .build()
        .start_node(temp_dir)
        .unwrap();
    let settings = jormungandr.rest().settings().unwrap();

    let transaction_sender = FragmentSender::new(
        Hash::from_str(&settings.block0_hash).unwrap(),
        settings.fees,
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
