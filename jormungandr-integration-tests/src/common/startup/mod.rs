use crate::common::{
    configuration::secret_model::SecretModel,
    data::address::{Account, Delegation, Utxo},
    file_utils,
    jcli_wrapper::{self, certificate::wrapper::JCLICertificateWrapper},
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter, StartupError},
};
use chain_addr::Discrimination;
use chain_crypto::{AsymmetricKey, Curve25519_2HashDH, Ed25519, Ed25519Extended, SumEd25519_12};
use chain_impl_mockchain::block::ConsensusVersion;
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{Block0Configuration, ConsensusLeaderId, InitialUTxO, Ratio, TaxType},
};
use std::path::PathBuf;

pub fn get_genesis_block_hash(block0_config: &Block0Configuration) -> String {
    let path_to_output_block = build_genesis_block(&block0_config);
    jcli_wrapper::assert_genesis_hash(&path_to_output_block)
}

pub fn build_genesis_block(block0_config: &Block0Configuration) -> PathBuf {
    let input_yaml_file_path = serialize_block0_config(&block0_config);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);

    path_to_output_block
}

pub fn serialize_block0_config(block0_config: &Block0Configuration) -> PathBuf {
    let content = serde_yaml::to_string(&block0_config).unwrap();
    let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);
    input_yaml_file_path
}

pub fn create_new_utxo_address() -> Utxo {
    let key_pair = create_new_key_pair::<Ed25519Extended>();
    let private_key = key_pair.signing_key().to_bech32_str();
    let public_key = key_pair.identifier().to_bech32_str();

    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    let utxo = Utxo {
        private_key,
        public_key,
        address,
    };
    utxo
}

pub fn create_new_account_address() -> Account {
    let key_pair = create_new_key_pair::<Ed25519Extended>();
    let private_key = key_pair.signing_key().to_bech32_str();
    let public_key = key_pair.identifier().to_bech32_str();

    let address = jcli_wrapper::assert_address_account(&public_key, Discrimination::Test);
    Account::new(&private_key, &public_key, &address)
}

pub fn create_new_delegation_address() -> Delegation {
    let key_pair = create_new_key_pair::<Ed25519Extended>();
    let public_delegation_key = key_pair.identifier().to_bech32_str();

    create_new_delegation_address_for(&public_delegation_key)
}

pub fn create_new_delegation_address_for(delegation_public_key: &str) -> Delegation {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_delegation(
        &public_key,
        delegation_public_key,
        Discrimination::Test,
    );

    let utxo_with_delegation = Delegation {
        private_key: private_key,
        public_key: public_key,
        address: address,
        delegation_key: delegation_public_key.to_string(),
    };
    println!(
        "New utxo with delegation generated: {:?}",
        &utxo_with_delegation
    );
    utxo_with_delegation
}

pub fn create_new_key_pair<K: AsymmetricKey>() -> KeyPair<K> {
    KeyPair::generate(rand::rngs::OsRng)
}

fn create_stake_pool(owner: &Account) -> StakePool {
    // leader
    let leader = create_new_key_pair::<Ed25519>();

    // stake pool
    let pool_vrf = create_new_key_pair::<Curve25519_2HashDH>();
    let pool_kes = create_new_key_pair::<SumEd25519_12>();

    // note we use the faucet as the owner to this pool
    let stake_key = owner.private_key.clone();
    let stake_key_pub = owner.public_key.clone();

    let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key);

    let jcli_certificate = JCLICertificateWrapper::new();

    let stake_pool_signcert_file = jcli_certificate.assert_new_signed_stake_pool_cert(
        &pool_kes.identifier().to_bech32_str(),
        &pool_vrf.identifier().to_bech32_str(),
        &stake_key_file,
        0,
        1,
        &stake_key_pub,
        Some(TaxType {
            fixed: 100.into(),
            ratio: Ratio::new_checked(1, 10).unwrap(),
            max_limit: None,
        }),
    );
    StakePool {
        owner: owner.clone(),
        leader: leader,
        pool_vrf: pool_vrf,
        pool_kes: pool_kes,
        stake_pool_signcert_file: stake_pool_signcert_file.clone(),
        stake_pool_id: jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file),
    }
}

fn create_stake_pool_owner_delegation_cert(stake_pool: &StakePool) -> String {
    let stake_key = stake_pool.owner.private_key.clone();
    let stake_key_pub = stake_pool.owner.public_key.clone();
    let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key);

    JCLICertificateWrapper::new().assert_new_signed_stake_pool_delegation(
        &stake_pool.stake_pool_id,
        &stake_key_pub,
        &stake_key_file,
    )
}

pub fn start_stake_pool(
    owners: &[Account],
    config_builder: &mut ConfigurationBuilder,
) -> Result<(JormungandrProcess, Vec<String>), StartupError> {
    let stake_pools: Vec<StakePool> = owners.iter().map(|x| create_stake_pool(x)).collect();

    let stake_pool_registration_certs: Vec<String> = stake_pools
        .iter()
        .map(|x| file_utils::read_file(&x.stake_pool_signcert_file))
        .collect();
    let stake_pool_owner_delegation_certs: Vec<String> = stake_pools
        .iter()
        .map(|x| create_stake_pool_owner_delegation_cert(&x))
        .collect();

    let mut initial_certs = stake_pool_registration_certs.clone();
    initial_certs.extend(stake_pool_owner_delegation_certs.iter().cloned());

    let leaders: Vec<ConsensusLeaderId> = stake_pools
        .iter()
        .map(|x| x.leader.identifier().into())
        .collect();

    let funds: Vec<InitialUTxO> = owners
        .iter()
        .map(|x| InitialUTxO {
            address: x.address.parse().unwrap(),
            value: 1_000_000_000.into(),
        })
        .collect();

    let mut config = config_builder
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .with_consensus_leaders_ids(leaders)
        .with_funds(funds)
        .with_initial_certs(initial_certs)
        .build();

    let secrets: Vec<SecretModel> = stake_pools
        .iter()
        .map(|x| {
            SecretModel::new_genesis(
                &x.pool_kes.signing_key().to_bech32_str(),
                &x.pool_vrf.signing_key().to_bech32_str(),
                &x.stake_pool_id,
            )
        })
        .collect();

    let secret_model_paths = secrets.iter().map(|x| SecretModel::serialize(&x)).collect();

    config.secret_models = secrets;
    config.secret_model_paths = secret_model_paths;

    let stake_pool_ids: Vec<String> = stake_pools
        .iter()
        .map(|x| x.stake_pool_id.clone())
        .collect();

    Starter::new()
        .config(config)
        .start()
        .map(|process| (process, stake_pool_ids))
}

// temporary struct which should be replaced by one from chain-libs or jormungandr-lib
struct StakePool {
    pub owner: Account,
    pub leader: KeyPair<Ed25519>,
    pub pool_vrf: KeyPair<Curve25519_2HashDH>,
    pub pool_kes: KeyPair<SumEd25519_12>,
    pub stake_pool_signcert_file: PathBuf,
    pub stake_pool_id: String,
}
