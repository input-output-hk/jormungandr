use crate::startup;
use chain_core::property::Fragment;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::jormungandr::{ConfigurationBuilder, JormungandrProcess};
use jormungandr_lib::interfaces::FragmentsProcessingSummary;
use rstest::*;
use thor::Wallet;

#[fixture]
fn world() -> (JormungandrProcess, Wallet, Wallet, Wallet) {
    let alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let clarice = thor::Wallet::default();

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
    let (jormungandr, alice, bob, _) = world;

    let alice_fragment = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&alice, bob.address(), 100.into())
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
