use crate::node::SpawnBuilder;
use crate::{
    legacy::LegacyNode,
    prepare_command,
    scenario::{dotifier::Dotifier, ContextChaCha, Error, ProgressBarMode, Result},
    style, Node, NodeBlock0,
};
use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate::{VoteAction, VotePlan};
use chain_impl_mockchain::ledger::governance::{
    ParametersGovernanceAction, TreasuryGovernanceAction,
};
use chain_impl_mockchain::testing::scenario::template::{
    ProposalDefBuilder, VotePlanDef, VotePlanDefBuilder,
};
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_lib::interfaces::Log;
use jormungandr_lib::interfaces::LogEntry;
use jormungandr_lib::interfaces::LogOutput;
use jormungandr_testing_utils::testing::network::Settings;
use jormungandr_testing_utils::testing::network::{
    builder::NetworkBuilder, controller::Controller as InnerController, VotePlanKey,
};
use jormungandr_testing_utils::testing::node::LogLevel;
use jormungandr_testing_utils::testing::utils::{Event, Observable, Observer};
use jormungandr_testing_utils::testing::BlockDateGenerator;
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        benchmark_consumption,
        fragments::DummySyncNode,
        network::{
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
    rc::Rc,
    sync::Arc,
};

pub struct ControllerBuilder {
    title: String,
    network_builder: NetworkBuilder,
}

pub struct Controller {
    inner: InnerController,

    context: ContextChaCha,

    working_directory: ChildPath,

    block0_file: PathBuf,
    block0: Block0Configuration,

    progress_bar: Arc<MultiProgress>,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,
}

impl ControllerBuilder {
    pub fn new(title: &str) -> Self {
        ControllerBuilder {
            title: title.to_owned(),
            network_builder: Default::default(),
        }
    }

    pub fn topology(mut self, topology: Topology) -> Self {
        self.network_builder = self.network_builder.topology(topology);
        self
    }

    pub fn blockchain(mut self, blockchain: Blockchain) -> Self {
        self.network_builder = self.network_builder.blockchain_config(blockchain);
        self
    }

    pub fn build(self, context: ContextChaCha) -> Result<Controller> {
        let working_directory = context.child_directory(&self.title);
        working_directory.create_dir_all()?;

        let observer: Rc<dyn Observer> = Rc::new(NetworkBuilderObserver::new(&self.title));
        let inner_controller = self.network_builder.register(&observer).build()?;
        if context.generate_documentation() {
            document(working_directory.path(), &inner_controller)?;
        }
        summary(&self.title);

        match context.progress_bar_mode() {
            ProgressBarMode::None => println!("nodes logging disabled"),
            ProgressBarMode::Standard => {
                println!("nodes monitoring disabled due to legacy logging setting enabled")
            }
            _ => (),
        }
        Controller::new(inner_controller, context, working_directory)
    }
}

struct NetworkBuilderObserver {
    controller_progress: ProgressBar,
}

impl NetworkBuilderObserver {
    pub fn new<S: Into<String>>(title: S) -> Self {
        let controller_progress = ProgressBar::new(3);
        controller_progress.set_prefix(&format!("{} {}", *style::icons::scenario, title.into()));
        controller_progress.set_message("building...");
        controller_progress.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {prefix:.bold.dim} [{bar:10.cyan/blue}] [{elapsed_precise}] {wide_msg}")
                .tick_chars(style::TICKER)
        );
        controller_progress.enable_steady_tick(250);
        Self {
            controller_progress,
        }
    }
}

impl Observer for NetworkBuilderObserver {
    fn notify(&self, event: Event) {
        self.controller_progress.inc(1);
        self.controller_progress.set_message(&event.message);
    }

    fn finished(&self) {
        self.controller_progress.finish_and_clear();
    }
}

fn summary(title: &str) {
    println!(
        r###"
# Running {title}
    "###,
        title = style::scenario_title.apply_to(title)
    )
}

fn document(path: &Path, inner: &InnerController) -> Result<()> {
    let file = std::fs::File::create(&path.join("initial_setup.dot"))?;

    let dotifier = Dotifier;
    dotifier.dottify(inner.settings(), file)?;

    for wallet in inner.settings().wallets.values() {
        wallet.save_to(path)?;
    }

    let file = std::fs::File::create(&path.join("genesis.yaml"))?;
    serde_yaml::to_writer(file, &inner.settings().block0).unwrap();

    Ok(())
}

