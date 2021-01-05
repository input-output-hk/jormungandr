use std::{path::Path, process::Command};
pub struct TallyCommand {
    command: Command,
}

impl TallyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn generate_decryption_share<P: AsRef<Path>, Q: AsRef<Path>>(
        mut self,
        decryption_key: P,
        encrypted_tally: Q,
    ) -> Self {
        self.command
            .arg("decryption-share")
            .arg("--key")
            .arg(decryption_key.as_ref())
            .arg("--tally")
            .arg(encrypted_tally.as_ref());
        self
    }

    pub fn decrypt_with_shares<P: AsRef<Path>, R: AsRef<Path>>(
        mut self,
        encrypted_tally: P,
        max_votes: u32,
        shares: R,
        tablesize: u32,
        threshold: u32,
    ) -> Self {
        self.command
            .arg("decrypt")
            .arg("--tally")
            .arg(encrypted_tally.as_ref())
            .arg("--max-votes")
            .arg(max_votes.to_string())
            .arg("--shares")
            .arg(shares.as_ref())
            .arg("--table-size")
            .arg(tablesize.to_string())
            .arg("--threshold")
            .arg(threshold.to_string());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
