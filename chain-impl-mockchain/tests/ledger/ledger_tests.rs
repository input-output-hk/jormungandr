use crate::common::accounts;
use crate::common::ledger;
use crate::common::ledger::ConfigBuilder;
use crate::common::tx_builder::TransactionBuilder;
use chain_addr::Discrimination;

use chain_impl_mockchain::account::Identifier;
use chain_impl_mockchain::account::SpendingCounter;

use chain_impl_mockchain::ledger::Error::NotEnoughSignatures;
use chain_impl_mockchain::transaction::*;
use chain_impl_mockchain::value::*;

macro_rules! assert_err {
    ($left: expr, $right: expr) => {
        match &($left) {
            left_val => match &($right) {
                Err(e) => {
                    if !(e == left_val) {
                        panic!(
                            "assertion failed: error mismatch \
                             (left: `{:?}, right: `{:?}`)",
                            *left_val, *e
                        )
                    }
                }
                Ok(_) => panic!(
                    "assertion failed: expected error {:?} but got success",
                    *left_val
                ),
            },
        }
    };
}

#[test]
pub fn utxo_no_enough_signatures() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (_, _, user1_address) = accounts::make_utxo_key(&mut rng, &discrimination);
    let (_, _, user2_address) = accounts::make_utxo_key(&mut rng, &discrimination);

    let (message, utxos) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (_, ledger) = ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
        })
        .finalize()
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert_err!(
        NotEnoughSignatures {
            actual: 0,
            expected: 1
        },
        r
    )
}

#[test]
pub fn utxo_to_utxo_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (sk1, _pk1, user1_address) = accounts::make_utxo_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_utxo_key(&mut rng, &discrimination);

    let (message, utxos) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });
    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(42000),
        })
        .finalize()
        .with_utxo_witness(&block0_hash, &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok())
}

#[test]
pub fn utxo_to_account_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (sk1, _pk1, user1_address) = accounts::make_utxo_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_account_key(&mut rng, &discrimination);

    let (message, utxos) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(42000),
        })
        .finalize()
        .with_utxo_witness(&block0_hash, &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok())
}

#[test]
pub fn account_to_account_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (sk1, pk1, user1_address) = accounts::make_account_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_account_key(&mut rng, &discrimination);

    let (message, _) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account(
            AccountIdentifier::from_single_account(Identifier::from(pk1)),
            Value(1),
        ))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
        })
        .finalize()
        .with_account_witness(&block0_hash, &SpendingCounter::zero(), &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok())
}

#[test]
pub fn account_to_delegation_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let mut delegation_rng = rand::thread_rng();
    let (sk1, pk1, user1_address) = accounts::make_account_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) =
        accounts::make_utxo_delegation_key(&mut rng, &mut delegation_rng, &discrimination);

    let (message, _) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account(
            AccountIdentifier::from_single_account(Identifier::from(pk1)),
            Value(1),
        ))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
        })
        .finalize()
        .with_account_witness(&block0_hash, &SpendingCounter::zero(), &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok());
}

#[test]
pub fn delegation_to_account_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let mut delegation_rng = rand::thread_rng();
    let (sk1, _pk1, user1_address) =
        accounts::make_utxo_delegation_key(&mut rng, &mut delegation_rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_account_key(&mut rng, &discrimination);

    let (message, utxos) = ledger::create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(42000),
        })
        .finalize()
        .with_utxo_witness(&block0_hash, &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok())
}
