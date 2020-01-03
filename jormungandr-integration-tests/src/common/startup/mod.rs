use crate::common::{
    configuration::{
        genesis_model::{Fund, GenesisYaml},
        secret_model::SecretModel,
    },
    data::address::{Account, Delegation, Utxo},
    file_utils,
    jcli_wrapper::{self, certificate::wrapper::JCLICertificateWrapper},
    jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter, StartupError},
};
use chain_addr::Discrimination;
use chain_crypto::{AsymmetricKey, Curve25519_2HashDH, Ed25519, SumEd25519_12};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{Ratio, TaxType},
};
use std::path::PathBuf;

pub fn get_genesis_block_hash(genesis_yaml: &GenesisYaml) -> String {
    let path_to_output_block = build_genesis_block(&genesis_yaml);

    jcli_wrapper::assert_genesis_hash(&path_to_output_block)
}

pub fn build_genesis_block(genesis_yaml: &GenesisYaml) -> PathBuf {
    let input_yaml_file_path = GenesisYaml::serialize(&genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);

    path_to_output_block
}

pub fn create_new_utxo_address() -> Utxo {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    let utxo = Utxo {
        private_key,
        public_key,
        address,
    };
    utxo
}

pub fn create_new_account_address() -> Account {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_account(&public_key, Discrimination::Test);
    Account::new(&private_key, &public_key, &address)
}

pub fn create_new_delegation_address() -> Delegation {
    let private_delegation_key = jcli_wrapper::assert_key_generate_default();
    let public_delegation_key = jcli_wrapper::assert_key_to_public_default(&private_delegation_key);
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
    KeyPair::generate(&mut rand::rngs::OsRng::new().unwrap())
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

    let leaders: Vec<String> = stake_pools
        .iter()
        .map(|x| x.leader.identifier().to_bech32_str())
        .collect();

    let funds: Vec<Fund> = owners
        .iter()
        .map(|x| Fund {
            address: x.address.clone(),
            value: 1_000_000.into(),
        })
        .collect();

    let mut config = config_builder
        .with_block0_consensus("genesis_praos")
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(leaders)
        .with_kes_update_speed(43200)
        .with_initial_certs(initial_certs)
        .with_funds(funds)
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
