use crate::startup;
use assert_fs::TempDir;
use chain_impl_mockchain::{
    block::BlockDate,
    fragment::Fragment,
    transaction::{NoExtra, Transaction},
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifier::ExplorerVerifier},
        ConfigurationBuilder, Starter,
    },
};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, FragmentLog};
use jortestkit::process::Wait;
use std::time::Duration;
use thor::TransactionHash;

#[test]
pub fn explorer_address_test() {
    let sender = thor::Wallet::default();
    let address_bech32_prefix = sender.address().0;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let params = ExplorerParams::new(None, None, address_bech32_prefix);
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let explorer_address = explorer.address(sender.address().to_string()).unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    ExplorerVerifier::assert_address(sender.address(), explorer_address.data.unwrap().address);
}

#[test]
pub fn explorer_transactions_not_existing_address_test() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let test_address = thor::Wallet::default();
    let transaction_value = 1_000;
    let attempts_number = 20;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[sender.clone()], &mut config).unwrap();

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&sender, receiver.address(), transaction_value.into())
    .unwrap();

    let wait = Wait::new(Duration::from_secs(3), attempts_number);

    let _fragment_id = jcli
        .fragment_sender(&jormungandr)
        .send(&transaction.encode())
        .assert_in_block_with_wait(&wait);

    let explorer_process = jormungandr.explorer(ExplorerParams::default());
    let explorer = explorer_process.client();

    let explorer_address = explorer
        .transactions_address(test_address.address().to_string())
        .unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );
    let explorer_transactions_by_address =
        explorer_address.data.unwrap().tip.transactions_by_address;

    ExplorerVerifier::assert_transactions_address(Vec::new(), explorer_transactions_by_address);
}

#[test] //BUG NPG-2869
pub fn explorer_transactions_address_test() {
    let jcli: JCli = Default::default();
    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let transaction1_value = 1_000;
    let transaction2_value = 2_0;
    let transaction3_value = 3_0;
    let attempts_number = 20;
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![sender.to_initial_fund(1_000_000)])
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

    let wait = Wait::new(Duration::from_secs(3), attempts_number);
    /*
        let transaction_1 = thor::FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
        )
        .transaction(&sender, receiver.address(), transaction1_value.into())
        .unwrap();

        let fragment_id_1 = jcli
            .fragment_sender(&jormungandr)
            .send(&transaction_1.encode())
            .assert_in_block_with_wait(&wait);

        sender.confirm_transaction();

        let transaction_2 = thor::FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
        )
        .transaction(&sender, receiver.address(), transaction2_value.into())
        .unwrap();

         let fragment_id2 = jcli
            .fragment_sender(&jormungandr)
            .send(&transaction_2.encode())
            .assert_in_block_with_wait(&wait);

        let transaction_3 = thor::FragmentBuilder::new(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
        )
        .transaction(&receiver, sender.address(), transaction3_value.into())
        .unwrap();

        let fragment_id_3 = jcli
            .fragment_sender(&jormungandr)
            .send(&transaction_3.encode())
            .assert_in_block_with_wait(&wait);

        let fragments = vec![transaction_1, transaction_2, transaction_3];

        let fragments_log = jcli.rest().v0().message().logs(jormungandr.rest_uri());

        let f = fragments_log
            .iter()
            .zip(fragments.iter())
            .filter(|&(a, b)| a.fragment_id().to_string() == b.hash().to_string())
            .collect::<Vec<_>>();

        println!("fragment log {:?}",fragments_log.len());
        for x in f.iter(){
            println!("FRAA {:?} {:?}",x.0.fragment_id().to_string(),x.1.hash().to_string());
        }
    */
    let explorer_process = jormungandr.explorer(ExplorerParams::default());
    let explorer = explorer_process.client();

    let explorer_address = explorer
        .transactions_address(sender.address().to_string())
        .unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    let explorer_transactions_by_address =
        explorer_address.data.unwrap().tip.transactions_by_address;
    println!(
        "EDGES {:?}",
        explorer_transactions_by_address.edges.unwrap().len()
    );

    //ExplorerVerifier::assert_transactions_address(vec![transaction_1,transaction_2,transaction_3], explorer_transactions_by_address);
}
