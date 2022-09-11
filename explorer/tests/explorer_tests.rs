#[cfg(test)]
mod tests {

    use chain_core::property::FromStr;

    use jormungandr_automation::jormungandr::{
        explorer::configuration::ExplorerParams, ConfigurationBuilder, Explorer,
    };

    use jormungandr_integration_tests::startup;
    use jormungandr_lib::crypto::hash::Hash;
    use jormungandr_lib::interfaces::ActiveSlotCoefficient;

    #[test]
    pub fn explorer_tests() {
        let faucet = thor::Wallet::default();

        let query_complexity_limit = 70;

        let mut config = ConfigurationBuilder::new();
        config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

        let (jormungandr, _initial_stake_pools) =
            startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

        let params = ExplorerParams::new(query_complexity_limit, None, None);
        let explorer_process = jormungandr.explorer(params);
        let explorer = explorer_process.client();

        get_invalid_block(explorer);
        get_valid_block(explorer, jormungandr.genesis_block_hash());
    }

    pub fn get_invalid_block(explorer: &Explorer) {
        let hash = Hash::from_str(
            &"000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f".to_string(),
        )
        .unwrap();

        if let Some(errors) = explorer.block(hash).unwrap().errors {
            for error in &errors {
                assert_eq!(
                    error.message.to_string(),
                    "internal error (this shouldn't happen) Couldn't find block in the explorer"
                        .to_string()
                );
            }
        }
    }

    pub fn get_valid_block(explorer: &Explorer, genesis_block: Hash) {
        if let Some(data) = explorer.block(genesis_block).unwrap().data {
            assert_eq!(data.block.id, genesis_block.to_string());
        }
    }
}
