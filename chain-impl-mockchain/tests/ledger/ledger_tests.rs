use crate::common::accounts;
use crate::common::tx_builder::TransactionBuilder;
use chain_addr::{Address, Discrimination};
use chain_crypto::SecretKey;
use chain_impl_mockchain::account::Identifier;
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::block::ConsensusVersion;
use chain_impl_mockchain::block::HeaderHash;
use chain_impl_mockchain::config::ConfigParam;
use chain_impl_mockchain::ledger::Error;
use chain_impl_mockchain::ledger::Ledger;
use chain_impl_mockchain::message::Message;
use chain_impl_mockchain::milli::Milli;
use chain_impl_mockchain::transaction::*;
use chain_impl_mockchain::value::*;
use std::vec::Vec;

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

// create an initial fake ledger with the non-optional parameter setup
pub fn create_initial_fake_ledger(
    discrimination: Discrimination,
    initial_msgs: &[Message],
) -> (HeaderHash, Ledger) {
    let block0_hash = HeaderHash::hash_bytes(&[1, 2, 3]);

    let mut ie = chain_impl_mockchain::message::config::ConfigParams::new();
    ie.push(ConfigParam::Discrimination(discrimination));
    ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));

    // TODO remove rng: make this creation deterministic
    let leader_pub_key = SecretKey::generate(rand::thread_rng()).to_public();
    ie.push(ConfigParam::AddBftLeader(leader_pub_key.into()));
    ie.push(ConfigParam::Block0Date(
        chain_impl_mockchain::config::Block0Date(0),
    ));
    ie.push(ConfigParam::SlotDuration(10));
    ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
        Milli::HALF,
    ));
    ie.push(ConfigParam::SlotsPerEpoch(21600));
    ie.push(ConfigParam::KESUpdateSpeed(3600 * 12));
    ie.push(ConfigParam::AllowAccountCreation(true));

    let mut messages = Vec::new();
    messages.push(Message::Initial(ie));
    messages.extend_from_slice(initial_msgs);

    let ledger = Ledger::new(block0_hash, &messages).expect("create initial fake ledger failed");

    (block0_hash, ledger)
}

pub fn create_initial_transaction(output: Output<Address>) -> (Message, Vec<UtxoPointer>) {
    let mut builder = TransactionBuilder::new();
    builder.with_output(output).finalize();
    (builder.as_message(), builder.as_utxos())
}

#[test]
pub fn utxo_no_enough_signatures() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (_, _, user1_address) = accounts::make_utxo_key(&mut rng, &discrimination);
    let (_, _, user2_address) = accounts::make_utxo_key(&mut rng, &discrimination);

    let (message, utxos) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (_, ledger) = create_initial_fake_ledger(discrimination, &[message]);

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
    assert_err!(Error::NotEnoughSignatures(1, 0), r)
}

#[test]
pub fn utxo_to_utxo_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (sk1, _pk1, user1_address) = accounts::make_utxo_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_utxo_key(&mut rng, &discrimination);

    let (message, utxos) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });
    let (block0_hash, ledger) = create_initial_fake_ledger(discrimination, &[message]);

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
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

    let (message, utxos) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) = create_initial_fake_ledger(discrimination, &[message]);

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
        })
        .finalize()
        .with_utxo_witness(&block0_hash, &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok());
}

#[test]
pub fn account_to_account_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let (sk1, pk1, user1_address) = accounts::make_account_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) = accounts::make_account_key(&mut rng, &discrimination);

    let (message, _) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) = create_initial_fake_ledger(discrimination, &[message]);

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
pub fn account_to_delegation_correct_transaction() {
    let discrimination = Discrimination::Test;

    let mut rng = rand::thread_rng();
    let mut delegation_rng = rand::thread_rng();
    let (sk1, pk1, user1_address) = accounts::make_account_key(&mut rng, &discrimination);
    let (_sk2, _pk2, user2_address) =
        accounts::make_utxo_delegation_key(&mut rng, &mut delegation_rng, &discrimination);

    let (message, _) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) = create_initial_fake_ledger(discrimination, &[message]);
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

    let (message, utxos) = create_initial_transaction(Output {
        address: user1_address.clone(),
        value: Value(42000),
    });

    let (block0_hash, ledger) = create_initial_fake_ledger(discrimination, &[message]);

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output {
            address: user2_address.clone(),
            value: Value(1),
        })
        .finalize()
        .with_utxo_witness(&block0_hash, &sk1)
        .seal();

    let dyn_params = ledger.get_ledger_parameters();
    let r = ledger.apply_transaction(&signed_tx, &dyn_params);
    assert!(r.is_ok());
}
