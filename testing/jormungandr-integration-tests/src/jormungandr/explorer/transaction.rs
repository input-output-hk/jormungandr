use crate::startup;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::jormungandr::explorer::verifier::ExplorerVerifier;
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::time::Duration;
use thor::TransactionHash;

#[test]
pub fn explorer_transaction_test() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let transaction_value = 1_000;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&sender, receiver.address(), transaction_value.into())
    .unwrap();

    let wait = Wait::new(Duration::from_secs(3), 20);
    let fragment_id = jcli
        .fragment_sender(&jormungandr)
        .send(&transaction.encode())
        .assert_in_block_with_wait(&wait);

    let explorer_transaction = explorer
        .transaction(fragment_id.into())
        .expect("non existing transaction")
        .data
        .unwrap()
        .transaction;

    ExplorerVerifier::assert_transaction(transaction, explorer_transaction).unwrap();
}
