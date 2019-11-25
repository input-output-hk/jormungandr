use crate::{
    fragment::Fragment, 
    testing::{
        builders::*,
        data::{
            StakePool,Wallet
        }        
    },
    certificate::Certificate,
    transaction::*, 
    key::EitherEd25519SecretKey,
};
use std::vec::Vec;

pub fn create_initial_stake_pool_registration(stake_pool: &StakePool, owners: &[Wallet]) -> Fragment {
    let cert = build_stake_pool_registration_cert(&stake_pool.info());
    let keys: Vec<EitherEd25519SecretKey>= owners.iter().cloned().map(|owner| owner.private_key()).collect();
    fragment(cert,keys)
}

pub fn create_initial_stake_pool_delegation(stake_pool: &StakePool, wallet: &Wallet) -> Fragment {
    let cert = build_stake_delegation_cert(&stake_pool.info(), &wallet.as_account_data());
    let keys: Vec<EitherEd25519SecretKey> = vec![wallet.private_key()];
    fragment(cert,keys)
}

fn set_initial_ios<P: Payload>(builder: TxBuilderState<SetIOs<P>>) -> TxBuilderState<SetAuthData<P>> {
    builder.set_ios(&[], &[]).set_witnesses(&[])
}

fn fragment(cert: Certificate, keys: Vec<EitherEd25519SecretKey>) -> Fragment {
   match cert {
        Certificate::StakeDelegation(s) => {
            let builder = set_initial_ios(TxBuilder::new().set_payload(&s));
            let signature = AccountBindingSignature::new_single(&keys[0], &builder.get_auth_data());
            let tx = builder.set_payload_auth(&signature);
            Fragment::StakeDelegation(tx)
        }
        Certificate::PoolRegistration(s) => {
            let builder = set_initial_ios(TxBuilder::new().set_payload(&s));
            let signature = pool_owner_sign(&keys, &builder);
            let tx = builder.set_payload_auth(&signature);
            Fragment::PoolRegistration(tx)
        }
        _ => unreachable!(),
    }
}
