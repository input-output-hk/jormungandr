use assert_fs::TempDir;
use chain_addr::Discrimination;
use chain_crypto::Ed25519;
use chain_impl_mockchain::certificate::UpdateProposal;
use chain_impl_mockchain::certificate::UpdateVote;
use jormungandr_lib::interfaces::{
    BlockContentMaxSize, ConfigParam, ConfigParams, ConsensusLeaderId,
};
use jormungandr_testing_utils::testing::startup::create_new_key_pair;
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::testing::FragmentSenderSetup;
use jormungandr_testing_utils::testing::{jormungandr::ConfigurationBuilder, startup};
use jormungandr_testing_utils::testing::{
    jormungandr::Starter,
    node::time::{get_current_date, wait_for_epoch},
};

#[test]
fn proposal_expired_after_proposal_expiration_deadline() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = startup::create_new_account_address();
    let bft_secret = create_new_key_pair::<Ed25519>();
    let wallet_initial_funds = 1_000_000;

    let config = ConfigurationBuilder::new()
        .with_funds(vec![alice.to_initial_fund(wallet_initial_funds)])
        .with_consensus_leaders_ids(vec![bft_secret.identifier().into()])
        .with_proposal_expiry_epochs(2)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let new_block_context_max_size = 1000;
    let change_params = ConfigParams::new(vec![ConfigParam::BlockContentMaxSize(
        BlockContentMaxSize::from(new_block_context_max_size),
    )]);

    let old_settings = jormungandr.rest().settings().unwrap();

    let current_epoch = get_current_date(&mut jormungandr.rest()).epoch();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        jormungandr.default_block_date_generator(),
        FragmentSenderSetup::resend_3_times(),
    );

    let update_proposal = UpdateProposal::new(
        change_params.into(),
        bft_secret.identifier().into_public_key().into(),
    );
    let check = fragment_sender
        .send_update_proposal(
            &mut alice,
            &bft_secret.signing_key().into_secret_key(),
            update_proposal,
            &jormungandr,
        )
        .unwrap();

    wait_for_epoch(current_epoch + 2, jormungandr.rest());

    let update_vote = UpdateVote::new(
        *check.fragment_id(),
        bft_secret.identifier().into_public_key().into(),
    );
    fragment_sender
        .send_update_vote(
            &mut alice,
            &bft_secret.signing_key().into_secret_key(),
            update_vote,
            &jormungandr,
        )
        .unwrap();

    wait_for_epoch(current_epoch + 4, jormungandr.rest());

    let new_settings = jormungandr.rest().settings().unwrap();

    assert_eq!(old_settings, new_settings);
}

#[test]
fn proposal_changes_immutable_setting() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = startup::create_new_account_address();
    let bft_secret = create_new_key_pair::<Ed25519>();
    let wallet_initial_funds = 1_000_000;

    let config = ConfigurationBuilder::new()
        .with_funds(vec![alice.to_initial_fund(wallet_initial_funds)])
        .with_discrimination(Discrimination::Test)
        .with_consensus_leaders_ids(vec![ConsensusLeaderId::from(alice.public_key())])
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let change_params = ConfigParams::new(vec![ConfigParam::Discrimination(
        Discrimination::Production,
    )]);

    let old_settings = jormungandr.rest().settings().unwrap();

    let current_epoch = get_current_date(&mut jormungandr.rest()).epoch();

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        jormungandr.default_block_date_generator(),
        FragmentSenderSetup::resend_3_times(),
    );

    let update_proposal = UpdateProposal::new(change_params.into(), alice.public_key().into());
    let check = fragment_sender
        .send_update_proposal(
            &mut alice,
            &bft_secret.signing_key().into_secret_key(),
            update_proposal,
            &jormungandr,
        )
        .unwrap();
    let update_vote = UpdateVote::new(*check.fragment_id(), alice.public_key().into());
    fragment_sender
        .send_update_vote(
            &mut alice,
            &bft_secret.signing_key().into_secret_key(),
            update_vote,
            &jormungandr,
        )
        .unwrap();

    wait_for_epoch(current_epoch + 2, jormungandr.rest());

    let new_settings = jormungandr.rest().settings().unwrap();

    assert_eq!(old_settings, new_settings);
}
