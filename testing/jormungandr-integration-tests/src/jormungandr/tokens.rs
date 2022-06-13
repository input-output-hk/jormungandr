use assert_fs::TempDir;
use chain_impl_mockchain::{
    fee::LinearFee,
    testing::TestGen,
    tokens::{identifier::TokenIdentifier, minting_policy::MintingPolicy},
    value::Value,
};
use jormungandr_automation::jormungandr::{ConfigurationBuilder, Starter};
use jormungandr_lib::interfaces::InitialToken;
use thor::{FragmentSender, FragmentVerifier};

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

    let config = ConfigurationBuilder::new()
        .with_fund(alice.to_initial_fund(1_000))
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(initial_token_value)],
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
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
pub fn can_assign_token_to_non_existing_account() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();

    let minting_policy = MintingPolicy::new();
    let token_id = TokenIdentifier {
        policy_hash: minting_policy.hash(),
        token_name: TestGen::token_name(),
    };

    ConfigurationBuilder::new()
        .with_token(InitialToken {
            token_id: token_id.into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(1_000)],
        })
        .build(&temp_dir);
}

#[test]
#[should_panic]
pub fn setup_wrong_policy_hash() {
    let temp_dir = TempDir::new().unwrap();
    let alice = thor::Wallet::default();

    ConfigurationBuilder::new()
        .with_fund(alice.to_initial_fund(1_000))
        .with_token(InitialToken {
            token_id: TestGen::token_id().into(),
            policy: MintingPolicy::new().into(),
            to: vec![alice.to_initial_token(1_000)],
        })
        .build(&temp_dir);
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

    let config = ConfigurationBuilder::new()
        .with_fund(alice.to_initial_fund(1_000))
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(0)],
        })
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
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

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            alice.to_initial_fund(1_000),
            bob.to_initial_fund(1_000),
        ])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(initial_token_value)],
        })
        .with_linear_fees(LinearFee::new(1, 1, 1))
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let fragment_sender = FragmentSender::from(jormungandr.block0_configuration());

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
