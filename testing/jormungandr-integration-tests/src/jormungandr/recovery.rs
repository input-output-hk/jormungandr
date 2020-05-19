use crate::common::{
    jcli_wrapper,
    jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Role, Starter},
    startup,
};

use jormungandr_lib::interfaces::{AccountState, InitialUTxO, SettingsDto, UTxOInfo};
use jormungandr_testing_utils::wallet::Wallet;

use assert_fs::prelude::*;
use assert_fs::TempDir;

#[derive(Clone, Debug, PartialEq)]
struct LedgerSnapshot {
    settings: SettingsDto,
    utxo_info: UTxOInfo,
    account_state: AccountState,
}

impl LedgerSnapshot {
    pub fn new(settings: SettingsDto, utxo_info: UTxOInfo, account_state: AccountState) -> Self {
        LedgerSnapshot {
            settings,
            utxo_info,
            account_state,
        }
    }
}

fn take_snapshot(
    account_receiver: &Wallet,
    jormungandr: &JormungandrProcess,
    utxo_info: UTxOInfo,
) -> LedgerSnapshot {
    let rest_uri = jormungandr.rest_uri();
    let settings = jcli_wrapper::assert_get_rest_settings(&rest_uri);
    let account = jcli_wrapper::assert_rest_account_get_stats(
        &account_receiver.address().to_string(),
        &rest_uri,
    );
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&rest_uri, &utxo_info);

    LedgerSnapshot::new(settings, utxo_info, account)
}

pub fn do_simple_transaction(
    sender: &Wallet,
    account_receiver: &Wallet,
    utxo_sender: &UTxOInfo,
    utxo_receiver: &Wallet,
    jormungandr: &JormungandrProcess,
) -> UTxOInfo {
    const TX_VALUE: u64 = 50;
    let mut tx =
        JCLITransactionWrapper::new_transaction(&jormungandr.genesis_block_hash().to_string());
    let transaction_message = tx
        .assert_add_input_from_utxo(utxo_sender)
        .assert_add_output(&account_receiver.address().to_string(), TX_VALUE.into())
        .assert_add_output(&utxo_receiver.address().to_string(), TX_VALUE.into())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();
    let tx_id = tx.get_fragment_id();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);

    UTxOInfo::new(tx_id, 1, utxo_receiver.address(), TX_VALUE.into())
}

#[test]
pub fn test_node_recovers_from_node_restart() {
    let temp_dir = TempDir::new().unwrap();

    let sender = startup::create_new_utxo_address();
    let account_receiver = startup::create_new_account_address();
    let utxo_receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .with_storage(&temp_dir.child("storage"))
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo_sender = config.block0_utxo_for_address(&sender);

    let new_utxo = do_simple_transaction(
        &sender,
        &account_receiver,
        &utxo_sender,
        &utxo_receiver,
        &jormungandr,
    );
    let snapshot_before = take_snapshot(&account_receiver, &jormungandr, new_utxo.clone());
    jormungandr.stop();
    let jormungandr = Starter::new()
        .config(config)
        .role(Role::Leader)
        .start()
        .unwrap();
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr, new_utxo);

    assert_eq!(
        snapshot_before, snapshot_after,
        "Different snaphot after restart {:?} vs {:?}",
        snapshot_before, snapshot_after
    );
}

#[test]
pub fn test_node_recovers_kill_signal() {
    let temp_dir = TempDir::new().unwrap();

    let sender = startup::create_new_utxo_address();
    let account_receiver = startup::create_new_account_address();
    let utxo_receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .with_storage(&temp_dir.child("storage"))
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo_sender = config.block0_utxo_for_address(&sender);

    let new_utxo = do_simple_transaction(
        &sender,
        &account_receiver,
        &utxo_sender,
        &utxo_receiver,
        &jormungandr,
    );
    let snapshot_before = take_snapshot(&account_receiver, &jormungandr, new_utxo.clone());
    jormungandr.stop();
    let jormungandr = Starter::new()
        .config(config)
        .role(Role::Passive)
        .start()
        .unwrap();
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr, new_utxo);

    assert_eq!(
        snapshot_before, snapshot_after,
        "Different snaphot after restart {:?} vs {:?}",
        snapshot_before, snapshot_after
    );
}
