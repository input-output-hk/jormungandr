use crate::startup;
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::fragment::FragmentId;
use chain_impl_mockchain::key::Hash;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::str::FromStr;
use std::time::Duration;
use thor::{StakePool, TransactionHash};

#[test]
pub fn explorer_sanity_test() {
    let jcli: JCli = Default::default();
    let faucet = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, initial_stake_pools) =
        startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&faucet, receiver.address(), 1_000.into())
    .unwrap()
    .encode();

    let wait = Wait::new(Duration::from_secs(3), 20);
    let fragment_id = jcli
        .fragment_sender(&jormungandr)
        .send(&transaction)
        .assert_in_block_with_wait(&wait);

    transaction_by_id(explorer, fragment_id);

}

fn transaction_by_id(explorer: &Explorer, fragment_id: FragmentId) {
    let explorer_transaction = explorer
        .transaction(fragment_id.into())
        .expect("non existing transaction");

    assert_eq!(
        fragment_id,
        Hash::from_str(&explorer_transaction.data.unwrap().transaction.id).unwrap(),
        "incorrect fragment id"
    );
}