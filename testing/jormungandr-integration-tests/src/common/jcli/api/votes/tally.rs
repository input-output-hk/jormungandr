use crate::common::jcli::command::votes::TallyCommand;
use assert_cmd::assert::OutputAssertExt;
use jortestkit::prelude::ProcessOutput;
use std::path::Path;

pub struct Tally {
    tally_command: TallyCommand,
}

impl Tally {
    pub fn new(tally_command: TallyCommand) -> Self {
        Self { tally_command }
    }

    pub fn generate_decryption_share<P: AsRef<Path>, Q: AsRef<Path>>(
        self,
        decryption_key: P,
        encrypted_tally: Q,
    ) -> String {
        self.tally_command
            .generate_decryption_share(decryption_key, encrypted_tally)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }

    pub fn decrypt_with_shares<P: AsRef<Path>, R: AsRef<Path>>(
        self,
        encrypted_tally: P,
        vote_stake_limit: u64,
        shares: R,
        tablesize: u32,
        threshold: u32,
    ) -> String {
        self.tally_command
            .decrypt_with_shares(
                encrypted_tally,
                vote_stake_limit,
                shares,
                tablesize,
                threshold,
            )
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }
}
