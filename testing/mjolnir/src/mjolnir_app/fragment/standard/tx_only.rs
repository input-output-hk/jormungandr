use crate::mjolnir_app::build_monitor;
use crate::mjolnir_app::MjolnirError;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::{
    testing::{fragments::TransactionGenerator, FragmentSenderSetup, RemoteJormungandrBuilder},
    wallet::Wallet,
};
use jortestkit::load::Configuration;
use jortestkit::prelude::parse_progress_bar_mode_from_str;
use jortestkit::prelude::ProgressBarMode;
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;
#[derive(StructOpt, Debug)]
pub struct TxOnly {
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

    /// Transaction validity deadline (inclusive)
    #[structopt(long, short, default_value = "1.0")]
    valid_until: BlockDate,
}

impl TxOnly {
    pub fn exec(&self) -> Result<(), MjolnirError> {
        let title = "standard load only transactions";
        let mut faucet = Wallet::import_account(
            self.faucet_key_file.clone(),
            Some(self.faucet_spending_counter),
        );
        let mut builder = RemoteJormungandrBuilder::new("node".to_owned());
        builder.with_rest(self.endpoint.parse().unwrap());
        let remote_jormungandr = builder.build();

        let settings = remote_jormungandr.rest().settings().unwrap();

        let block0_hash = Hash::from_str(&settings.block0_hash).unwrap();
        let fees = settings.fees;

        let mut generator = TransactionGenerator::new(
            FragmentSenderSetup::no_verify(),
            remote_jormungandr,
            block0_hash,
            fees,
            self.valid_until,
        );
        generator.fill_from_faucet(&mut faucet);

        let config = Configuration::duration(
            self.count,
            std::time::Duration::from_secs(self.duration),
            self.pace,
            build_monitor(&self.progress_bar_mode),
            30,
            1,
        );
        let stats = jortestkit::load::start_sync(generator, config, title);
        stats.print_summary(title);
        Ok(())
    }
}
