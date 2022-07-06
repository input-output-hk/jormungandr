use std::{path::Path, process::Command};
pub struct TallyCommand {
    command: Command,
}

impl TallyCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn decryption_shares<P: AsRef<Path>, Q: AsRef<Path>, S: Into<String>>(
        mut self,
        vote_plan: Q,
        vote_plan_id: S,
        member_key: P,
    ) -> Self {
        self.command
            .arg("decryption-shares")
            .arg("--vote-plan")
            .arg(vote_plan.as_ref())
            .arg("--vote-plan-id")
            .arg(vote_plan_id.into())
            .arg("--key")
            .arg(member_key.as_ref());
        self
    }

    pub fn decrypt_results<P: AsRef<Path>, R: AsRef<Path>, S: Into<String>>(
        mut self,
        vote_plan: P,
        vote_plan_id: S,
        shares: R,
        threshold: u32,
    ) -> Self {
        self.command
            .arg("decrypt-results")
            .arg("--vote-plan")
            .arg(vote_plan.as_ref())
            .arg("--vote-plan-id")
            .arg(vote_plan_id.into())
            .arg("--shares")
            .arg(shares.as_ref())
            .arg("--threshold")
            .arg(threshold.to_string())
            .arg("--output-format")
            .arg("json");
        self
    }

    pub fn merge_shares<P: AsRef<Path>>(mut self, shares: Vec<P>) -> Self {
        self.command.arg("merge-shares");

        for share in shares {
            self.command.arg(share.as_ref());
        }
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }

    pub fn merge_results<P: AsRef<Path>>(mut self, vote_plan_statuses: P) -> Self {
        self.command
            .arg("merge-results")
            .arg("--vote-plans")
            .arg(vote_plan_statuses.as_ref())
            .arg("--output-format")
            .arg("json");
        self
    }
}
