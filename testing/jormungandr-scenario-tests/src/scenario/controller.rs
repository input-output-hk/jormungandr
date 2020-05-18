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
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_integration_tests::common::legacy::Version;
use jormungandr_lib::crypto::hash::Hash;
use jormungandr_testing_utils::{
    testing::{
        network_builder::{
            Blockchain, LeadershipMode, PersistenceMode, Settings, SpawnParams, Topology,
        },
        FragmentSender,
    },
    wallet::Wallet,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::prelude::*;
use tokio::runtime;

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

    working_directory: PathBuf,

    block0_file: PathBuf,
    block0_hash: HeaderId,

    progress_bar: Arc<MultiProgress>,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,

    runtime: runtime::Runtime,
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
        let topology = std::mem::replace(&mut self.topology, None).unwrap();
        let blockchain = std::mem::replace(&mut self.blockchain, None).unwrap();
        self.settings = Some(Settings::prepare(topology, blockchain, context));
        self.controller_progress.inc(5);
    }

    pub fn build(self, context: ContextChaCha) -> Result<Controller> {
        let working_directory = context.working_directory().join(&self.title);
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&working_directory)?;
        if context.generate_documentation() {
            self.document(&working_directory)?;
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

        Controller::new(self.settings.unwrap(), context, working_directory)
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
    fn new(settings: Settings, context: ContextChaCha, working_directory: PathBuf) -> Result<Self> {
        use chain_core::property::Serialize as _;

        let block0 = settings.block0.to_block();
        let block0_hash = block0.header.hash();

        let block0_file = working_directory.join("block0.bin");
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Controller {
            settings: settings,
            context,
            block0_file,
            block0_hash,
            progress_bar,
            progress_bar_thread: None,
            runtime: runtime::Runtime::new()?,
            working_directory,
        })
    }

    pub fn wallet(&mut self, wallet: &str) -> Result<Wallet> {
        if let Some(wallet) = self.settings.wallets.remove(wallet) {
            Ok(wallet.into())
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
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0_hash.clone()),
        };

        let jormungandr = match &params.get_jormungandr() {
            Some(jormungandr) => prepare_command(jormungandr.clone()),
            None => self.context.jormungandr().clone(),
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut legacy_node_settings =
            LegacySettings::from_settings(node_setting_overriden, version);

        println!("settings: {:?}, debug: {:?}", legacy_node_settings, version);

        let mut node = LegacyNode::spawn(
            &jormungandr,
            &self.context,
            pb,
            &params.get_alias(),
            &mut legacy_node_settings,
            block0_setting,
            &self.working_directory,
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

        let block0_setting = match params.get_leadership_mode() {
            LeadershipMode::Leader => NodeBlock0::File(self.block0_file.as_path().into()),
            LeadershipMode::Passive => NodeBlock0::Hash(self.block0_hash.clone()),
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
            &self.working_directory,
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
        let hash = Hash::from_hash(self.block0_hash.clone());
        FragmentSender::new(
            hash,
            self.settings
                .block0
                .blockchain_configuration
                .linear_fees
                .clone(),
        )
    }
}
