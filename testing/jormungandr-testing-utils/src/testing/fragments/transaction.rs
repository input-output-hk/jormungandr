use super::FragmentBuilderError;
use crate::wallet::Wallet;
use chain_impl_mockchain::{
    block::BlockDate,
    fee::{FeeAlgorithm, LinearFee},
    fragment::Fragment,
    transaction::{InputOutputBuilder, TxBuilder},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
};

pub fn transaction_to(
    block0_hash: &Hash,
    fees: &LinearFee,
    expiry_date: BlockDate,
    from: &Wallet,
    address: Address,
    value: Value,
) -> Result<Fragment, FragmentBuilderError> {
    transaction_to_many(block0_hash, fees, expiry_date, from, &[address], value)
}

pub fn transaction_to_many(
    block0_hash: &Hash,
    fees: &LinearFee,
    expiry_date: BlockDate,
    from: &Wallet,
    addresses: &[Address],
    value: Value,
) -> Result<Fragment, FragmentBuilderError> {
    let mut iobuilder = InputOutputBuilder::empty();

    for address in addresses {
        iobuilder
            .add_output(address.clone().into(), value.into())
            .unwrap();
    }

    let value_u64: u64 = value.into();
    let input_without_fees: Value = (value_u64 * addresses.len() as u64).into();
    let input_value = fees.calculate(None, 1, addresses.len() as u8) + input_without_fees.into();
    let input = from.add_input_with_value(input_value.unwrap().into());
    iobuilder.add_input(&input).unwrap();

    let ios = iobuilder.build();
    let txbuilder = TxBuilder::new()
        .set_nopayload()
        .set_expiry_date(expiry_date)
        .set_ios(&ios.inputs, &ios.outputs);

    let sign_data = txbuilder.get_auth_data_for_witness().hash();
    let witness = from.mk_witness(block0_hash, &sign_data);
    let witnesses = vec![witness];
    let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
    Ok(Fragment::Transaction(tx))
}
