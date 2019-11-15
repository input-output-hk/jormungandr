#![cfg(test)]

use crate::{
    accounting::account::LedgerError::NonExistent,
    ledger::{self, check::TxVerifyError, Error::{TransactionMalformed, Account}},
    testing::{
        ConfigBuilder, LedgerBuilder,
        KeysDb,
        data::AddressData,
        TestTxBuilder,
        TestCryptoGen,
    },
    transaction::*,
    value::*,
};
use chain_addr::Discrimination;

#[test]
pub fn transaction_fail_when_255_outputs() {
    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucet_value(Value(1000))
        .build()
        .expect("cannot build test ledger");

    // random output repeated 255 times.
    let receiver = AddressData::utxo(Discrimination::Test);
    let output = Output { address: receiver.address, value: Value(1) };
    let outputs : Vec<_> = std::iter::repeat(output).take(255).collect();

    let fragment = TestTxBuilder::new(&test_ledger.block0_hash)
        .move_to_outputs_from_faucet(&mut test_ledger, &outputs).get_fragment();

    assert_err!(
        TransactionMalformed {
            source: TxVerifyError::TooManyOutputs {
                expected: 254,
                actual: 255
            }
        },
        test_ledger.apply_transaction(fragment)
    );
}

#[test]
pub fn duplicated_account_transaction() {
    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucet_value(Value(1000))
        .build()
        .expect("cannot build test ledger");

    let receiver = AddressData::utxo(Discrimination::Test);

    let fragment = TestTxBuilder::new(&test_ledger.block0_hash)
        .move_from_faucet(&mut test_ledger, &receiver.address, &Value(100)).get_fragment();
    let fragment2 = fragment.clone();
    let result = test_ledger.apply_transaction(fragment);

    match result {
        Err(err) => panic!("first transaction should be succesful but {}", err),
        Ok(_) => {
            assert_err_match!(
                &ledger::Error::AccountInvalidSignature{..},
                test_ledger.apply_transaction(fragment2)
            );
        }
    }
}

#[test]
pub fn transaction_nonexisting_account_input() {
    let receiver = AddressData::utxo(Discrimination::Test);

    let mut kdb = KeysDb::new(TestCryptoGen(0));

    let mut test_ledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucet_value(Value(1000))
        .build()
        .expect("cannot build test ledger");

    let unregistered_account = kdb.new_account_address();
    let value = Value(10);
    let fragment = TestTxBuilder::new(&test_ledger.block0_hash)
        .inputs_to_outputs(&mut kdb, &mut test_ledger,
            &[Output { address: unregistered_account, value }],
            &[Output { address: receiver.address, value }])
        .get_fragment();

    assert_err!(
        Account { source: NonExistent },
        test_ledger.apply_transaction(fragment)
    );
}
