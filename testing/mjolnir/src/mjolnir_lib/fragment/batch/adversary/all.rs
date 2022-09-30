use crate::{
    args::parse_shift,
    build_monitor,
    generators::{AdversaryFragmentGenerator, FragmentStatusProvider},
    MjolnirError,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_automation::jormungandr::RemoteJormungandrBuilder;
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::{
    load::ConfigurationBuilder,
    prelude::{parse_progress_bar_mode_from_str, ProgressBarMode},
};
use loki::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use std::{path::PathBuf, str::FromStr, time::Duration};
use structopt::StructOpt;
use thor::{
    BlockDateGenerator, DiscriminationExtension, FragmentSender, FragmentSenderSetup, Wallet,
};
#[derive(StructOpt, Debug)]
pub struct AdversaryAll {
    /// Number of threads
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: usize,

    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub endpoint: String,

    /// amount of delay [milliseconds] between sync attempts
    #[structopt(long = "delay", default_value = "50")]
    pub delay: u64,

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

    /// Transaction validity deadline (inclusive)
    #[structopt(short = "v", long = "valid-until", conflicts_with = "ttl")]
    valid_until: Option<BlockDate>,

    /// Transaction time to live (can be negative e.g. ~4.2)
    #[structopt(short = "t", long= "ttl", default_value = "1.0", parse(try_from_str = parse_shift))]
    ttl: (BlockDate, bool),

    /// Set the discrimination type to testing (default is production).
    #[structopt(long = "testing")]
    testing: bool,
}

impl AdversaryAll {
    pub fn exec(&self) -> Result<(), MjolnirError> {
        let title = "adversary load transactions";
        let mut faucet = Wallet::import_account(
            self.faucet_key_file.clone(),
            Some(self.faucet_spending_counter.into()),
            Discrimination::from_testing_bool(self.testing),
        );
        let remote_jormungandr = RemoteJormungandrBuilder::new("node".to_owned())
            .with_rest(self.endpoint.parse().unwrap())
            .build();

        let settings = remote_jormungandr.rest().settings().unwrap();

        let block0_hash = Hash::from_str(&settings.block0_hash).unwrap();
        let fees = settings.fees.clone();

        let expiry_generator = self
            .valid_until
            .map(BlockDateGenerator::Fixed)
            .unwrap_or_else(|| BlockDateGenerator::rolling(&settings, self.ttl.0, self.ttl.1));

        let transaction_sender = FragmentSender::new(
            block0_hash,
            fees.clone(),
            expiry_generator.clone(),
            FragmentSenderSetup::no_verify(),
        );

        let adversary_transaction_sender = AdversaryFragmentSender::new(
            block0_hash,
            fees,
            expiry_generator,
            AdversaryFragmentSenderSetup::no_verify(),
        );

        let mut generator = AdversaryFragmentGenerator::new(
            remote_jormungandr.clone_with_rest(),
            transaction_sender,
            adversary_transaction_sender,
        );
        generator.fill_from_faucet(&mut faucet);

        let adversary_noise_config =
            ConfigurationBuilder::duration(Duration::from_secs(self.duration))
                .thread_no(self.count)
                .step_delay(Duration::from_millis(self.delay))
                .monitor(build_monitor(&self.progress_bar_mode))
                .shutdown_grace_period(Duration::from_secs(30))
                .build();

        let noise_stats = jortestkit::load::start_background_async(
            generator,
            FragmentStatusProvider::new(remote_jormungandr),
            adversary_noise_config,
            "noise fragments",
        )
        .stats();

        noise_stats.print_summary(title);
        Ok(())
    }
}
