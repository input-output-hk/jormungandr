use assert_fs::{fixture::PathChild, TempDir};
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, ConfigurationBuilder, Starter,
        StartupVerificationMode,
    },
    testing::{BranchCount, StopCriteria, StorageBuilder},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use std::time::Duration;

#[test]
#[ignore]
pub fn bootstrap_from_100_mb_storage() {
    let storage_size = 100;
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
        .config(config.clone())
        .benchmark(&format!("bootstrap from {} MB storage", storage_size))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    jormungandr.shutdown();

    let jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config.clone())
        .benchmark(&format!(
            "bootstrap from {} MB storage after restart",
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    jormungandr.stop();

    let _jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config)
        .benchmark(&format!(
            "bootstrap from {} MB storage after kill",
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

#[test]
#[ignore]
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

    let legacy_release = download_last_n_releases(1).get(0).cloned().unwrap();
    let jormungandr_app = get_jormungandr_bin(&legacy_release, &temp_dir);

    let jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config.clone())
        .legacy(legacy_release.version())
        .jormungandr_app(jormungandr_app.clone())
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    jormungandr.shutdown();

    let jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config.clone())
        .legacy(legacy_release.version())
        .jormungandr_app(jormungandr_app.clone())
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage after restart",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    jormungandr.stop();

    let _jormungandr = Starter::new()
        .timeout(Duration::from_secs(24_000))
        .config(config)
        .legacy(legacy_release.version())
        .jormungandr_app(jormungandr_app)
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage after kill",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}
