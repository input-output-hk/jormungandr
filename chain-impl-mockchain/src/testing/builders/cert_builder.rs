use crate::{
    account::Identifier,
    certificate::{
        Certificate, PoolId, PoolManagement, PoolOwnersSigned, PoolRegistration, PoolRetirement,
        StakeDelegation,
    },
    key::EitherEd25519SecretKey,
    testing::data::AddressData,
    transaction::AccountIdentifier,
};
use chain_time::units::DurationSeconds;
use typed_bytes::ByteBuilder;

pub fn build_stake_delegation_cert(
    stake_pool: &PoolRegistration,
    delegate_from: &AddressData,
) -> Certificate {
    let account_id =
        AccountIdentifier::from_single_account(Identifier::from(delegate_from.delegation_key()));
    Certificate::StakeDelegation(StakeDelegation {
        account_id: account_id,
        pool_id: stake_pool.to_id(),
    })
}

pub fn build_stake_pool_registration_cert(stake_pool: &PoolRegistration) -> Certificate {
    Certificate::PoolRegistration(stake_pool.clone())
}

pub fn build_stake_pool_retirement_cert(
    pool_id: PoolId,
    start_validity: u64,
    owners_private_keys: &Vec<EitherEd25519SecretKey>,
) -> Certificate {
    let retirement = PoolRetirement {
        pool_id: pool_id,
        retirement_time: DurationSeconds(start_validity).into(),
    };

    let mut signatures = Vec::new();
    for (i, owner) in owners_private_keys.iter().enumerate() {
        let byte_array = retirement.serialize_in(ByteBuilder::new()).finalize();
        signatures.push((i as u16, owner.sign(&byte_array)));
    }

    Certificate::PoolManagement(PoolManagement::Retirement(PoolOwnersSigned {
        inner: retirement,
        signatures: signatures,
    }))
}
