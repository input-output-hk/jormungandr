use crate::mjolnir_app::MjolnirError;
use jormungandr_testing_utils::testing::node::{JormungandrRest, RestRequestGen};
use jortestkit::{
    load::Configuration,
    load::Monitor,
    prelude::{parse_progress_bar_mode_from_str, ProgressBarMode},
};
use structopt::StructOpt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestLoadCommandError {
    #[error("Client Error")]
    ClientError(#[from] MjolnirError),
}

#[derive(StructOpt, Debug)]
pub struct RestLoadCommand {
    /// Number of threads
    #[structopt(short = "c", long = "count", default_value = "3")]
    pub count: usize,
    /// address in format:
    /// http://127.0.0.1:8002/api/
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

impl RestLoadCommand {
    pub fn exec(&self) -> Result<(), RestLoadCommandError> {
        let mut rest_client = JormungandrRest::new(self.endpoint.clone());
        rest_client.disable_logger();
        let mut request_gen = RestRequestGen::new(rest_client);
        request_gen.do_setup(Vec::new()).unwrap();

        let config = Configuration::duration(
            self.count,
            std::time::Duration::from_secs(self.duration),
            self.pace,
            self.build_monitor(),
            0,
            1,
        );
        let stats = jortestkit::load::start_sync(request_gen, config, "rest load test");
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
