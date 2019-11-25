use crate::{
    account::SpendingCounter,
    header::HeaderId,
    key::EitherEd25519SecretKey,
    testing::data::AddressData,
    transaction::{TransactionSignDataHash, Witness},
};
use chain_addr::Kind;


pub fn make_witnesses(block0: &HeaderId,
    addresses_data: Vec<&AddressData>,
    transaction_hash: &TransactionSignDataHash) -> Vec<Witness> {
    addresses_data.iter().map(|x| make_witness(block0,x,transaction_hash)).collect()
}

pub fn make_witness(
    block0: &HeaderId,
    addres_data: &AddressData,
    transaction_hash: &TransactionSignDataHash,
) -> Witness {
    match addres_data.address.kind() {
        Kind::Account(_) => self::make_account_witness(
            block0,
            &addres_data.spending_counter.unwrap(),
            &addres_data.private_key(),
            transaction_hash,
        ),
        _ => self::make_utxo_witness(block0, &addres_data.private_key(), &transaction_hash),
    }
}

pub fn make_utxo_witness(
    block0: &HeaderId,
    secret_key: &EitherEd25519SecretKey,
    transaction_hash: &TransactionSignDataHash,
) -> Witness {
    Witness::new_utxo(block0, transaction_hash, secret_key)
}

pub fn make_account_witness(
    block0: &HeaderId,
    spending_counter: &SpendingCounter,
    secret_key: &EitherEd25519SecretKey,
    transaction_hash: &TransactionSignDataHash,
) -> Witness {
    Witness::new_account(block0, transaction_hash, spending_counter, secret_key)
}
