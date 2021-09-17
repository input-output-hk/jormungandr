use jormungandr_testing_utils::testing::jormungandr::ConfigurationBuilder;
use jormungandr_testing_utils::testing::startup;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::{Mempool, PersistentLog};
use jormungandr_testing_utils::testing::fragments::PersistentLogViewer;
use jormungandr_testing_utils::testing::{
    BatchFragmentGenerator, FragmentSenderSetup, FragmentStatusProvider,
};
pub use jortestkit::console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode};
use jortestkit::load::{self, Configuration, Monitor};

#[test]
pub fn persistent_log_load_test() {
    let mut faucet = startup::create_new_account_address();

    let temp_dir = TempDir::new().unwrap();
    let persistent_log_path = temp_dir.child("fragment_dump");

    let jormungandr = startup::start_bft(
        vec![&faucet],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_slot_duration(1)
            .with_explorer()
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: Some(PersistentLog {
                    dir: persistent_log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let batch_size = 100;
    let requests_per_thread = 50;
    let threads_count = 1;

    let configuration = Configuration::requests_per_thread(
        threads_count,
        requests_per_thread,
        1,
        Monitor::Standard(100),
        1,
        1,
    );

    let mut request_generator = BatchFragmentGenerator::new(
        FragmentSenderSetup::no_verify(),
        jormungandr.to_remote(),
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().into(),
        batch_size,
    );
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
