use crate::{startup, startup::SingleNodeTestBootstrapper};
use assert_fs::TempDir;
use chain_impl_mockchain::{
    fee::LinearFee,
    testing::TestGen,
    tokens::{identifier::TokenIdentifier, minting_policy::MintingPolicy},
    value::Value,
};
use jormungandr_automation::jormungandr::{
    Block0ConfigurationBuilder, JormungandrBootstrapper, SecretModelFactory,
};
use jormungandr_lib::interfaces::{ConsensusLeaderId, InitialToken};
use thor::{Block0ConfigurationBuilderExtension, FragmentSender, FragmentVerifier};

#[test]
pub fn rest_shows_initial_token_state() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();

    let initial_token_value = 1_000;

    let minting_policy = MintingPolicy::new();
    let token_id = TokenIdentifier {
        policy_hash: minting_policy.hash(),
        token_name: TestGen::token_name(),
    };

    let config = Block0ConfigurationBuilder::default()
        .with_utxo(alice.to_initial_fund(1_000))
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(initial_token_value)],
        });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let alice_account_state = jormungandr
        .rest()
        .account_state(&alice.account_id())
        .unwrap();
    assert_eq!(
        alice_account_state.tokens()[&token_id.into()],
        Value(initial_token_value).into()
    );
}

#[test]
pub fn cannot_assign_token_to_non_existing_account() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();
    let leader_key = startup::create_new_leader_key();
    let minting_policy = MintingPolicy::new();
    let token_id = TokenIdentifier {
        policy_hash: minting_policy.hash(),
        token_name: TestGen::token_name(),
    };

    let block0 = Block0ConfigurationBuilder::default()
        .with_consensus_leaders_ids(vec![ConsensusLeaderId::from(leader_key.identifier())])
        .with_token(InitialToken {
            token_id: token_id.into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(1_000)],
        })
        .build();

    JormungandrBootstrapper::default()
        .with_block0_configuration(block0)
        .with_secret(SecretModelFactory::bft(leader_key.signing_key()))
        .into_starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message("Account does not exist")
        .unwrap();
}

#[test]
#[should_panic]
pub fn setup_wrong_policy_hash() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();

    let block0_config = Block0ConfigurationBuilder::default()
        .with_utxo(alice.to_initial_fund(1_000))
        .with_token(InitialToken {
            token_id: TestGen::token_id().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1_000)],
        })
        .build();

    JormungandrBootstrapper::default()
        .with_block0_configuration(block0_config)
        .into_starter(temp_dir)
        .unwrap()
        .start()
        .unwrap();
}

#[test]
pub fn setup_0_token_assigned() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();

    let minting_policy = MintingPolicy::new();
    let token_id = TokenIdentifier {
        policy_hash: minting_policy.hash(),
        token_name: TestGen::token_name(),
    };

    let config = Block0ConfigurationBuilder::default()
        .with_wallet(&alice, 1_000.into())
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(0)],
        });

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(config)
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let alice_account_state = jormungandr
        .rest()
        .account_state(&alice.account_id())
        .unwrap();
    assert_eq!(
        alice_account_state.tokens()[&token_id.into()],
        Value(0).into()
    );
}

#[test]
pub fn transaction_does_not_influence_token_count() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();

    let initial_token_value = 1_000;

    let minting_policy = MintingPolicy::new();
    let token_id = TokenIdentifier {
        policy_hash: minting_policy.hash(),
        token_name: TestGen::token_name(),
    };

    let config = Block0ConfigurationBuilder::default()
        .with_utxos(vec![
            alice.to_initial_fund(1_000),
            bob.to_initial_fund(1_000),
        ])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(initial_token_value)],
        })
        .with_linear_fees(LinearFee::new(1, 1, 1));

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::try_from(&jormungandr).unwrap();

    let check = fragment_sender
        .send_transaction(&mut alice, &bob, &jormungandr, 10.into())
        .unwrap();

    FragmentVerifier::wait_and_verify_is_in_block(
        std::time::Duration::from_secs(15),
        check,
        &jormungandr,
    )
    .unwrap();

    let alice_account_state = jormungandr
        .rest()
        .account_state(&alice.account_id())
        .unwrap();
    assert_eq!(
        alice_account_state.tokens()[&token_id.into()],
        Value(initial_token_value).into()
    );
}
