use super::Error;
use crate::{
    jcli::{JCli, JCliCommand},
    jormungandr::JormungandrProcess,
};
use jormungandr_lib::interfaces::FragmentLog;
use jortestkit::{prelude::*, process::run_process_until_response_matches};
use std::process::Command;

pub struct FragmentsCheck<'a> {
    jcli: JCli,
    jormungandr: &'a JormungandrProcess,
}

impl<'a> FragmentsCheck<'a> {
    pub fn new(jcli: JCli, jormungandr: &'a JormungandrProcess) -> Self {
        Self { jcli, jormungandr }
    }

    pub fn wait_until_in_block(&self) -> Result<(), Error> {
        self.wait_until_all_processed(&Default::default())?;
        self.check_log_shows_in_block()
    }

    pub fn wait_until_all_processed(&self, wait: &Wait) -> Result<(), Error> {
        run_process_until_response_matches(
            JCliCommand::new(Command::new(self.jcli.path()))
                .rest()
                .v0()
                .message()
                .logs(self.jormungandr.rest_uri())
                .build(),
            |output| {
                let content = output.as_lossy_string();
                let fragments: Vec<FragmentLog> =
                    serde_yaml::from_str(&content).expect("Cannot parse fragment logs");
                let at_least_one_pending = fragments.iter().any(|x| x.is_pending());
                !at_least_one_pending
            },
            wait.sleep_duration().as_secs(),
            wait.attempts(),
            "Waiting for last transaction to be inBlock or rejected",
            "transaction is pending for too long",
        )
        .map_err(|_| Error::TransactionsNotInBlock {
            message_log: format!(
                "{:?}",
                self.jcli
                    .clone()
                    .rest()
                    .v0()
                    .message()
                    .logs(self.jormungandr.rest_uri())
            ),
            log_content: self.jormungandr.logger.get_log_content(),
        })
    }

    pub fn check_log_shows_in_block(&self) -> Result<(), Error> {
        let fragments = self
            .jcli
            .rest()
            .v0()
            .message()
            .logs(self.jormungandr.rest_uri());
        for fragment in fragments.iter() {
            if !fragment.is_in_a_block() {
                return Err(Error::TransactionNotInBlock {
                    message_log: format!("{:?}", fragments.clone()),
                    transaction_id: *fragment.fragment_id(),
                    log_content: self.jormungandr.logger.get_log_content(),
                });
            }
        }
        Ok(())
    }
}
