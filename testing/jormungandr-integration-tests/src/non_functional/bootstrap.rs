use crate::common::jormungandr::{Starter, StartupVerificationMode};
use crate::common::{
    jcli::JCli, jormungandr::ConfigurationBuilder, startup, transaction_utils::TransactionHash,
};
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jormungandr_testing_utils::testing::{
    benchmark_consumption, benchmark_endurance, ResourcesUsage,
};
use jormungandr_testing_utils::testing::{BranchCount, StopCriteria, StorageBuilder};
use jortestkit::process as process_utils;
use std::time::Duration;

#[test]
pub fn bootstrap_from_500_mb_storage() {
    let storage_size = 500;
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let child = temp_dir.child("storage");
    let path = child.path();
    let storage_builder = StorageBuilder::new(
        BranchCount::Unlimited,
        StopCriteria::SizeInMb(storage_size),
        path,
    );
    storage_builder.build();

    let config = ConfigurationBuilder::new()
        .with_slots_per_epoch(20)
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_storage(&child)
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config)
        .benchmark(&format!("bootstrap from {} MB storage", storage_size))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}
