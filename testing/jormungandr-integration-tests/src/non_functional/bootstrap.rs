use crate::startup::{LegacySingleNodeTestBootstrapper, SingleNodeTestBootstrapper};
use assert_fs::{fixture::PathChild, TempDir};
use jormungandr_automation::{
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, Block0ConfigurationBuilder,
        JormungandrBootstrapper, LegacyNodeConfigBuilder, NodeConfigBuilder,
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

    let config = Block0ConfigurationBuilder::default()
        .with_slots_per_epoch(20.try_into().unwrap())
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let node_config = NodeConfigBuilder::default().with_storage(child.to_path_buf());

    let test_context = SingleNodeTestBootstrapper::default()
        .with_node_config(node_config)
        .with_block0_config(config)
        .as_bft_leader()
        .build();

    let mut jormungandr = test_context
        .starter(temp_dir)
        .unwrap()
        .timeout(Duration::from_secs(24_000))
        .benchmark(&format!("bootstrap from {} MB storage", storage_size))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    let temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();
    jormungandr.shutdown();

    let mut jormungandr = JormungandrBootstrapper::default()
        .with_node_config(test_context.node_config())
        .with_block0_configuration(test_context.block0_config())
        .into_starter(temp_dir)
        .unwrap()
        .timeout(Duration::from_secs(24_000))
        .benchmark(&format!(
            "bootstrap from {} MB storage after restart",
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    let temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();
    jormungandr.stop();

    let _jormungandr = JormungandrBootstrapper::default()
        .with_node_config(test_context.node_config())
        .with_block0_configuration(test_context.block0_config())
        .into_starter(temp_dir)
        .unwrap()
        .timeout(Duration::from_secs(24_000))
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

    let block0_config = Block0ConfigurationBuilder::default()
        .with_slots_per_epoch(20.try_into().unwrap())
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let node_config = LegacyNodeConfigBuilder::default().with_storage(child.to_path_buf());

    let legacy_release = download_last_n_releases(1).get(0).cloned().unwrap();
    let jormungandr_app = get_jormungandr_bin(&legacy_release, &temp_dir);

    let test_context = LegacySingleNodeTestBootstrapper::from(legacy_release.version())
        .with_block0_config(block0_config)
        .as_bft_leader()
        .with_jormungandr_app(jormungandr_app.clone())
        .with_node_config(node_config)
        .build()
        .unwrap();

    let mut jormungandr = test_context
        .starter(temp_dir)
        .unwrap()
        .timeout(Duration::from_secs(24_000))
        .jormungandr_app(jormungandr_app.clone())
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    let temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();
    jormungandr.shutdown();

    let mut jormungandr = JormungandrBootstrapper::default()
        .with_legacy_node_config(test_context.legacy_node_config.clone())
        .with_block0_configuration(test_context.block0_config())
        .into_starter(temp_dir)
        .unwrap()
        .timeout(Duration::from_secs(24_000))
        .jormungandr_app(jormungandr_app.clone())
        .benchmark(&format!(
            "legacy {} bootstrap from {} MB storage after restart",
            legacy_release.version(),
            storage_size
        ))
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    let temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();
    jormungandr.stop();

    let _jormungandr = JormungandrBootstrapper::default()
        .with_legacy_node_config(test_context.legacy_node_config.clone())
        .with_block0_configuration(test_context.block0_config())
        .into_starter(temp_dir)
        .unwrap()
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
