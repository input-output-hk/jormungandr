use crate::common::address::AddressData;
use crate::common::ledger;
use crate::common::ledger::ConfigBuilder;
use crate::common::tx_builder::TransactionBuilder;
use chain_addr::{Discrimination, Kind, KindType};
use chain_impl_mockchain::transaction::*;
use chain_impl_mockchain::value::*;
use quickcheck::{Arbitrary, Gen, TestResult};
use quickcheck_macros::quickcheck;

#[derive(Clone, Debug)]
pub struct ArbitraryAddressKind(KindType);

impl Arbitrary for ArbitraryAddressKind {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => ArbitraryAddressKind(KindType::Single),
            1 => ArbitraryAddressKind(KindType::Group),
            2 => ArbitraryAddressKind(KindType::Account),
            _ => unreachable!(),
        }
    }
}

#[quickcheck]
pub fn ledger_verifies_faucet_discrimination(
    arbitrary_faucet_disc: Discrimination,
    arbitrary_faucet_address_kind: ArbitraryAddressKind,
    arbitrary_ledger_disc: Discrimination,
) {
    let faucet = AddressData::from_discrimination_and_kind_type(
        arbitrary_faucet_disc,
        &arbitrary_faucet_address_kind.0,
    );

    let (message, _) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(100),
    ));

    let are_discriminations_unified = arbitrary_faucet_disc == arbitrary_ledger_disc;

    let config = ConfigBuilder::new()
        .with_discrimination(arbitrary_ledger_disc)
        .build();
    match (
        are_discriminations_unified,
        ledger::create_initial_fake_ledger(&[message], config),
    ) {
        (true, Ok(_)) => TestResult::passed(),
        (false, Ok(_)) => {
            TestResult::error("Ledger should reject transaction with mixed discriminations")
        }
        (true, Err(_)) => {
            TestResult::error("Ledger should accept transaction with unified discriminations")
        }
        (false, Err(_)) => TestResult::passed(),
    };
}

fn from_address_or_utxo(faucet: &AddressData, value: Value, utxo: UtxoPointer) -> Input {
    match faucet.address.kind() {
        Kind::Account { .. } => {
            Input::from_account_public_key(faucet.public_key.clone(), value.clone())
        }
        _ => Input::from_utxo(utxo),
    }
}

#[quickcheck]
pub fn ledger_verifies_transaction_discrimination(
    arbitrary_input_disc: Discrimination,
    arbitrary_output_disc: Discrimination,
    arbitrary_input_address_kind: ArbitraryAddressKind,
    arbitrary_output_address_kind: ArbitraryAddressKind,
) -> TestResult {
    let faucet = AddressData::from_discrimination_and_kind_type(
        arbitrary_input_disc,
        &arbitrary_input_address_kind.0,
    );
    let receiver = AddressData::from_discrimination_and_kind_type(
        arbitrary_output_disc,
        &arbitrary_output_address_kind.0,
    );
    let value = Value(100);
    let (message, utxos) =
        ledger::create_initial_transaction(Output::from_address(faucet.address.clone(), value));

    let config = ConfigBuilder::new()
        .with_discrimination(arbitrary_input_disc)
        .build();
    let (block0_hash, ledger) = ledger::create_initial_fake_ledger(&[message], config).unwrap();
    let signed_tx = TransactionBuilder::new()
        .with_input(from_address_or_utxo(&faucet, value, utxos[0]))
        .with_output(Output::from_address(receiver.address.clone(), value))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let are_discriminations_unified = arbitrary_input_disc == arbitrary_output_disc;

    let fees = ledger.get_ledger_parameters();
    let actual_result = ledger.apply_transaction(&signed_tx, &fees);

    match (are_discriminations_unified, actual_result) {
        (true, Ok(_)) => TestResult::passed(),
        (false, Ok(_)) => {
            TestResult::error("Ledger should reject transaction with mixed discriminations")
        }
        (true, Err(_)) => {
            TestResult::error("Ledger should accept transaction with unified discriminations")
        }
        (false, Err(_)) => TestResult::passed(),
    }
}
