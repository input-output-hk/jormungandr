use crate::common::jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode};
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jormungandr_testing_utils::testing::node::{download_last_n_releases, get_jormungandr_bin};
use jormungandr_testing_utils::testing::{BranchCount, StopCriteria, StorageBuilder};
use std::time::Duration;

#[test]
pub fn bootstrap_from_1_gb_storage() {
    let storage_size = 1_000;
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

    let _jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config)
        .benchmark(&format!("bootstrap from {} MB storage", storage_size))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

#[test]
pub fn legacy_bootstrap_from_1_gb_storage() {
    let storage_size = 1_000;
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

    let legacy_release = download_last_n_releases(1).iter().cloned().next().unwrap();
    let jormungandr_app = get_jormungandr_bin(&legacy_release, &temp_dir);

    let _jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config)
        .legacy(legacy_release.version())
        .jormungandr_app(jormungandr_app)
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}
