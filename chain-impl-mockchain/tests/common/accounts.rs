use chain_addr::{Address, Discrimination, Kind};
use chain_impl_mockchain::key::{SpendingPublicKey, SpendingSecretKey};
use rand::{CryptoRng, RngCore};

pub fn make_utxo_key<R: RngCore + CryptoRng>(
    rng: &mut R,
    discrimination: &Discrimination,
) -> (SpendingSecretKey, SpendingPublicKey, Address) {
    let sk = SpendingSecretKey::generate(rng);
    let pk = sk.to_public();
    let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
    (sk, pk, user_address)
}

pub fn make_account_key<R: RngCore + CryptoRng>(
    rng: &mut R,
    discrimination: &Discrimination,
) -> (SpendingSecretKey, SpendingPublicKey, Address) {
    let sk = SpendingSecretKey::generate(rng);
    let pk = sk.to_public();
    let user_address = Address(discrimination.clone(), Kind::Account(pk.clone()));
    (sk, pk, user_address)
}

pub fn make_utxo_delegation_key<R: RngCore + CryptoRng>(
    rng_single: &mut R,
    rng_delegation: &mut R,
    discrimination: &Discrimination,
) -> (SpendingSecretKey, SpendingPublicKey, Address) {
    let single_sk = SpendingSecretKey::generate(rng_single);
    let single_pk = single_sk.to_public();

    let delegation_sk = SpendingSecretKey::generate(rng_delegation);
    let delegation_pk = delegation_sk.to_public();

    let user_address = Address(
        discrimination.clone(),
        Kind::Group(single_pk.clone(), delegation_pk.clone()),
    );
    (single_sk, single_pk, user_address)
}
