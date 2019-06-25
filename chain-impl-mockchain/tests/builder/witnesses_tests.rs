use crate::common::address::AddressData;
use crate::common::arbitrary::address::*;
use crate::common::ledger;
use crate::common::ledger::ConfigBuilder;
use crate::common::tx_builder::TransactionBuilder;
use chain_addr::Discrimination;
use chain_impl_mockchain::transaction::*;
use chain_impl_mockchain::value::*;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_witnesses_for_inputs(input_addresses: ArbitraryAddressesData) {
    let faucet = AddressData::utxo(Discrimination::Test);
    let value = Value(1000);

    let (message, utxos) =
        ledger::create_initial_transaction(Output::from_address(faucet.address.clone(), value));
    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.as_input(value, utxos[0]))
        .with_outputs(
            input_addresses
                .0
                .iter()
                .map(|x| x.as_output(Value(1)))
                .collect(),
        )
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}
