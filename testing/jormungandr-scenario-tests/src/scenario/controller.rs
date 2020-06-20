use crate::{
    legacy::{LegacyNode, LegacyNodeController, LegacySettings},
    prepare_command,
    scenario::{
        settings::{Dotifier, PrepareSettings},
        ContextChaCha, ErrorKind, ProgressBarMode, Result,
    },
    style, Node, NodeBlock0, NodeController,
};

use chain_impl_mockchain::header::HeaderId;
use jormungandr_integration_tests::common::legacy::Version;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        benchmark_consumption,
        network_builder::{
            Blockchain, LeadershipMode, NodeAlias, NodeSetting, PersistenceMode, Settings,
            SpawnParams, Topology, Wallet as WalletSetting, WalletAlias,
        },
        ConsumptionBenchmarkRun, FragmentSender, FragmentSenderSetup, FragmentSenderSetupBuilder,
    },
    wallet::Wallet,
};

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use indicatif::{MultiProgress, ProgressBar};
use tokio::prelude::*;
use tokio::runtime;

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

    runtime: runtime::Runtime,
    topology: Topology,
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

        Controller::new(
            self.settings.unwrap(),
            context,
            working_directory,
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
            dotifier.dottify(&settings, file)?;

            for wallet in settings.wallets.values() {
                wallet.save_to(path)?;
            }

            let file = std::fs::File::create(&path.join("genesis.yaml"))?;
            serde_yaml::to_writer(file, &settings.block0).unwrap();
        }

        Ok(())
    }
}

impl Controller {
    fn new(
        settings: Settings,
        context: ContextChaCha,
        working_directory: ChildPath,
        topology: Topology,
    ) -> Result<Self> {
        use chain_core::property::Serialize as _;

        let block0 = settings.block0.to_block();
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
            runtime: runtime::Runtime::new()?,
            working_directory,
            topology,
        })
    }

    pub fn stake_pool(&mut self, node_alias: &str) -> Result<StakePool> {
        if let Some(stake_pool) = self.settings.stake_pools.get(node_alias) {
            Ok(stake_pool.clone().into())
        } else {
            Err(ErrorKind::StakePoolNotFound(node_alias.to_owned()).into())
        }
    }

    pub fn working_directory(&self) -> &ChildPath {
        &self.working_directory
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&NodeAlias, &NodeSetting)> {
        self.settings.nodes.iter()
    }

    pub fn wallets(&self) -> impl Iterator<Item = (&WalletAlias, &WalletSetting)> {
        self.settings.wallets.iter()
    }

    pub fn get_all_wallets(&mut self) -> Vec<Wallet> {
        let mut wallets = vec![];

        for alias in self.settings.wallets.clone().keys() {
            wallets.push(self.wallet(alias).unwrap());
        }
        wallets
    }

    pub fn topology(&self) -> &Topology {
        &self.topology
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

    pub fn wallet(&mut self, wallet: &str) -> Result<Wallet> {
        if let Some(wallet) = self.settings.wallets.get(wallet) {
            Ok(wallet.clone().into())
        } else {
            Err(ErrorKind::WalletNotFound(wallet.to_owned()).into())
        }
    }

    pub fn new_spawn_params(&self, node_alias: &str) -> SpawnParams {
        SpawnParams::new(node_alias)
    }

    pub fn spawn_legacy_node(
        &mut self,
        params: &mut SpawnParams,
        version: &Version,
    ) -> Result<LegacyNodeController> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(&params.get_alias())
        {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(params.get_alias()))
        };

        let mut node_setting_overriden = node_setting.clone();
        params.override_settings(&mut node_setting_overriden.config);

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0_hash),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(jormungandr.clone()),
            None => self.context.jormungandr().clone(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut legacy_node_settings =
            LegacySettings::from_settings(node_setting_overriden, version);

        let mut node = LegacyNode::spawn(
            &jormungandr,
            &self.context,
            pb,
            &params.get_alias(),
            &mut legacy_node_settings,
            block0_setting,
            self.working_directory.path(),
            params.get_persistence_mode(),
        )?;
        let controller = node.controller();

        self.runtime.executor().spawn(node.capture_logs());
        self.runtime.executor().spawn(node);

        Ok(controller)
    }

    pub fn spawn_node_custom(&mut self, params: &mut SpawnParams) -> Result<NodeController> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(&params.get_alias())
        {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(params.get_alias()))
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
            Some(jormungandr) => prepare_command(jormungandr.clone()),
            None => self.context.jormungandr().clone(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut node = Node::spawn(
            &jormungandr,
            &self.context,
            pb,
            &params.get_alias(),
            &mut node_setting_overriden,
            block0_setting,
            self.working_directory.path(),
            params.get_persistence_mode(),
        )?;
        let controller = node.controller();

        self.runtime.executor().spawn(node.capture_logs());
        self.runtime.executor().spawn(node);

        Ok(controller)
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
        self.runtime.shutdown_now().wait().unwrap();
        if let Some(thread) = self.progress_bar_thread {
            thread.join().unwrap()
        }
    }

    pub fn fragment_sender(&self) -> FragmentSender {
        self.fragment_sender_with_setup(Default::default())
    }

    pub fn fragment_sender_with_setup<'a>(
        &self,
        setup: FragmentSenderSetup<'a>,
    ) -> FragmentSender<'a> {
        let mut builder = FragmentSenderSetupBuilder::from(setup);
        let root_dir: PathBuf = PathBuf::from(self.working_directory().path());
        builder.dump_fragments_into(root_dir.join("fragments"));
        let hash = Hash::from_hash(self.block0_hash.clone());

        FragmentSender::new(
            hash,
            self.settings.block0.blockchain_configuration.linear_fees,
            builder.build(),
        )
    }
}
