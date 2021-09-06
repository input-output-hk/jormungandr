use crate::mjolnir_app::build_monitor;
use crate::mjolnir_app::MjolnirError;
use jormungandr_integration_tests::common::startup;
use jormungandr_lib::{crypto::hash::Hash, interfaces::BlockDate};
use jormungandr_testing_utils::{
    testing::{
        node::time, FragmentGenerator, FragmentSender, FragmentSenderSetup, FragmentStatusProvider,
        RemoteJormungandrBuilder,
    },
    wallet::Wallet,
};
use jortestkit::prelude::parse_progress_bar_mode_from_str;
use jortestkit::{load::Configuration, prelude::ProgressBarMode};
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct AllFragments {
    /// Number of threads
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: usize,

    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub endpoint: String,

    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "e", long = "explorer")]
    pub explorer_endpoint: String,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "p", long = "pace", default_value = "2")]
    pub pace: u64,

    /// amount of delay [seconds] between sync attempts
    #[structopt(short = "d", long = "duration")]
    pub duration: u64,

    // show progress
    #[structopt(
        long = "progress-bar-mode",
        short = "b",
        default_value = "Monitor",
        parse(from_str = parse_progress_bar_mode_from_str)
    )]
    progress_bar_mode: ProgressBarMode,

    #[structopt(short = "m", long = "measure")]
    pub measure: bool,

    #[structopt(long = "key", short = "k")]
    faucet_key_file: PathBuf,

    #[structopt(long = "spending-counter", short = "s")]
    faucet_spending_counter: u32,

    /// load test rump up period
    #[structopt(long = "rump-up")]
    rump_up: u32,

    /// Transaction validity deadline (inclusive)
    #[structopt(long, short, default_value = "1.0")]
    valid_until: BlockDate,
}

impl AllFragments {
    pub fn exec(&self) -> Result<(), MjolnirError> {
        let title = "all fragment load test";
        let faucet =
            Wallet::import_account(&self.faucet_key_file, Some(self.faucet_spending_counter));
        let receiver = startup::create_new_account_address();
        let mut builder = RemoteJormungandrBuilder::new("node".to_string());
        builder.with_rest(self.endpoint.parse().unwrap());
        let remote_jormungandr = builder.build();

        let rest = remote_jormungandr.rest().clone();
        let settings = rest.settings().unwrap();

        let block0_hash = Hash::from_str(&settings.block0_hash).unwrap();
        let fees = settings.fees;

        let fragment_sender = FragmentSender::new(
            block0_hash,
            fees,
            chain_impl_mockchain::block::BlockDate {
                epoch: self.valid_until.epoch(),
                slot_id: self.valid_until.slot(),
            },
            FragmentSenderSetup::no_verify(),
        );

        let mut generator = FragmentGenerator::new(
            faucet,
            receiver,
            remote_jormungandr,
            settings.slots_per_epoch,
            30,
            30,
            30,
            fragment_sender,
        );

        let current_date = BlockDate::from_str(
            rest.stats()
                .unwrap()
                .stats
                .unwrap()
                .last_block_date
                .unwrap()
                .as_ref(),
        )
        .unwrap();

        let target_date = current_date.shift_slot(
            self.rump_up,
            &current_date.time_era(settings.slots_per_epoch),
        );

        generator.prepare(target_date);

        time::wait_for_date(target_date, rest);

        let config = Configuration::duration(
            self.count,
            std::time::Duration::from_secs(self.duration),
            self.pace,
            build_monitor(&self.progress_bar_mode),
            30,
            1,
        );
        let mut builder = RemoteJormungandrBuilder::new("node".to_string());
        builder.with_rest(self.endpoint.parse().unwrap());
        let remote_jormungandr = builder.build();
        let fragment_status_provider = FragmentStatusProvider::new(remote_jormungandr);

        let stats =
            jortestkit::load::start_async(generator, fragment_status_provider, config, title);
        stats.print_summary(title);

        Ok(())
    }
}
