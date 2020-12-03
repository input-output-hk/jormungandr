use assert_fs::TempDir;
use chain_impl_mockchain::certificate::VotePlan;
use chain_impl_mockchain::testing::scenario::template::VotePlanDef;
use std::path::PathBuf;
use vit_servicing_station_lib::db::models::vote_options::VoteOptions;
use vit_servicing_station_tests::common::data::Generator;
use vit_servicing_station_tests::common::startup::db::DbBuilder;

pub struct DbGenerator {
    vote_plans: Vec<VotePlanDef>,
}

impl DbGenerator {
    pub fn new(vote_plans: Vec<VotePlanDef>) -> Self {
        Self { vote_plans }
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_vote_plan(vote_plan_def: &VotePlanDef) -> VotePlan {
        vote_plan_def.clone().into()
    }

    pub fn build(self, db_file: &PathBuf) {
        std::fs::File::create(&db_file).unwrap();

        let snapshot = Generator::new().snapshot();
        let mut snapshot_proposals = snapshot.proposals();

        for (_, vote_plan) in self.vote_plans.iter().enumerate() {
            for (index, proposal) in vote_plan.proposals().iter().enumerate() {
                let mut proposal_snapshot = snapshot_proposals.get_mut(index).unwrap();

                proposal_snapshot.proposal_id = proposal.id().to_string();
                proposal_snapshot.chain_proposal_id = proposal.id().to_string().as_bytes().to_vec();
                proposal_snapshot.chain_proposal_index = index as i64;
                proposal_snapshot.chain_vote_options =
                    VoteOptions::parse_coma_separated_value("blank,yes,no");
                proposal_snapshot.chain_voteplan_id =
                    Self::to_vote_plan(&vote_plan).to_id().to_string();
            }
        }

        let path = std::path::Path::new(".").join("resources/vit_station/migration");

        let temp_dir = TempDir::new().unwrap().into_persistent();
        let temp_db_path = DbBuilder::new()
            .with_proposals(snapshot_proposals)
            .with_tokens(snapshot.tokens().values().cloned().collect())
            .with_migrations_from(std::fs::canonicalize(path).unwrap())
            .with_funds(snapshot.funds())
            .build(&temp_dir)
            .unwrap();

        jortestkit::file::copy_file(temp_db_path, db_file, true);
    }
}
