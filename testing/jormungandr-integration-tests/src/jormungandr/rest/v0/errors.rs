use crate::common::jormungandr::JormungandrProcess;
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use jormungandr_testing_utils::testing::node::assert_bad_request;
use jormungandr_testing_utils::wallet::Wallet;
use rstest::*;

#[fixture]
fn world() -> (JormungandrProcess, Wallet, Wallet, Wallet) {
    let alice = startup::create_new_account_address();
    let bob = startup::create_new_account_address();
    let clarice = startup::create_new_account_address();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    (jormungandr, alice, bob, clarice)
}

#[rstest]
pub fn fragment_already_in_log(world: (JormungandrProcess, Wallet, Wallet, Wallet)) {
    let (jormungandr, mut alice, bob, _) = world;

    let alice_fragment = alice
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            bob.address(),
            100.into(),
        )
        .unwrap();

    assert_bad_request(
        jormungandr
            .rest()
            .send_fragment_batch(vec![alice_fragment.clone(), alice_fragment.clone()], false),
    );
}
