use crate::{
    account::Identifier,
    certificate::{
        Certificate, PoolManagement, PoolOwnersSigned, PoolRegistration, PoolRetirement,
        StakeDelegation,
    },
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
    stake_pool: PoolRegistration,
    owners: &[AddressData],
) -> Certificate {
    let retirement = PoolRetirement {
        pool_id: stake_pool.to_id(),
        retirement_time: DurationSeconds(0).into(),
    };

    let mut signatures = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let byte_array = retirement.serialize_in(ByteBuilder::new()).finalize();
        signatures.push((i as u16, owner.private_key().sign(&byte_array)));
    }

    Certificate::PoolManagement(PoolManagement::Retirement(PoolOwnersSigned {
        inner: retirement,
        signatures: signatures,
    }))
}
