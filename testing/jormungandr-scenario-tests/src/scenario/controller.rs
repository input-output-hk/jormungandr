use crate::scenario::settings::Settings;
use crate::{
    legacy::{LegacyNode, LegacyNodeController},
    prepare_command,
    scenario::{
        settings::{Dotifier, PrepareSettings},
        ContextChaCha, Error, ProgressBarMode, Result,
    },
    style, Node, NodeBlock0, NodeController,
};
use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate::{VoteAction, VotePlan};
use chain_impl_mockchain::header::HeaderId;
use chain_impl_mockchain::ledger::governance::{
    ParametersGovernanceAction, TreasuryGovernanceAction,
};
use chain_impl_mockchain::testing::scenario::template::{
    ProposalDefBuilder, VotePlanDef, VotePlanDefBuilder,
};
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        benchmark_consumption,
        fragments::DummySyncNode,
        network_builder::{
            Blockchain, LeadershipMode, NodeAlias, NodeSetting, PersistenceMode, SpawnParams,
            Topology, Wallet as WalletSetting, WalletAlias,
        },
        ConsumptionBenchmarkRun, FragmentSender, FragmentSenderSetup, FragmentSenderSetupBuilder,
        SyncNode,
    },
    wallet::Wallet,
    Version,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct ControllerBuilder {
    title: String,
    controller_progress: ProgressBar,

    topology: Option<Topology>,
    blockchain: Option<Blockchain>,
    settings: Option<Settings>,
}

pub struct Controller {
    settings: Settings,

    context: ContextChaCha,

    working_directory: ChildPath,

    block0_file: PathBuf,
    block0_hash: HeaderId,

    progress_bar: Arc<MultiProgress>,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,

    topology: Topology,
    blockchain: Blockchain,
}

impl ControllerBuilder {
    pub fn new(title: &str) -> Self {
        let controller_progress = ProgressBar::new(10);
        controller_progress.set_prefix(&format!("{} {}", *style::icons::scenario, title));
        controller_progress.set_message("building...");
        controller_progress.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {prefix:.bold.dim} [{bar:10.cyan/blue}] [{elapsed_precise}] {wide_msg}")
                .tick_chars(style::TICKER)
        );
        controller_progress.enable_steady_tick(100);

        ControllerBuilder {
            title: title.to_owned(),
            controller_progress,
            topology: None,
            blockchain: None,
            settings: None,
        }
    }

    pub fn set_topology(&mut self, topology: Topology) {
        self.controller_progress.inc(1);
        self.topology = Some(topology)
    }

    pub fn set_blockchain(&mut self, blockchain: Blockchain) {
        self.controller_progress.inc(1);
        self.blockchain = Some(blockchain)
    }

    pub fn build_settings(&mut self, context: &mut ContextChaCha) {
        self.controller_progress.inc(1);
        let topology = self.topology.clone().expect("topology not set");
        let blockchain = self.blockchain.clone().expect("blockchain not defined");
        self.settings = Some(Settings::prepare(topology, blockchain, context));
        self.controller_progress.inc(5);
    }

    pub fn build(self, context: ContextChaCha) -> Result<Controller> {
        let working_directory = context.child_directory(&self.title);
        working_directory.create_dir_all()?;
        if context.generate_documentation() {
            self.document(working_directory.path())?;
        }
        self.controller_progress.finish_and_clear();
        self.summary();

        match context.progress_bar_mode() {
            ProgressBarMode::None => println!("nodes logging disabled"),
            ProgressBarMode::Standard => {
                println!("nodes monitoring disabled due to legacy logging setting enabled")
            }
            _ => (),
        }

        let settings = self.settings.unwrap();

        Controller::new(
            settings,
            context,
            working_directory,
            self.blockchain.unwrap(),
            self.topology.unwrap(),
        )
    }

    fn summary(&self) {
        println!(
            r###"
# Running {title}
        "###,
            title = style::scenario_title.apply_to(&self.title)
        )
    }

    fn document(&self, path: &Path) -> Result<()> {
        if let Some(settings) = &self.settings {
            let file = std::fs::File::create(&path.join("initial_setup.dot"))?;

            let dotifier = Dotifier;
            dotifier.dottify(settings, file)?;

            for wallet in settings.network_settings.wallets.values() {
                wallet.save_to(path)?;
            }

            let file = std::fs::File::create(&path.join("genesis.yaml"))?;
            serde_yaml::to_writer(file, &settings.network_settings.block0).unwrap();
        }

        Ok(())
    }
}

