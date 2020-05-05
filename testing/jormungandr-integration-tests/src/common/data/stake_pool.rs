use crate::common::{
    file_utils, jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    startup::create_new_key_pair,
};
use chain_crypto::{bech32::Bech32, Curve25519_2HashDH, Ed25519, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::PoolPermissions,
    rewards::{Ratio as RatioLib, TaxType},
    testing::{builders::StakePoolBuilder, data::StakePool as StakePoolLib},
    value::Value as ValueLib,
};
use jormungandr_lib::crypto::key::KeyPair;
use jormungandr_testing_utils::wallet::Wallet;
use std::num::NonZeroU64;
use std::path::PathBuf;
// temporary struct which should be replaced by one from chain-libs or jormungandr-lib
#[derive(Clone, Debug)]
pub struct StakePool {
    leader: KeyPair<Ed25519>,
    owner: Wallet,
    inner: StakePoolLib,
    stake_pool_signcert_file: PathBuf,
    stake_pool_id: String,
}

impl StakePool {
    pub fn new(owner: &Wallet) -> Self {
        let leader = create_new_key_pair::<Ed25519>();

        let stake_key = owner.signing_key_as_str();
        let stake_key_pub = owner.identifier().to_bech32_str();
        let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key);

        let jcli_certificate = JCLICertificateWrapper::new();

        let stake_pool = StakePoolBuilder::new()
            .with_owners(vec![owner.identifier().into_public_key()])
            .with_pool_permissions(PoolPermissions::new(1))
            .with_reward_account(false)
            .with_tax_type(TaxType {
                fixed: ValueLib(100),
                ratio: RatioLib {
                    numerator: 1,
                    denominator: NonZeroU64::new(10).unwrap(),
                },
                max_limit: None,
            })
            .build();

        let stake_pool_signcert_file = jcli_certificate.assert_new_signed_stake_pool_cert(
            &stake_pool.kes().public_key().to_bech32_str(),
            &stake_pool.vrf().public_key().to_bech32_str(),
            &stake_key_file,
            0,
            stake_pool.info().permissions.management_threshold().into(),
            &stake_key_pub,
            Some(stake_pool.info().rewards.into()),
        );

        StakePool {
            owner: owner.clone(),
            leader: leader,
            inner: stake_pool,
            stake_pool_signcert_file: stake_pool_signcert_file.clone(),
            stake_pool_id: jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file),
        }
    }

    pub fn leader(&self) -> &KeyPair<Ed25519> {
        &self.leader
    }

    pub fn stake_pool_signcert_file(&self) -> &PathBuf {
        &self.stake_pool_signcert_file
    }

    pub fn owner(&self) -> &Wallet {
        &self.owner
    }

    pub fn id(&self) -> &str {
        &self.stake_pool_id
    }

    pub fn kes(&self) -> KeyPair<SumEd25519_12> {
        KeyPair::<SumEd25519_12>(self.inner.kes())
    }

    pub fn vrf(&self) -> KeyPair<Curve25519_2HashDH> {
        KeyPair::<Curve25519_2HashDH>(self.inner.vrf())
    }
}

impl Into<StakePoolLib> for StakePool {
    fn into(self) -> StakePoolLib {
        self.inner
    }
}