impl Controller {
    fn new(
        controller: InnerController,
        context: ContextChaCha,
        working_directory: ChildPath,
    ) -> Result<Self> {
        use chain_core::property::Serialize as _;

        let block0 = controller.settings().block0.to_block();
        let block0_file = working_directory.child("block0.bin").path().into();
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Controller {
            block0: controller.settings().block0.clone(),
            inner: controller,
            context,
            block0_file,
            progress_bar,
            progress_bar_thread: None,
            working_directory,
        })
    }

    pub fn stake_pool(&mut self, node_alias: &str) -> Result<StakePool> {
        if let Some(stake_pool) = self.inner.settings().stake_pools.get(node_alias) {
            Ok(stake_pool.clone())
        } else {
            Err(Error::StakePoolNotFound(node_alias.to_owned()))
        }
    }

    pub fn working_directory(&self) -> &ChildPath {
        &self.working_directory
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&NodeAlias, &NodeSetting)> {
        self.inner.settings().nodes.iter()
    }

    pub fn vote_plan(&self, alias: &str) -> Result<VotePlanDef> {
        if let Some((key, vote_plan)) = self
            .inner
            .settings()
            .vote_plans
            .iter()
            .find(|(x, _y)| x.alias == alias)
        {
            Ok(self.convert_to_def(key, &vote_plan.vote_plan().into()))
        } else {
            Err(Error::VotePlanNotFound(alias.to_owned()))
        }
    }

    pub fn vote_plans(&self) -> Vec<VotePlanDef> {
        self.inner
            .settings()
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

    pub fn wallets(&self) -> impl Iterator<Item = (&WalletAlias, &WalletSetting)> {
        self.inner.settings().wallets.iter()
    }

    pub fn get_all_wallets(&mut self) -> Vec<Wallet> {
        let mut wallets = vec![];

        for alias in self.inner.settings().wallets.keys() {
            wallets.push(self.wallet(alias).unwrap());
        }
        wallets
    }

    pub fn settings(&self) -> &Settings {
        self.inner.settings()
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
        nodes: Vec<&Node>,
    ) -> ConsumptionBenchmarkRun {
        benchmark_consumption(info.to_owned())
            .for_processes(nodes.iter().map(|x| x.as_named_process()).collect())
            .bare_metal_stake_pool_consumption_target()
            .start()
    }

    pub fn wallet(&self, wallet: &str) -> Result<Wallet> {
        if let Some(wallet) = self.settings().wallets.get(wallet) {
            Ok(wallet.clone().into())
        } else {
            Err(Error::WalletNotFound(wallet.to_owned()))
        }
    }

    pub fn new_spawn_params(&self, node_alias: &str) -> SpawnParams {
        SpawnParams::new(node_alias).node_key_file(self.node_dir(node_alias).path().into())
    }

    fn node_dir(&self, alias: &str) -> ChildPath {
        self.working_directory.child(alias)
    }

    pub fn spawn_legacy_node(
        &mut self,
        params: SpawnParams,
        version: &Version,
    ) -> Result<LegacyNode> {
        let mut node_setting = self
            .settings()
            .nodes
            .get(params.get_alias())
            .ok_or_else(|| Error::NodeNotFound(params.get_alias().clone()))?
            .clone();
        params.override_settings(&mut node_setting.config);

        let log_file_path = self
            .node_dir(params.get_alias())
            .child("node.log")
            .path()
            .to_path_buf();
        node_setting.config.log = Some(Log(LogEntry {
            format: "json".into(),
            level: params
                .get_log_level()
                .unwrap_or(&LogLevel::DEBUG)
                .to_string(),
            output: LogOutput::File(log_file_path),
        }));

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0.to_block().header().hash()),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(&jormungandr),
            None => self.context.jormungandr().to_path_buf(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut spawn_builder: SpawnBuilder<_, LegacyNode> =
            SpawnBuilder::new(&self.context, &mut node_setting);
        spawn_builder
            .path_to_jormungandr(jormungandr)
            .progress_bar(pb)
            .alias(params.get_alias())
            .block0(self.block0.clone())
            .block0_setting(block0_setting)
            .working_dir(self.node_dir(params.get_alias()).path())
            .peristence_mode(params.get_persistence_mode());
        spawn_builder.build(version).map_err(Into::into)
    }

    pub fn spawn_node_custom(&mut self, params: SpawnParams) -> Result<Node> {
        let mut node_setting = self
            .settings()
            .nodes
            .get(params.get_alias())
            .ok_or_else(|| Error::NodeNotFound(params.get_alias().clone()))?
            .clone();
        params.override_settings(&mut node_setting.config);

        let log_file_path = self
            .node_dir(params.get_alias())
            .child("node.log")
            .path()
            .to_path_buf();
        node_setting.config.log = Some(Log(LogEntry {
            format: "json".into(),
            level: params
                .get_log_level()
                .unwrap_or(&LogLevel::DEBUG)
                .to_string(),
            output: LogOutput::File(log_file_path),
        }));

        // remove all id from trusted peers for current version
        for trusted_peer in node_setting.config.p2p.trusted_peers.iter_mut() {
            trusted_peer.id = None;
        }

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0.to_block().header().hash()),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(&jormungandr),
            None => self.context.jormungandr().to_path_buf(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut spawn_builder: SpawnBuilder<_, Node> =
            SpawnBuilder::new(&self.context, &mut node_setting);
        spawn_builder
            .path_to_jormungandr(jormungandr)
            .progress_bar(pb)
            .alias(params.get_alias())
            .block0(self.block0.clone())
            .block0_setting(block0_setting)
            .working_dir(self.node_dir(params.get_alias()).path())
            .peristence_mode(params.get_persistence_mode());
        if let Some(faketime) = params.get_faketime().take() {
            spawn_builder.faketime(faketime.clone());
        }

        spawn_builder.build().map_err(Into::into)
    }

    pub fn spawn_node(
        &mut self,
        node_alias: &str,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<Node> {
        self.spawn_node_custom(
            self.new_spawn_params(node_alias)
                .leadership_mode(leadership_mode)
                .persistence_mode(persistence_mode),
        )
    }

    pub fn restart_node(
        &mut self,
        mut node: Node,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<Node> {
        node.shutdown()?;
        let new_node = self.spawn_node(&node.alias(), leadership_mode, persistence_mode)?;
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
        let hash = Hash::from_hash(self.block0.to_block().header().hash());

        let blockchain_configuration = self.settings().block0.blockchain_configuration.clone();

        let block_date_generator = BlockDateGenerator::rolling_from_blockchain_config(
            &blockchain_configuration,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            false,
        );

        FragmentSender::new(
            hash,
            blockchain_configuration.linear_fees,
            block_date_generator,
            builder.build(),
        )
    }
}
