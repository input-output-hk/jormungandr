use crate::startup;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
        ConfigurationBuilder,
    },
};
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
    let query_complexity_limit = 140;
    let attempts_number = 20;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let params = ExplorerParams::new(query_complexity_limit, None, None);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&sender, receiver.address(), transaction_value.into())
    .unwrap();

    let wait = Wait::new(Duration::from_secs(3), attempts_number);

    let fragment_id = jcli
        .fragment_sender(&jormungandr)
        .send(&transaction.encode())
        .assert_in_block_with_wait(&wait);

    let explorer_transaction = explorer
        .transaction_certificates(fragment_id.into())
        .expect("Non existing transaction")
        .data
        .unwrap()
        .transaction;

    ExplorerVerifier::assert_transaction_certificates(transaction, explorer_transaction).unwrap();
}
