use std::time::Duration;

use assert_fs::TempDir;
use jormungandr_testing_utils::{
    testing::{FragmentSender, FragmentSenderSetup, FragmentVerifier},
    wallet::Wallet,
};
use rand::seq::SliceRandom;
use rand_core::OsRng;

use crate::common::jormungandr::{ConfigurationBuilder, Starter};

#[test]
fn all_reordered_fragments_are_submitted() {
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let mut alice = Wallet::new_account(&mut rng);
    let mut bob = Wallet::new_account(&mut rng);
    let clarice = Wallet::new_account(&mut rng);

    let initial_fund_per_wallet = 1_000_000;
    let wallets = [&alice, &bob, &clarice];
    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.to_initial_fund(initial_fund_per_wallet))
                .collect(),
        )
        .with_committees(&wallets)
        .with_slots_per_epoch(60)
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();

    let block0_hash = jormungandr.genesis_block_hash();
    let fees = jormungandr.fees();

    let mut fragments = Vec::new();
    for _i in 0..10 {
        for wallet in &mut [&mut alice, &mut bob] {
            let fragment = wallet
                .transaction_to(&block0_hash, &fees, clarice.address(), 100.into())
                .unwrap();
            fragments.push((wallet.clone(), fragment));
            wallet.confirm_transaction();
        }
    }
    fragments.shuffle(&mut rng);

    let fragment_sender = FragmentSender::new(block0_hash, fees, FragmentSenderSetup::no_verify());

    let checks = fragments
        .into_iter()
        .map(|(mut wallet, fragment)| {
            fragment_sender.send_fragment(&mut wallet, fragment, &jormungandr)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let fragment_verifier = FragmentVerifier;
    fragment_verifier
        .wait_and_verify_all_are_in_block(Duration::from_secs(5), checks, &jormungandr)
        .unwrap();
}
