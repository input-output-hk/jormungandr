use crate::common::jormungandr::JormungandrProcess;
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use chain_core::property::Fragment;
use jormungandr_lib::interfaces::FragmentsProcessingSummary;
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
            chain_impl_mockchain::block::BlockDate::first().next_epoch(),
            bob.address(),
            100.into(),
        )
        .unwrap();

    let response = jormungandr
        .rest()
        .raw()
        .send_fragment_batch(vec![alice_fragment.clone(), alice_fragment.clone()], false)
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let summary: FragmentsProcessingSummary =
        serde_json::from_str(&response.text().unwrap()).unwrap();
    assert_eq!(summary.accepted, vec![alice_fragment.id()]);
    assert_eq!(summary.rejected, vec![]);
}
