use chain_core::property::FromStr;

use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_automation::jormungandr::{
    explorer::configuration::ExplorerParams, ConfigurationBuilder, Explorer,
};

use jormungandr_integration_tests::startup;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;

pub struct ExplorerTestConfig {
    query_complexity_limit: u64,
}

pub fn explorer_test_context(
    test_config: ExplorerTestConfig,
) -> (
    jormungandr_automation::jormungandr::ExplorerProcess,
    JormungandrProcess,
) {
    let faucet = thor::Wallet::default();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[faucet], &[], &mut config).unwrap();

    let params = ExplorerParams::new(test_config.query_complexity_limit, None, None);

    (jormungandr.explorer(params), jormungandr)
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

#[test]
pub fn explorer_tests() {
    let config = ExplorerTestConfig {
        query_complexity_limit: 70,
    };

    let (explorer, jormungandr) = explorer_test_context(config);

    get_invalid_block(explorer.client());
    get_valid_block(explorer.client(), jormungandr.genesis_block_hash());
}
