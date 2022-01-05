use assert_fs::fixture::TempDir;
use chain_crypto::Ed25519;
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::{
    crypto::key::Identifier,
    interfaces::{ConsensusLeaderId, InitialUTxO, NodeSecret, SignedCertificate},
};
use jormungandr_testing_utils::testing::{
    configuration::SecretModelFactory,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter, StartupError},
};
use thor::{signed_delegation_cert, signed_stake_pool_cert, StakePool, Wallet};

pub fn create_new_utxo_address() -> Wallet {
    Wallet::new_utxo(&mut rand::rngs::OsRng)
}

pub fn create_new_account_address() -> Wallet {
    Wallet::new_account(&mut rand::rngs::OsRng)
}

pub fn create_new_delegation_address() -> Wallet {
    let account = Wallet::new_account(&mut rand::rngs::OsRng);
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
    config_builder: &mut ConfigurationBuilder,
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
        .map(|x| x.leader().identifier().into())
        .collect();

    let mut funds: Vec<InitialUTxO> = owners
        .iter()
        .map(|x| InitialUTxO {
            address: x.address(),
            value: 1_000_000_000.into(),
        })
        .collect();

    let funds_non_owners: Vec<InitialUTxO> = initial_funds
        .iter()
        .map(|x| InitialUTxO {
            address: x.address(),
            value: 1_000_000_000.into(),
        })
        .collect();

    funds.extend(funds_non_owners);

    let temp_dir = TempDir::new()?;

    let secret: NodeSecret = SecretModelFactory::genesis(
        stake_pools[0].kes().signing_key(),
        stake_pools[0].vrf().signing_key(),
        &stake_pools[0].id().to_string(),
    );

    let config = config_builder
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .with_consensus_leaders_ids(leaders)
        .with_funds(funds)
        .with_explorer()
        .with_initial_certs(initial_certs)
        .with_secret(secret)
        .build(&temp_dir);

    Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .map(|process| (process, stake_pools))
}

pub fn start_bft(
    initial_funds: Vec<&Wallet>,
    config_builder: &mut ConfigurationBuilder,
) -> Result<JormungandrProcess, StartupError> {
    let temp_dir = TempDir::new()?;

    let config = config_builder
        .with_funds(
            initial_funds
                .iter()
                .map(|x| InitialUTxO {
                    address: x.address(),
                    value: 1_000_000_000.into(),
                })
                .collect(),
        )
        .with_block0_consensus(ConsensusVersion::Bft)
        .with_explorer()
        .build(&temp_dir);

    Starter::new().temp_dir(temp_dir).config(config).start()
}
