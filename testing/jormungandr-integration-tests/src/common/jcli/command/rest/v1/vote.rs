use std::process::Command;

pub struct VoteCommand {
    command: Command,
}

impl VoteCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn account_votes(
        mut self,
        account: impl Into<String>,
        voteplan: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        self.command
            .arg("account-votes")
            .arg("--host")
            .arg(host.into())
            .arg("--account")
            .arg(account.into())
            .arg("--vote-plan-id")
            .arg(voteplan.into());
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
