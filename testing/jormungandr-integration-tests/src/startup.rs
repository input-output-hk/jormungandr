#![allow(dead_code)]

use crate::context::{ActorsTestContext, LegacyTestContext, TestContext};
use assert_fs::fixture::TempDir;
use chain_crypto::Ed25519;
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_automation::jormungandr::{
    Block0ConfigurationBuilder, JormungandrBootstrapper, JormungandrProcess,
    LegacyNodeConfigBuilder, NodeConfigBuilder, SecretModelFactory, StartupError, Version,
};
use jormungandr_lib::{
    crypto::{
        hash::Hash,
        key::{Identifier, KeyPair},
    },
    interfaces::{
        ActiveSlotCoefficient, Bft, ConsensusLeaderId, GenesisPraos, Initial, InitialUTxO,
        SignedCertificate, Value,
    },
};
use std::path::PathBuf;
use thor::{
    signed_delegation_cert, signed_stake_pool_cert, Block0ConfigurationBuilderExtension, StakePool,
    Wallet,
};

#[derive(Default)]
pub struct ActorsTestBootstrapper {
    bootstrapper: SingleNodeTestBootstrapper,
    alice: Option<Wallet>,
    bob: Option<Wallet>,
}

impl ActorsTestBootstrapper {
    pub(crate) fn with_bft_leader_node(mut self) -> Self {
        self.bootstrapper = self.bootstrapper.as_bft_leader();
        self
    }
}

impl ActorsTestBootstrapper {
    pub(crate) fn with_bob_without_funds(mut self) -> Self {
        self.bob = Some(Wallet::default());
        self
    }
}

impl ActorsTestBootstrapper {
    pub(crate) fn with_alice(mut self, value: Value) -> Self {
        let alice = Wallet::default();
        self.bootstrapper.block0_builder =
            self.bootstrapper.block0_builder.with_wallet(&alice, value);
        self.alice = Some(alice);
        self
    }

    pub fn build(self) -> ActorsTestContext {
        ActorsTestContext {
            test_context: self.bootstrapper.build(),
            alice: self.alice,
            bob: self.bob,
        }
    }
}

pub struct LegacySingleNodeTestBootstrapper {
    inner: SingleNodeTestBootstrapper,
    legacy_node_config: LegacyNodeConfigBuilder,
    jormungandr_app: Option<PathBuf>,
    version: Version,
}

impl From<Version> for LegacySingleNodeTestBootstrapper {
    fn from(version: Version) -> Self {
        Self {
            version,
            inner: Default::default(),
            legacy_node_config: Default::default(),
            jormungandr_app: Default::default(),
        }
    }
}

impl LegacySingleNodeTestBootstrapper {
    pub fn with_jormungandr_app(mut self, jormungandr_app: PathBuf) -> Self {
        self.jormungandr_app = Some(jormungandr_app);
        self
    }

    pub fn with_version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    pub fn with_rewards_history(mut self) -> Self {
        self.inner = self.inner.with_rewards_history();
        self
    }
    pub fn with_node_config(mut self, legacy_node_config: LegacyNodeConfigBuilder) -> Self {
        self.legacy_node_config = legacy_node_config;
        self
    }

    pub fn as_bft_leader(mut self) -> Self {
        self.inner = self.inner.as_bft_leader();
        self
    }

    pub fn with_block0_config(mut self, config: Block0ConfigurationBuilder) -> Self {
        self.inner = self.inner.with_block0_config(config);
        self
    }

    pub fn build(
        self,
    ) -> Result<LegacyTestContext, jormungandr_automation::jormungandr::LegacyConfigError> {
        Ok(LegacyTestContext {
            jormungandr_app: self.jormungandr_app,
            version: self.version,
            test_context: self.inner.build(),
            legacy_node_config: self.legacy_node_config.build()?,
        })
    }
}

#[derive(Default)]
pub struct SingleNodeTestBootstrapper {
    block0_builder: Block0ConfigurationBuilder,
    node_config_builder: NodeConfigBuilder,
    secret: Option<SecretModelFactory>,
    reward_history: bool,
}

impl SingleNodeTestBootstrapper {
    pub fn with_rewards_history(mut self) -> Self {
        self.reward_history = true;
        self
    }
    pub fn with_node_config(mut self, node_config: NodeConfigBuilder) -> Self {
        self.node_config_builder = node_config;
        self
    }

    pub fn as_bft_leader(mut self) -> Self {
        self.secret = Some(SecretModelFactory::bft(
            create_new_leader_key().signing_key(),
        ));
        self
    }

