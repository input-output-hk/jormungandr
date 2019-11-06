use crate::common::{
    configuration::genesis_model::Fund,
    data::address::{Account, AddressDataProvider, Utxo},
    jcli_wrapper,
    jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper,
    jormungandr::{
        starter::restart_jormungandr_node, ConfigurationBuilder, JormungandrProcess, Role, Starter,
    },
    startup,
};

use jormungandr_lib::interfaces::{AccountState, SettingsDto, UTxOInfo};

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
    account_receiver: &Account,
    jormungandr: &JormungandrProcess,
    utxo_info: UTxOInfo,
) -> LedgerSnapshot {
    let settings = jcli_wrapper::assert_get_rest_settings(&jormungandr.rest_address());
    let account = jcli_wrapper::assert_rest_account_get_stats(
        &account_receiver.address,
        &jormungandr.rest_address(),
    );
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&jormungandr.rest_address(), &utxo_info);

    LedgerSnapshot::new(settings, utxo_info, account)
}

pub fn do_simple_transaction<T: AddressDataProvider>(
    sender: &T,
    account_receiver: &Account,
    utxo_sender: &UTxOInfo,
    utxo_receiver: &Utxo,
    jormungandr: &JormungandrProcess,
) -> UTxOInfo {
    const TX_VALUE: u64 = 50;
    let config = jormungandr.config();
    let mut tx = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = tx
        .assert_add_input_from_utxo(utxo_sender)
        .assert_add_output(&account_receiver.address, &TX_VALUE.into())
        .assert_add_output(&utxo_receiver.address, &TX_VALUE.into())
        .assert_finalize()
        .seal_with_witness_for_address(sender)
        .assert_to_message();
    let tx_id = tx.get_fragment_id();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr.rest_address());

    UTxOInfo::new(
        tx_id,
        1,
        utxo_receiver.address.parse().unwrap(),
        TX_VALUE.into(),
    )
}

#[test]
pub fn test_node_recovers_from_node_restart() {
    let sender = startup::create_new_utxo_address();
    let account_receiver = startup::create_new_account_address();
    let utxo_receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

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
    let jormungandr = restart_jormungandr_node(jormungandr, Role::Leader);
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr, new_utxo);

    assert_eq!(
        snapshot_before, snapshot_after,
        "Different snaphot after restart {:?} vs {:?}",
        snapshot_before, snapshot_after
    );
}

#[test]
pub fn test_node_recovers_kill_signal() {
    let sender = startup::create_new_utxo_address();
    let account_receiver = startup::create_new_account_address();
    let utxo_receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

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
    let jormungandr = restart_jormungandr_node(jormungandr, Role::Passive);
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr, new_utxo);

    assert_eq!(
        snapshot_before, snapshot_after,
        "Different snaphot after restart {:?} vs {:?}",
        snapshot_before, snapshot_after
    );
}
