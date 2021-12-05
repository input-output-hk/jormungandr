use crate::{stake_pool::StakePool, wallet::Wallet};
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{
        PoolId, PoolOwnersSigned, PoolSignature, SignedCertificate, StakeDelegation, VotePlan,
        VotePlanProof,
    },
    transaction::{AccountBindingSignature, SingleAccountBindingSignature, TxBuilder},
};

pub fn signed_delegation_cert(
    wallet: &Wallet,
    valid_until: BlockDate,
    pool_id: PoolId,
) -> SignedCertificate {
    let stake_delegation = StakeDelegation {
        account_id: wallet.stake_key().unwrap(),
        delegation: chain_impl_mockchain::account::DelegationType::Full(pool_id),
    };
    let txb = TxBuilder::new()
        .set_payload(&stake_delegation)
        .set_expiry_date(valid_until)
        .set_ios(&[], &[])
        .set_witnesses(&[]);
    let auth_data = txb.get_auth_data();

    let sig = AccountBindingSignature::new_single(&auth_data, |d| wallet.sign_slice(d.0));
    SignedCertificate::StakeDelegation(stake_delegation, sig)
}

pub fn signed_stake_pool_cert(valid_until: BlockDate, stake_pool: &StakePool) -> SignedCertificate {
    let owner = stake_pool.owner().clone();
    let txb = TxBuilder::new()
        .set_payload(&stake_pool.info())
        .set_expiry_date(valid_until)
        .set_ios(&[], &[])
        .set_witnesses(&[]);

    let auth_data = txb.get_auth_data();
    let sig0 = SingleAccountBindingSignature::new(&auth_data, |d| owner.sign_slice(d.0));
    let owner_signed = PoolOwnersSigned {
        signatures: vec![(0, sig0)],
    };

    SignedCertificate::PoolRegistration(stake_pool.info(), PoolSignature::Owners(owner_signed))
}

pub fn vote_plan_cert(
    wallet: &Wallet,
    valid_until: BlockDate,
    vote_plan: &VotePlan,
) -> SignedCertificate {
    let txb = TxBuilder::new()
        .set_payload(vote_plan)
        .set_expiry_date(valid_until)
        .set_ios(&[], &[])
        .set_witnesses(&[]);

    let auth_data = txb.get_auth_data();

    let signature = SingleAccountBindingSignature::new(&auth_data, |d| wallet.sign_slice(d.0));

    SignedCertificate::VotePlan(
        vote_plan.clone(),
        VotePlanProof {
            id: wallet.identifier().into_public_key().into(),
            signature,
        },
    )
}
