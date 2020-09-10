use jormungandr_integration_tests::common::load::ClientLoadError;
use jormungandr_testing_utils::testing::node::explorer::load::ExplorerRequestGen;
use jormungandr_testing_utils::testing::node::Explorer;
use jortestkit::load::Configuration;
use jortestkit::load::Monitor;
use jortestkit::prelude::parse_progress_bar_mode_from_str;
use jortestkit::prelude::ProgressBarMode;
use structopt::StructOpt;
use thiserror::Error;

pub fn main() -> Result<(), ExplorerLoadCommandError> {
    ExplorerLoadCommand::from_args().exec()
}

#[derive(Error, Debug)]
pub enum ExplorerLoadCommandError {
    #[error("No scenario defined for run. Available: [duration,iteration]")]
    NoScenarioDefined,
    #[error("Client Error")]
    ClientError(#[from] ClientLoadError),
}

#[derive(StructOpt, Debug)]
pub struct ExplorerLoadCommand {
    /// Prints nodes related data, like stats,fragments etc.
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: usize,
    /// address in format:
    /// /ip4/54.193.75.55/tcp/3000
    #[structopt(short = "e", long = "endpoint")]
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
}

impl ExplorerLoadCommand {
    pub fn exec(&self) -> Result<(), ExplorerLoadCommandError> {
        let mut explorer = Explorer::new(self.endpoint.clone());
        explorer.disable_logs();
        let mut request_gen = ExplorerRequestGen::new(explorer);
        request_gen.do_setup(Vec::new()).unwrap();

        let config = Configuration::duration(
            self.count,
            std::time::Duration::from_secs(self.duration),
            self.pace,
            self.build_monitor(),
            0,
        );
        let stats = jortestkit::load::start_sync(request_gen, config, "Explorer load test");
        if self.measure {
            assert!((stats.calculate_passrate() as u32) > 95);
        }
        Ok(())
    }

    fn build_monitor(&self) -> Monitor {
        match self.progress_bar_mode {
            ProgressBarMode::Monitor => Monitor::Progress(100),
            ProgressBarMode::Standard => Monitor::Standard(100),
            ProgressBarMode::None => Monitor::Disabled(10),
        }
    }
}
