use assert_fs::TempDir;
use chain_core::property::BlockDate as propertyBlockDate;
use chain_impl_mockchain::{block::BlockDate, tokens::minting_policy::MintingPolicy, vote::Choice};
use jormungandr_automation::{
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifier::ExplorerVerifier},
        ConfigurationBuilder, Starter,
    },
    testing::{
        time::{get_current_date, wait_for_date},
        VotePlanBuilder,
    },
};
use jormungandr_lib::interfaces::InitialToken;
use thor::{vote_plan_cert, FragmentSender, FragmentSenderSetup, Wallet};

const VOTE_PLAN_QUERY_COMPLEXITY_LIMIT: u64 = 50;
const VOTE_PLAN_QUERY_DEPTH_LIMIT: u64 = 30;

#[test]
pub fn explorer_vote_plan_flow_test() {
    let temp_dir = TempDir::new().unwrap();
    let alice = Wallet::default();
    let bob = Wallet::default();
    let mut wallets = vec![alice, bob];
    let proposal_count = 1;

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(proposal_count)
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(1, 10))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(
        &wallets[0],
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        &vote_plan,
    )
    .into();

    let config = ConfigurationBuilder::new()
        .with_funds(wallets.iter().map(|x| x.to_initial_fund(1000)).collect())
        .with_token(InitialToken {
            token_id: vote_plan.voting_token().clone().into(),
            policy: MintingPolicy::new().into(),
            to: vec![
                wallets[0].to_initial_token(1000),
                wallets[1].to_initial_token(1000),
            ],
        })
        .with_committees(&[wallets[0].to_committee_id()])
        .with_slots_per_epoch(20)
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
            epoch: 3,
            slot_id: 0,
        }
        .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    let params = ExplorerParams::new(
        VOTE_PLAN_QUERY_COMPLEXITY_LIMIT,
        VOTE_PLAN_QUERY_DEPTH_LIMIT,
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    //1.Vote plan started
    let query_response = explorer
        .vote_plan(vote_plan.to_id().to_string())
        .expect("vote plan transaction not found");

    assert!(
        query_response.errors.is_none(),
        "{:?}",
        query_response.errors.unwrap()
    );

    let vote_plan_transaction = query_response.data.unwrap().vote_plan.proposals;

    for proposal in vote_plan_transaction {
        println!(
            "VOTES proposal {} {:?}",
            proposal.proposal_id, proposal.votes.total_count
        );
    }
    println!("{:#?}", vote_plan);

    assert!(vote_plan.can_vote(get_current_date(&mut jormungandr.rest()).into()));

    //2. Voting
    transaction_sender
        .send_vote_cast(
            &mut wallets[0],
            &vote_plan,
            0,
            &Choice::new(1),
            &jormungandr,
        )
        .unwrap();
    transaction_sender
        .send_vote_cast(
            &mut wallets[1],
            &vote_plan,
            0,
            &Choice::new(1),
            &jormungandr,
        )
        .unwrap();
    // transaction_sender
    //     .send_vote_cast(&mut wallets[1], &vote_plan, 2, &Choice::new(1), &jormungandr)
    //     .unwrap();

    let query_response = explorer
        .vote_plan(vote_plan.to_id().to_string())
        .expect("vote plan transaction not found");

    assert!(
        query_response.errors.is_none(),
        "{:?}",
        query_response.errors.unwrap()
    );

    let vote_plan_transaction = query_response.data.unwrap().vote_plan.proposals;
    for proposal in vote_plan_transaction {
        println!(
            "VOTES proposal {} {:?}",
            proposal.proposal_id, proposal.votes.total_count
        );
    }
    wait_for_date(vote_plan.vote_end().into(), jormungandr.rest());

    //3.Start talling
    transaction_sender
        .send_public_vote_tally(&mut wallets[0], &vote_plan, &jormungandr)
        .unwrap();
    let query_response = explorer
        .vote_plan(vote_plan.to_id().to_string())
        .expect("vote plan transaction not found");

    assert!(
        query_response.errors.is_none(),
        "{:?}",
        query_response.errors.unwrap()
    );

    let vote_plan_transaction = query_response.data.unwrap().vote_plan.proposals;
    for proposal in vote_plan_transaction {
        println!(
            "VOTES proposal {} tally {:?}",
            proposal.proposal_id,
            proposal.tally.unwrap()
        );
    }

    wait_for_date(vote_plan.committee_end().into(), jormungandr.rest());

    //4. End talling
    let query_response = explorer
        .vote_plan(vote_plan.to_id().to_string())
        .expect("vote plan transaction not found");

    assert!(
        query_response.errors.is_none(),
        "{:?}",
        query_response.errors.unwrap()
    );

    let vote_plan_transaction = query_response.data.unwrap().vote_plan.proposals;
    for proposal in vote_plan_transaction {
        println!(
            "VOTES proposal {} tally {:?}",
            proposal.proposal_id,
            proposal.tally.unwrap()
        );
    }

    println!("{:?}", jormungandr.rest().vote_plan_statuses().unwrap());
}