    pub fn as_genesis_praos_stake_pool(mut self, stake_pool: &StakePool) -> Self {
        self.secret = Some(SecretModelFactory {
            bft: Some(Bft {
                signing_key: create_new_leader_key().signing_key(),
            }),
            genesis: Some(GenesisPraos {
                sig_key: stake_pool.kes().signing_key(),
                vrf_key: stake_pool.vrf().signing_key(),
                node_id: Hash::from(stake_pool.id()),
            }),
        });
        self
    }

    pub fn with_block0_config(mut self, config: Block0ConfigurationBuilder) -> Self {
        self.block0_builder.initial.extend(config.initial);
        self.block0_builder.blockchain_configuration = config.blockchain_configuration;
        self
    }

    pub fn build(mut self) -> TestContext {
        let secret_factory = self.secret.unwrap_or_default();

        if secret_factory.genesis.is_some() && secret_factory.bft.is_some() {
            self.block0_builder = self
                .block0_builder
                .with_block0_consensus(ConsensusVersion::GenesisPraos)
                .with_leader_signing_key(secret_factory.bft.as_ref().unwrap().signing_key.clone());
        } else if secret_factory.genesis.is_some() {
            self.block0_builder = self
                .block0_builder
                .with_block0_consensus(ConsensusVersion::GenesisPraos);
        } else if let Some(bft) = &secret_factory.bft {
            self.block0_builder = self
                .block0_builder
                .with_leader_signing_key(bft.signing_key.clone())
                .with_block0_consensus(ConsensusVersion::Bft);
        }

        TestContext {
            block0_config: self.block0_builder.build(),
            secret_factory,
            node_config: self.node_config_builder.build(),
            reward_history: self.reward_history,
        }
    }
}

pub fn create_new_utxo_address() -> Wallet {
    Wallet::new_utxo(&mut rand::rngs::OsRng)
}

pub fn create_new_leader_key() -> KeyPair<Ed25519> {
    KeyPair::generate(&mut rand::thread_rng())
}

pub fn create_new_account_address() -> Wallet {
    Wallet::default()
}

pub fn create_new_delegation_address() -> Wallet {
    let account = Wallet::default();
    create_new_delegation_address_for(&account.identifier())
}

pub fn create_new_delegation_address_for(delegation_identifier: &Identifier<Ed25519>) -> Wallet {
    Wallet::new_delegation(
        &delegation_identifier.clone().into(),
        &mut rand::rngs::OsRng,
    )
}

pub fn start_stake_pool(
    owners: &[Wallet],
    initial_funds: &[Wallet],
    block0_config: Block0ConfigurationBuilder,
    node_config_builder: NodeConfigBuilder,
) -> Result<(JormungandrProcess, Vec<StakePool>), StartupError> {
    let stake_pools: Vec<StakePool> = owners.iter().map(StakePool::new).collect();

    let stake_pool_registration_certs: Vec<SignedCertificate> = stake_pools
        .iter()
        .map(|x| {
            signed_stake_pool_cert(
                chain_impl_mockchain::block::BlockDate {
                    epoch: 1,
                    slot_id: 0,
                },
                x,
            )
            .into()
        })
        .collect();
    let stake_pool_owner_delegation_certs: Vec<SignedCertificate> = stake_pools
        .iter()
        .map(|x| {
            signed_delegation_cert(
                x.owner(),
                chain_impl_mockchain::block::BlockDate {
                    epoch: 1,
                    slot_id: 0,
                },
                x.id(),
            )
            .into()
        })
        .collect();

    let mut initial_certs = stake_pool_registration_certs;
    initial_certs.extend(stake_pool_owner_delegation_certs.iter().cloned());

    let leaders: Vec<ConsensusLeaderId> = stake_pools
        .iter()
        .map(|_| create_new_leader_key().identifier().into())
        .collect();

    let mut funds: Vec<InitialUTxO> = owners
        .iter()
        .map(|x| x.to_initial_fund(1_000_000_000))
        .collect();

    let funds_non_owners: Vec<InitialUTxO> = initial_funds
        .iter()
        .map(|x| x.to_initial_fund(1_000_000_000))
        .collect();

    funds.extend(funds_non_owners);

    let temp_dir = TempDir::new()?;

    let secret = SecretModelFactory::genesis(
        stake_pools[0].kes().signing_key(),
        stake_pools[0].vrf().signing_key(),
        &stake_pools[0].id().to_string(),
    );

    let block0_config = block0_config
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .with_consensus_leaders_ids(leaders)
        .with_funds(vec![Initial::Fund(funds)])
        .with_certs(initial_certs.into_iter().map(Initial::Cert).collect())
        .build();

    JormungandrBootstrapper::default()
        .with_block0_configuration(block0_config)
        .with_secret(secret)
        .with_node_config(node_config_builder.build())
        .into_starter(temp_dir)?
        .start()
        .map(|process| (process, stake_pools))
}
