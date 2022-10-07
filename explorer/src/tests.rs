use assert_fs::TempDir;
use chain_core::property::FromStr;
use jormungandr_automation::{
    jormungandr::{
        explorer::configuration::ExplorerParams, Block0ConfigurationBuilder, Explorer,
        JormungandrProcess,
    },
    testing::block0::Block0ConfigurationExtension,
};
use jormungandr_integration_tests::startup::SingleNodeTestBootstrapper;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{ActiveSlotCoefficient, Block0Configuration},
};
use thor::{Block0ConfigurationBuilderExtension, StakePool};

pub struct ExplorerTestConfig {
    query_complexity_limit: u64,
}

impl Default for ExplorerTestConfig {
    fn default() -> Self {
        ExplorerTestConfig {
            query_complexity_limit: 150,
        }
    }
}

pub fn explorer_test_context(
    test_config: ExplorerTestConfig,
) -> (
    jormungandr_automation::jormungandr::ExplorerProcess,
    JormungandrProcess,
    Block0Configuration,
) {
    let temp_dir = TempDir::new().unwrap();
    let faucet = thor::Wallet::default();
    let stake_pool = StakePool::new(&faucet);
    let config = Block0ConfigurationBuilder::default()
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_wallets_having_some_values(vec![&faucet])
        .with_stake_pool_and_delegation(&stake_pool, vec![&faucet]);

    let context = SingleNodeTestBootstrapper::default()
        .as_genesis_praos_stake_pool(&stake_pool)
        .as_bft_leader()
        .with_block0_config(config)
        .build();
    let jormungandr = context.start_node(temp_dir).unwrap();
    let params = ExplorerParams::new(test_config.query_complexity_limit, None, None);

    let explorer_process = jormungandr.explorer(params).unwrap();
    println!("{:?}", explorer_process.client().current_time());
    (explorer_process, jormungandr, context.block0_config)
}

pub fn get_invalid_block(explorer: &Explorer) {
    let hash =
        Hash::from_str("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f").unwrap();

    let predicted_errors = vec![
        "internal error (this shouldn't happen) Couldn't find block in the explorer".to_string(),
    ];

    let actual_errors: Vec<String> = explorer
        .block(hash)
        .unwrap()
        .errors
        .unwrap()
        .iter()
        .map(|error| error.message.to_string())
        .collect();

    assert_eq!(predicted_errors, actual_errors);
}

pub fn get_valid_block(explorer: &Explorer, genesis_block: Hash) {
    let block_id = explorer
        .block(genesis_block)
        .unwrap()
        .data
        .unwrap()
        .block
        .id;
    assert_eq!(block_id, genesis_block.to_string());
}

pub fn verify_config_params_present(
    explorer: &Explorer,
    block0_configuration: Block0Configuration,
) {
    let binding = block0_configuration.to_block();

    // first fragment txs should contain config params
    let block0fragment = binding.fragments().next().unwrap();

    let params = explorer
        .transaction(Hash::from_str(&block0fragment.hash().to_string()).unwrap())
        .unwrap()
        .data
        .unwrap()
        .transaction
        .initial_configuration_params;

    assert!(params.is_some());
}

#[test]
pub fn explorer_tests() {
    let config = ExplorerTestConfig::default();

    let (explorer, _jormungandr, block0_config) = explorer_test_context(config);
    let block0_hash = block0_config.to_block_hash();

    get_invalid_block(explorer.client());
    get_valid_block(explorer.client(), block0_hash);
    verify_config_params_present(explorer.client(), block0_config);
}
