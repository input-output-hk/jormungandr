use crate::startup;
use assert_fs::TempDir;
use chain_addr::Discrimination;
use chain_core::property::BlockDate as propertyBlockDate;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{UpdateProposal, UpdateVote, VoteAction, VoteTallyPayload},
    fee::LinearFee,
    tokens::minting_policy::MintingPolicy,
    transaction::AccountIdentifier,
    vote::Choice,
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifier::ExplorerVerifier},
        ConfigurationBuilder, Starter,
    },
    testing::{
        keys::create_new_key_pair,
        time::{wait_for_date, wait_for_epoch},
        VotePlanBuilder,
    },
};
use jormungandr_lib::interfaces::InitialToken;
use thor::{
    BlockDateGenerator::Fixed, FragmentBuilder, FragmentSender, StakePool, TransactionHash, FragmentSenderSetup, vote_plan_cert, Wallet,
};
use rstest::*;

const VOTE_PLAN_QUERY_COMPLEXITY_LIMIT: u64 = 50;
const VOTE_PLAN_QUERY_DEPTH_LIMIT: u64 = 30;



#[test]
pub fn explorer_vote_plan_test() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
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
        .with_token(InitialToken {
            token_id: vote_plan.voting_token().clone().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1000)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
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

    let params = ExplorerParams::new(
            VOTE_PLAN_QUERY_COMPLEXITY_LIMIT,
            VOTE_PLAN_QUERY_DEPTH_LIMIT,
            None,
        );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let trans = explorer
        .vote_plan(vote_plan.to_id().to_string())
        .expect("vote plan transaction not found");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let vote_plan_transaction = trans.data.unwrap().vote_plan;

}