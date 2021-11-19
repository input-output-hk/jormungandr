use crate::testing::jormungandr::TestingDirectory;
use crate::testing::network::VotePlanKey;
use crate::testing::network::WalletAlias;
use crate::testing::{
    jormungandr::starter::{Starter, StartupError},
    jormungandr::JormungandrProcess,
    node::LogLevel,
};
use crate::{
    testing::{
        network::{
            NodeAlias, NodeSetting, PersistenceMode, Settings, SpawnParams,
            Wallet as WalletSettings,
        },
        JormungandrParams,
    },
    wallet::Wallet,
};
use assert_fs::fixture::FixtureError;
use assert_fs::prelude::*;
use chain_impl_mockchain::certificate::{VoteAction, VotePlan};
use chain_impl_mockchain::ledger::governance::{
    ParametersGovernanceAction, TreasuryGovernanceAction,
};
use chain_impl_mockchain::testing::scenario::template::{
    ProposalDefBuilder, VotePlanDef, VotePlanDefBuilder,
};
use jormungandr_lib::interfaces::{Log, LogEntry, LogOutput, NodeConfig};
use std::path::PathBuf;
use thiserror::Error;

const NODE_CONFIG_FILE: &str = "node_config.yaml";
const NODE_SECRETS_FILE: &str = "node_secret.yaml";
const NODE_TOPOLOGY_KEY_FILE: &str = "node_topology_key";

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("node not found {0}")]
    NodeNotFound(String),
    #[error("wallet not found {0}")]
    WalletNotFound(String),
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("fixture filesystem error")]
    FsFixture(#[from] FixtureError),
    #[error("serialization error")]
    Serialization(#[from] serde_yaml::Error),
    #[error("node startup error")]
    Spawn(#[from] StartupError),
    #[error("VotePlan '{0}' was not found. Used before or never initialize")]
    VotePlanNotFound(String),
}

pub struct Controller {
    settings: Settings,
    working_directory: TestingDirectory,
    block0_file: PathBuf,
}

impl Controller {
    pub fn new(
        settings: Settings,
        working_directory: TestingDirectory,
    ) -> Result<Self, ControllerError> {
        use chain_core::property::Serialize as _;

        let block0_file = working_directory.child("block0.bin").path().into();
        let file = std::fs::File::create(&block0_file)?;
        settings.block0.to_block().serialize(file)?;

        Ok(Controller {
            settings,
            working_directory,
            block0_file,
        })
    }

    pub fn wallet(&mut self, wallet: &str) -> Result<Wallet, ControllerError> {
        if let Some(wallet) = self.settings.wallets.remove(wallet) {
            Ok(wallet.into())
        } else {
            Err(ControllerError::WalletNotFound(wallet.to_owned()))
        }
    }

    pub fn working_directory(&self) -> &TestingDirectory {
        &self.working_directory
    }

    pub fn block0_file(&self) -> PathBuf {
        self.block0_file.to_path_buf()
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn node_config(&self, alias: &str) -> Result<NodeConfig, ControllerError> {
        Ok(self.node_settings(alias)?.config.clone())
    }

    pub fn node_settings(&self, alias: &str) -> Result<&NodeSetting, ControllerError> {
        self.settings
            .nodes
            .get(alias)
            .ok_or_else(|| ControllerError::NodeNotFound(alias.to_string()))
    }

    pub fn defined_wallets(&self) -> impl Iterator<Item = (&WalletAlias, &WalletSettings)> {
        self.settings().wallets.iter()
    }

    pub fn defined_nodes(&self) -> impl Iterator<Item = (&NodeAlias, &NodeSetting)> {
        self.settings().nodes.iter()
    }

    pub fn defined_vote_plan(&self, alias: &str) -> Result<VotePlanDef, ControllerError> {
        if let Some((key, vote_plan)) = self
            .settings()
            .vote_plans
            .iter()
            .find(|(x, _y)| x.alias == alias)
        {
            Ok(self.convert_to_def(key, &vote_plan.vote_plan().into()))
        } else {
            Err(ControllerError::VotePlanNotFound(alias.to_owned()))
        }
    }

    pub fn defined_vote_plans(&self) -> Vec<VotePlanDef> {
        self.settings()
            .vote_plans
            .iter()
            .map(|(x, y)| self.convert_to_def(x, &y.vote_plan().into()))
            .collect()
    }

    fn convert_to_def(&self, key: &VotePlanKey, vote_plan: &VotePlan) -> VotePlanDef {
        let mut builder = VotePlanDefBuilder::new(&key.alias);
        builder
            .owner(&key.owner_alias)
            .payload_type(vote_plan.payload_type())
            .committee_keys(vote_plan.committee_public_keys().to_vec())
            .vote_phases(
                vote_plan.vote_start().epoch,
                vote_plan.committee_start().epoch,
                vote_plan.committee_end().epoch,
            );

        for proposal in vote_plan.proposals().iter() {
            let mut proposal_builder = ProposalDefBuilder::new(proposal.external_id().clone());

            let length = proposal
                .options()
                .choice_range()
                .end
                .checked_sub(proposal.options().choice_range().start)
                .unwrap();

            proposal_builder.options(length);

            match proposal.action() {
                VoteAction::OffChain => {
                    proposal_builder.action_off_chain();
                }
                VoteAction::Treasury { action } => match action {
                    TreasuryGovernanceAction::TransferToRewards { value } => {
                        proposal_builder.action_rewards_add(value.0);
                    }
                    TreasuryGovernanceAction::NoOp => {
                        unimplemented!();
                    }
                },
                VoteAction::Parameters { action } => match action {
                    ParametersGovernanceAction::RewardAdd { value } => {
                        proposal_builder.action_transfer_to_rewards(value.0);
                    }
                    ParametersGovernanceAction::NoOp => {
                        proposal_builder.action_parameters_no_op();
                    }
                },
            };

            builder.with_proposal(&mut proposal_builder);
        }
        builder.build()
    }

    pub fn spawn_node_async(&mut self, alias: &str) -> Result<JormungandrProcess, ControllerError> {
        let mut starter = self.make_starter_for(
            SpawnParams::new(alias).persistence_mode(PersistenceMode::InMemory),
        )?;
        let process = starter.start_async()?;
        Ok(process)
    }

    pub fn expect_spawn_failed(
        &mut self,
        spawn_params: SpawnParams,
        expected_msg: &str,
    ) -> Result<(), ControllerError> {
        let mut starter = self.make_starter_for(spawn_params)?;
        starter.start_with_fail_in_logs(expected_msg)?;
        Ok(())
    }

    pub fn spawn(
        &mut self,
        spawn_params: SpawnParams,
    ) -> Result<JormungandrProcess, ControllerError> {
        Ok(self.make_starter_for(spawn_params)?.start()?)
    }

    pub fn make_starter_for(
        &mut self,
        mut spawn_params: SpawnParams,
    ) -> Result<Starter, ControllerError> {
        let node_key_file = self
            .working_directory
            .child(spawn_params.get_alias())
            .child(NODE_TOPOLOGY_KEY_FILE)
            .path()
            .into();

        spawn_params = spawn_params.node_key_file(node_key_file);

        let node_setting = self.node_settings(spawn_params.get_alias())?;
        let dir = self.working_directory.child(spawn_params.get_alias());
        let mut config = node_setting.config.clone();
        spawn_params.override_settings(&mut config);

        for peer in config.p2p.trusted_peers.iter_mut() {
            peer.id = None;
        }

        let log_file_path = dir.child("node.log").path().to_path_buf();
        config.log = Some(Log(LogEntry {
            format: "json".into(),
            level: spawn_params
                .get_log_level()
                .unwrap_or(&LogLevel::DEBUG)
                .to_string(),
            output: LogOutput::Stdout,
        }));

        if let PersistenceMode::Persistent = spawn_params.get_persistence_mode() {
            let path_to_storage = dir.child("storage").path().into();
            config.storage = Some(path_to_storage);
        }
        dir.create_dir_all()?;

        let config_file = dir.child(NODE_CONFIG_FILE);
        let yaml = serde_yaml::to_string(&config)?;
        config_file.write_str(&yaml)?;

        let secret_file = dir.child(NODE_SECRETS_FILE);
        let yaml = serde_yaml::to_string(&node_setting.secret)?;
        secret_file.write_str(&yaml)?;

        let topology_file = dir.child(NODE_TOPOLOGY_KEY_FILE);
        topology_file.write_str(&node_setting.topology_secret.to_bech32_str())?;

        let params = JormungandrParams::new(
            config,
            config_file.path(),
            &self.block0_file,
            self.settings.block0.to_block().header().hash().to_string(),
            secret_file.path(),
            self.settings.block0.clone(),
            false,
            log_file_path,
        );

        let mut starter = Starter::new();
        starter
            .config(params)
            .verbose(spawn_params.get_verbose())
            .alias(spawn_params.get_alias().clone())
            .from_genesis(spawn_params.get_leadership_mode().into())
            .leadership_mode(spawn_params.get_leadership_mode());
        Ok(starter)
    }
}
