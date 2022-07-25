use crate::jcli::command::votes::TallyCommand;
use assert_cmd::assert::OutputAssertExt;
use jcli_lib::vote::MergedVotePlan;
use jortestkit::prelude::ProcessOutput;
use std::path::Path;

pub struct Tally {
    tally_command: TallyCommand,
}

impl Tally {
    pub fn new(tally_command: TallyCommand) -> Self {
        Self { tally_command }
    }

    pub fn decryption_shares<P: AsRef<Path>, Q: AsRef<Path>, S: Into<String>>(
        self,
        vote_plan: Q,
        vote_plan_id: S,
        member_key: P,
    ) -> String {
        self.tally_command
            .decryption_shares(vote_plan, vote_plan_id, member_key)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn decrypt_results<P: AsRef<Path>, R: AsRef<Path>, S: Into<String>>(
        self,
        vote_plan: P,
        vote_plan_id: S,
        shares: R,
        threshold: u32,
    ) -> String {
        self.tally_command
            .decrypt_results(vote_plan, vote_plan_id, shares, threshold)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }

    pub fn decrypt_results_expect_fail<P: AsRef<Path>, R: AsRef<Path>, S: Into<String>>(
        self,
        vote_plan: P,
        vote_plan_id: S,
        shares: R,
        threshold: u32,
        expected_msg: &str,
    ) {
        self.tally_command
            .decrypt_results(vote_plan, vote_plan_id, shares, threshold)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn merge_shares<P: AsRef<Path>>(self, shares_to_merge: Vec<P>) -> String {
        self.tally_command
            .merge_shares(shares_to_merge)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }

    pub fn merge_results<P: AsRef<Path>>(
        self,
        vote_plans: P,
    ) -> Result<Vec<MergedVotePlan>, serde_json::Error> {
        serde_json::from_str(
            &self
                .tally_command
                .merge_results(vote_plans)
                .build()
                .assert()
                .success()
                .get_output()
                .as_lossy_string(),
        )
    }
}
