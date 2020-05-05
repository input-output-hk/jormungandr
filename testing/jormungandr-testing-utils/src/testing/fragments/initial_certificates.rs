use crate::wallet::Wallet;
use chain_impl_mockchain::{
    certificate::{PoolId, SignedCertificate, StakeDelegation},
    transaction::{AccountBindingSignature, TxBuilder},
};
use jormungandr_lib::interfaces::Initial;

pub fn full_delegation_cert_for_block0(wallet: &Wallet, pool_id: PoolId) -> Initial {
    let stake_delegation = StakeDelegation {
        account_id: wallet.stake_key().unwrap(),
        delegation: chain_impl_mockchain::account::DelegationType::Full(pool_id),
    };
    let txb = TxBuilder::new()
        .set_payload(&stake_delegation)
        .set_ios(&[], &[])
        .set_witnesses(&[]);
    let auth_data = txb.get_auth_data();

    let sig = AccountBindingSignature::new_single(&auth_data, |d| wallet.sign_slice(d.0));
    Initial::Cert(SignedCertificate::StakeDelegation(stake_delegation, sig).into())
}
