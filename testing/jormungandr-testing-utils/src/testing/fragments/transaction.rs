use super::FragmentBuilderError;
use crate::wallet::Wallet;
use chain_impl_mockchain::{
    fee::LinearFee,
    fragment::Fragment,
    transaction::{InputOutputBuilder, NoExtra, Payload, TxBuilder},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
};

pub fn transaction_to(
    block0_hash: &Hash,
    fees: &LinearFee,
    from: &Wallet,
    address: Address,
    value: Value,
) -> Result<Fragment, FragmentBuilderError> {
    let mut iobuilder = InputOutputBuilder::empty();
    iobuilder.add_output(address.into(), value.into()).unwrap();

    let payload_data = NoExtra.payload_data();
    from.add_input(payload_data.borrow(), &mut iobuilder, fees)?;

    let ios = iobuilder.build();
    let txbuilder = TxBuilder::new()
        .set_nopayload()
        .set_ios(&ios.inputs, &ios.outputs);

    let sign_data = txbuilder.get_auth_data_for_witness().hash();
    let witness = from.mk_witness(block0_hash, &sign_data);
    let witnesses = vec![witness];
    let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
    Ok(Fragment::Transaction(tx))
}
