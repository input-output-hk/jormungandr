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
            .as_single_line()
    }

    pub fn decrypt_with_shares<P: AsRef<Path>, R: AsRef<Path>>(
        self,
        encrypted_tally: P,
        max_votes: u64,
        shares: R,
        threshold: u32,
    ) -> String {
        self.tally_command
            .decrypt_with_shares(encrypted_tally, max_votes, shares, threshold)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }
}