impl Controller {
    fn new(
        settings: Settings,
        context: ContextChaCha,
        working_directory: ChildPath,
        blockchain: Blockchain,
        topology: Topology,
    ) -> Result<Self> {
        use chain_core::property::Serialize as _;

        let block0 = settings.network_settings.block0.to_block();
        let block0_hash = block0.header.hash();

        let block0_file = working_directory.child("block0.bin").path().into();
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Controller {
            settings,
            context,
            block0_file,
            block0_hash,
            progress_bar,
            progress_bar_thread: None,
            working_directory,
            blockchain,
            topology,
        })
    }

    pub fn stake_pool(&mut self, node_alias: &str) -> Result<StakePool> {
        if let Some(stake_pool) = self.settings.network_settings.stake_pools.get(node_alias) {
            Ok(stake_pool.clone())
        } else {
            Err(Error::StakePoolNotFound(node_alias.to_owned()))
        }
    }

    pub fn working_directory(&self) -> &ChildPath {
        &self.working_directory
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&NodeAlias, &NodeSetting)> {
        self.settings.network_settings.nodes.iter()
    }

    pub fn vote_plan(&self, alias: &str) -> Result<VotePlanDef> {
        if let Some(vote_plan) = self.settings.network_settings.vote_plans.get(alias) {
            Ok(self.convert_to_def(alias, vote_plan))
        } else {
            Err(Error::VotePlanNotFound(alias.to_owned()))
        }
    }

    pub fn vote_plans(&self) -> Vec<VotePlanDef> {
        self.settings
            .network_settings
            .vote_plans
            .iter()
            .map(|(x, y)| self.convert_to_def(x, y))
            .collect()
    }

    fn convert_to_def(&self, alias: &str, vote_plan: &VotePlan) -> VotePlanDef {
        let templates = self.blockchain.vote_plans();
        let template = templates.iter().find(|x| x.alias() == alias).unwrap();
        let mut builder = VotePlanDefBuilder::new(alias);
        builder
            .owner(&template.owner())
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

    pub fn wallets(&self) -> impl Iterator<Item = (&WalletAlias, &WalletSetting)> {
        self.settings.network_settings.wallets.iter()
    }

    pub fn get_all_wallets(&mut self) -> Vec<Wallet> {
        let mut wallets = vec![];

        for alias in self.settings.network_settings.wallets.clone().keys() {
            wallets.push(self.wallet(alias).unwrap());
        }
        wallets
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn context(&self) -> &ContextChaCha {
        &self.context
    }

    pub fn add_to_progress_bar(&mut self, pb: ProgressBar) -> ProgressBar {
        self.progress_bar.add(pb)
    }

    pub fn block0_file(&self) -> PathBuf {
        self.block0_file.clone()
    }

    pub fn start_monitor_resources(
        &mut self,
        info: &str,
        nodes: Vec<&NodeController>,
    ) -> ConsumptionBenchmarkRun {
        benchmark_consumption(info.to_owned())
            .for_processes(nodes.iter().map(|x| x.as_named_process()).collect())
            .bare_metal_stake_pool_consumption_target()
            .start()
    }

    pub fn wallet(&self, wallet: &str) -> Result<Wallet> {
        if let Some(wallet) = self.settings.network_settings.wallets.get(wallet) {
            Ok(wallet.clone().into())
        } else {
            Err(Error::WalletNotFound(wallet.to_owned()))
        }
    }

    pub fn new_spawn_params(&self, node_alias: &str) -> SpawnParams {
        let mut spawn_params = SpawnParams::new(node_alias);
        spawn_params.node_key_file(self.node_dir(node_alias).path().into());
        spawn_params
    }

    fn node_dir(&self, alias: &str) -> ChildPath {
        self.working_directory.child(alias)
    }

    pub fn spawn_legacy_node(
        &mut self,
        params: &mut SpawnParams,
        version: &Version,
    ) -> Result<LegacyNodeController> {
        let node_setting = if let Some(node_setting) = self
            .settings
            .network_settings
            .nodes
            .get(&params.get_alias())
        {
            node_setting
        } else {
            return Err(Error::NodeNotFound(params.get_alias()));
        };

        let mut node_setting_overriden = node_setting.clone();
        params.override_settings(&mut node_setting_overriden.config);

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0_hash),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(&jormungandr),
            None => self.context.jormungandr().to_path_buf(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut spawn_builder = LegacyNode::spawn(&self.context, &mut node_setting_overriden);
        spawn_builder
            .path_to_jormungandr(jormungandr)
            .progress_bar(pb)
            .alias(params.get_alias())
            .block0(block0_setting)
            .working_dir(self.node_dir(&params.get_alias()).path())
            .peristence_mode(params.get_persistence_mode());
        let node = spawn_builder.build(version)?;
        Ok(node.controller())
    }

    pub fn spawn_node_custom(&mut self, params: &mut SpawnParams) -> Result<NodeController> {
        let node_setting = if let Some(node_setting) = self
            .settings
            .network_settings
            .nodes
            .get(&params.get_alias())
        {
            node_setting
        } else {
            return Err(Error::NodeNotFound(params.get_alias()));
        };
        let mut node_setting_overriden = node_setting.clone();
        params.override_settings(&mut node_setting_overriden.config);

        // remove all id from trusted peers for current version
        for trusted_peer in node_setting_overriden.config.p2p.trusted_peers.iter_mut() {
            trusted_peer.id = None;
        }

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0_hash),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(&jormungandr),
            None => self.context.jormungandr().to_path_buf(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut spawn_builder = Node::spawn(&self.context, &mut node_setting_overriden);
        spawn_builder
            .path_to_jormungandr(jormungandr)
            .progress_bar(pb)
            .alias(params.get_alias())
            .block0(block0_setting)
            .working_dir(self.node_dir(&params.get_alias()).path())
            .peristence_mode(params.get_persistence_mode());
        if let Some(faketime) = params.faketime.take() {
            spawn_builder.faketime(faketime);
        }
        let node = spawn_builder.build()?;

        Ok(node.controller())
    }

    pub fn spawn_node(
        &mut self,
        node_alias: &str,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<NodeController> {
        let mut params = self.new_spawn_params(node_alias);
        params.leadership_mode(leadership_mode);
        params.persistence_mode(persistence_mode);
        self.spawn_node_custom(&mut params)
    }

    pub fn restart_node(
        &mut self,
        node: NodeController,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<NodeController> {
        node.shutdown()?;
        let new_node = self.spawn_node(node.alias(), leadership_mode, persistence_mode)?;
        new_node.wait_for_bootstrap()?;
        Ok(new_node)
    }

    pub fn monitor_nodes(&mut self) {
        if let ProgressBarMode::Monitor = self.context.progress_bar_mode() {
            let pb = Arc::clone(&self.progress_bar);
            self.progress_bar_thread = Some(std::thread::spawn(move || {
                pb.join().unwrap();
            }));
        }
    }

    pub fn finalize(self) {
        if let Some(thread) = self.progress_bar_thread {
            thread.join().unwrap()
        }
    }

    pub fn fragment_sender(&self) -> FragmentSender<DummySyncNode> {
        self.fragment_sender_with_setup(FragmentSenderSetup::default())
    }

    pub fn fragment_sender_with_setup<'a, S: SyncNode + Send>(
        &self,
        setup: FragmentSenderSetup<'a, S>,
    ) -> FragmentSender<'a, S> {
        let mut builder = FragmentSenderSetupBuilder::from(setup);
        let root_dir: PathBuf = PathBuf::from(self.working_directory().path());
        builder.dump_fragments_into(root_dir.join("fragments"));
        let hash = Hash::from_hash(self.block0_hash);

        FragmentSender::new(
            hash,
            self.settings
                .network_settings
                .block0
                .blockchain_configuration
                .linear_fees,
            BlockDate::first(),
            builder.build(),
        )
    }
}
