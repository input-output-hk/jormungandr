use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::{fixture::PathChild, TempDir};
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder};
use jormungandr_lib::interfaces::{Mempool, PersistentLog};
pub use jortestkit::{
    console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode},
    load::{self, ConfigurationBuilder as LoadConfigurationBuilder, Monitor},
};
use mjolnir::generators::{BatchFragmentGenerator, FragmentStatusProvider};
use std::time::Duration;
use thor::{
    Block0ConfigurationBuilderExtension, BlockDateGenerator, FragmentSenderSetup,
    PersistentLogViewer,
};

#[test]
pub fn persistent_log_load_test() {
    let mut faucet = thor::Wallet::default();

    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("fragment_dump");

    let jormungandr = SingleNodeTestBootstrapper::default()
        .with_block0_config(
            Block0ConfigurationBuilder::default()
                .with_wallets_having_some_values(vec![&faucet])
                .with_slots_per_epoch(60.try_into().unwrap())
                .with_slot_duration(1.try_into().unwrap()),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: Some(PersistentLog {
                dir: persistent_log_path.path().to_path_buf(),
            }),
        }))
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();

    let batch_size = 10;
    let requests_per_thread = 50;
    let threads_count = 1;

    let configuration = LoadConfigurationBuilder::requests_per_thread(requests_per_thread)
        .thread_no(threads_count)
        .step_delay(Duration::from_secs(1))
        .monitor(Monitor::Standard(100))
        .shutdown_grace_period(Duration::from_secs(1))
        .status_pace(Duration::from_millis(30))
        .build();

    let settings = jormungandr.rest().settings().unwrap();

    let mut request_generator = BatchFragmentGenerator::from_node_with_setup(
        FragmentSenderSetup::no_verify(),
        &jormungandr,
        BlockDateGenerator::rolling(
            &settings,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            false,
        ),
        batch_size,
    )
    .unwrap();
    request_generator.fill_from_faucet(&mut faucet);

    let base_fragment_count = jormungandr.rest().fragment_logs().unwrap().len();

    load::start_async(
        request_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );

    let persistent_log_viewer = PersistentLogViewer::new(persistent_log_path.path().to_path_buf());
    assert_eq!(
        base_fragment_count
            + (batch_size as usize) * (requests_per_thread as usize) * threads_count,
        persistent_log_viewer.get_all().len()
    );
}
