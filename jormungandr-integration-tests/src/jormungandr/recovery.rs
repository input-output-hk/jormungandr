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

#[derive(Clone, Debug)]
struct LedgerSnapshot {
    settings: SettingsDto,
    utxos: Vec<UTxOInfo>,
    account_state: AccountState,
}

impl PartialEq for LedgerSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.settings == other.settings
            && self.utxos == other.utxos
            && self.account_state == other.account_state
    }
}
impl Eq for LedgerSnapshot {}

impl LedgerSnapshot {
    pub fn new(settings: SettingsDto, utxos: Vec<UTxOInfo>, account_state: AccountState) -> Self {
        LedgerSnapshot {
            settings,
            utxos,
            account_state,
        }
    }
}

fn take_snapshot(account_receiver: &Account, jormungandr: &JormungandrProcess) -> LedgerSnapshot {
    let settings = jcli_wrapper::assert_get_rest_settings(&jormungandr.rest_address());
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr.rest_address());
    let account = jcli_wrapper::assert_rest_account_get_stats(
        &account_receiver.address,
        &jormungandr.rest_address(),
    );

    LedgerSnapshot::new(settings, utxos, account)
}

pub fn do_simple_transaction<T: AddressDataProvider>(
    sender: &T,
    account_receiver: &Account,
    utxo_receiver: &Utxo,
    jormungandr: &JormungandrProcess,
) {
    let config = jormungandr.config();
    let utxo = startup::get_utxo_for_address(sender, &jormungandr.rest_address());

    let transaction_message = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&account_receiver.address, &50.into())
        .assert_add_output(&utxo_receiver.address, &50.into())
        .assert_finalize()
        .seal_with_witness_for_address(sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr.rest_address());
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

    do_simple_transaction(&sender, &account_receiver, &utxo_receiver, &jormungandr);
    let snapshot_before = take_snapshot(&account_receiver, &jormungandr);
    let jormungandr = restart_jormungandr_node(jormungandr, Role::Leader);
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr);

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

    do_simple_transaction(&sender, &account_receiver, &utxo_receiver, &jormungandr);
    let snapshot_before = take_snapshot(&account_receiver, &jormungandr);
    let jormungandr = restart_jormungandr_node(jormungandr, Role::Passive);
    let snapshot_after = take_snapshot(&account_receiver, &jormungandr);

    assert_eq!(
        snapshot_before, snapshot_after,
        "Different snaphot after restart {:?} vs {:?}",
        snapshot_before, snapshot_after
    );
}
