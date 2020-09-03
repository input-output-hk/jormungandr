use crate::common::{
    jcli_wrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    process_utils::Wait,
    startup,
    transaction_utils::TransactionHash,
};
use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::key::Hash;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use std::str::FromStr;
use std::{path::PathBuf, process::Command, time::Duration};
/// test checks if there is upto date schema
/// prereq:
/// -npm
/// read more: https://github.com/prisma-labs/get-graphql-schema
#[test]
#[cfg(feature = "explorer-schema-gen")]
#[cfg(unix)]
pub fn explorer_schema_diff_test() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new().with_explorer().build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let schema_temp_dir = TempDir::new().unwrap();
    let actual_schema_path = schema_temp_dir.child("new_schema.graphql");

    Command::new("../jormungandr-testing-utils/resources/explorer/graphql/generate_schema.sh")
        .args(&[
            jormungandr.explorer().uri(),
            actual_schema_path
                .path()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string(),
        ])
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    jormungandr_testing_utils::testing::node::explorer::compare_schema(actual_schema_path.path());
}

#[test]
pub fn explorer_sanity_test() {
    let mut faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_explorer();

    let (jormungandr, _) = startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let transaction = faucet
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1_000.into(),
        )
        .unwrap()
        .encode();

    let wait = Wait::new(Duration::from_secs(3), 20);

    let fragment_id =
        jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, &wait);

    let explorer = jormungandr.explorer();
    let explorer_transaction = explorer
        .get_transaction(fragment_id)
        .expect("non existing transaction");

    assert_eq!(
        fragment_id,
        Hash::from_str(&explorer_transaction.data.unwrap().transaction.id).unwrap(),
        "incorrect fragment id"
    );
}
