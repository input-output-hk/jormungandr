use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fragment::FragmentId, key::Hash};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{explorer::configuration::ExplorerParams, ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;
use std::{str::FromStr, time::Duration};
use thor::{StakePool, TransactionHash};

/// test checks if there is upto date schema
/// prereq:
/// -npm
/// read more: https://github.com/prisma-labs/get-graphql-schema
#[test]
#[cfg(feature = "explorer-schema-gen")]
#[cfg(unix)]
pub fn explorer_schema_diff_test() {
    use assert_fs::{fixture::PathChild, TempDir};
    use jormungandr_automation::jormungandr::Starter;

    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new().build(&temp_dir);

    let _jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let schema_temp_dir = TempDir::new().unwrap();
    let actual_schema_path = schema_temp_dir.child("new_schema.graphql");

    std::process::Command::new(
        "../jormungandr-automation/resources/explorer/graphql/generate_schema.sh",
    )
    .args(&[actual_schema_path
        .path()
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string()])
    .spawn()
    .unwrap()
    .wait()
    .unwrap();

    jormungandr_automation::jormungandr::compare_explorer_schema(actual_schema_path.path());
}

#[test]
pub fn explorer_sanity_test() {
    let jcli: JCli = Default::default();
    let faucet = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let query_complexity_limit = 100;
    let attempts_number = 20;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, initial_stake_pools) =
        startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let params = ExplorerParams::new(query_complexity_limit, None, None);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let transaction = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&faucet, receiver.address(), 1_000.into())
    .unwrap()
    .encode();

    let wait = Wait::new(Duration::from_secs(3), attempts_number);
    let fragment_id = jcli
        .fragment_sender(&jormungandr)
        .send(&transaction)
        .assert_in_block_with_wait(&wait);

    transaction_by_id(explorer, fragment_id);
    blocks(explorer, jormungandr.logger.get_created_blocks_hashes());
    stake_pools(explorer, initial_stake_pools.as_ref());
    stake_pool(explorer, initial_stake_pools.as_ref());
    block_at_chain_length(explorer, jormungandr.logger.get_created_blocks_hashes());
    epoch(explorer);
}

fn transaction_by_id(explorer: &Explorer, fragment_id: FragmentId) {
    let explorer_transaction = explorer
        .transaction(fragment_id.into())
        .expect("Non existing transaction");

    assert_eq!(
        fragment_id,
        Hash::from_str(&explorer_transaction.data.unwrap().transaction.id).unwrap(),
        "Incorrect fragment id"
    );
}

fn blocks(explorer: &Explorer, blocks_from_logs: Vec<Hash>) {
    let blocks = explorer.blocks(1000).unwrap();

    // we are skipping first block because log doesn't contains genesis block
    let explorer_blocks = blocks
        .data
        .unwrap()
        .tip
        .blocks
        .edges
        .iter()
        .skip(1)
        .map(|x| Hash::from_str(&x.node.id).unwrap())
        .collect::<Vec<Hash>>();

    let mut common_blocks = blocks_from_logs.clone();
    common_blocks.retain(|x| !explorer_blocks.contains(x));

    // we can have at least one non duplicated block
    // due to explorer delay to logs content
    assert!(
        common_blocks.len() <= 1,
        "blocks differents: Explorer {:?} vs Logs {:?}",
        explorer_blocks,
        blocks_from_logs
    );
}

fn stake_pools(explorer: &Explorer, initial_stake_pools: &[StakePool]) {
    let stake_pools = explorer.stake_pools(1000).unwrap();
    let explorer_stake_pools = stake_pools.data.unwrap().tip.all_stake_pools.edges;
    // we are skipping first block because log doesn't contains genesis block
    assert_eq!(
        initial_stake_pools
            .iter()
            .map(|x| x.id().to_string())
            .collect::<Vec<String>>(),
        explorer_stake_pools
            .iter()
            .map(|x| x.node.id.clone())
            .collect::<Vec<String>>(),
        "blocks are empty"
    );
}

fn stake_pool(explorer: &Explorer, initial_stake_pools: &[StakePool]) {
    let stake_pool_id = initial_stake_pools.first().unwrap().id().to_string();
    let stake_pool = explorer.stake_pool(stake_pool_id, 100).unwrap();
    let explorer_stake_pool_id = stake_pool.data.unwrap().stake_pool.id;

    assert!(
        initial_stake_pools
            .iter()
            .any(|x| x.id().to_string() == explorer_stake_pool_id),
        "stake pool id"
    );
}

fn block_at_chain_length(explorer: &Explorer, blocks_from_logs: Vec<Hash>) {
    let block = explorer.blocks_at_chain_length(1).unwrap();

    assert_eq!(
        blocks_from_logs.first().unwrap().to_string(),
        block.data.unwrap().blocks_by_chain_length[0].id,
        "can't find block"
    );
}

fn epoch(explorer: &Explorer) {
    let epoch = explorer.epoch(1, 100).unwrap();

    assert_eq!(epoch.data.unwrap().epoch.id, "1", "can't find epoch");
}

struct ExplorerProcess {
    handler: Option<std::process::Child>,
    logs_dir: Option<std::path::PathBuf>,
}

impl Drop for ExplorerProcess {
    fn drop(&mut self) {
        let output = if let Some(mut handler) = self.handler.take() {
            let _ = handler.kill();
            handler.wait_with_output().unwrap()
        } else {
            return;
        };

        if std::thread::panicking() {
            if let Some(logs_dir) = &self.logs_dir {
                println!(
                    "persisting explorer logs after panic: {}",
                    logs_dir.display()
                );

                std::fs::write(logs_dir.join("explorer.log"), output.stdout)
                    .unwrap_or_else(|e| eprint!("Could not write node logs to disk: {}", e));

                std::fs::write(logs_dir.join("explorer.err"), output.stderr)
                    .unwrap_or_else(|e| eprint!("Could not write node logs to disk: {}", e));
            }
        }
    }
}
