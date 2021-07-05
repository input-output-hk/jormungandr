use crate::mjolnir_app::build_monitor;
use crate::mjolnir_app::MjolnirError;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::testing::block0::get_block;
use jormungandr_testing_utils::testing::block0::Block0ConfigurationExtension;
use jormungandr_testing_utils::testing::{
    AdversaryVoteCastsGenerator, FragmentSender, FragmentStatusProvider,
};
use jormungandr_testing_utils::{
    testing::{FragmentSenderSetup, RemoteJormungandrBuilder},
    wallet::Wallet,
};
use jortestkit::prelude::parse_progress_bar_mode_from_str;
use jortestkit::{load::Configuration, prelude::ProgressBarMode};
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;
#[derive(StructOpt, Debug)]
pub struct VotesOnly {
    /// Number of threads
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: usize,

    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "a", long = "address")]
    pub endpoint: String,

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

    #[structopt(long = "block-path", short = "b")]
    block0_path: String,
}

impl VotesOnly {
    pub fn exec(&self) -> Result<(), MjolnirError> {
        let title = "adversary load transactions";
        let faucet = Wallet::import_account(
            self.faucet_key_file.clone(),
            Some(self.faucet_spending_counter),
        );
        let block0 = get_block(&self.block0_path)?;
        let vote_plans = block0.vote_plans();

        let mut builder = RemoteJormungandrBuilder::new("node".to_owned());
        builder.with_rest(self.endpoint.parse().unwrap());
        let remote_jormungandr = builder.build();

        let settings = remote_jormungandr.rest().settings().unwrap();

        let block0_hash = Hash::from_str(&settings.block0_hash).unwrap();
        let fees = settings.fees;

        let transaction_sender =
            FragmentSender::new(block0_hash, fees, FragmentSenderSetup::no_verify());

        let generator = AdversaryVoteCastsGenerator::new(
            faucet,
            vote_plans,
            remote_jormungandr.clone_with_rest(),
            transaction_sender,
        );

        let adversary_noise_config = Configuration::duration(
            self.count,
            std::time::Duration::from_secs(self.duration),
            self.pace,
            build_monitor(&self.progress_bar_mode),
            30,
            1,
        );

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
