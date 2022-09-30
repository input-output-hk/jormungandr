mod node;

use crate::{
    builder::{NetworkBuilder, Settings, Topology, Wallet as WalletSetting},
    config::{Blockchain, SessionSettings, SpawnParams},
    controller::{Controller as InnerController, Error},
    style,
};
use chain_impl_mockchain::testing::scenario::template::VotePlanDef;
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_automation::{
    jormungandr::{LeadershipMode, PersistenceMode, TestingDirectory, Version},
    testing::observer::{Event, Observable, Observer},
};
use jormungandr_lib::interfaces::Block0Configuration;
pub use node::{Error as NodeError, LegacyNode, Node, ProgressBarController};
use std::{net::SocketAddr, path::PathBuf, rc::Rc, sync::Arc};
use thor::{StakePool, Wallet, WalletAlias};

pub struct MonitorControllerBuilder {
    title: String,
    network_builder: NetworkBuilder,
}

impl MonitorControllerBuilder {
    pub fn new(title: &str) -> Self {
        Self {
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

    pub fn build(self, session_settings: SessionSettings) -> Result<MonitorController, Error> {
        let observer: Rc<dyn Observer> = Rc::new(NetworkBuilderObserver::new(&self.title));
        let inner_controller = self
            .network_builder
            .session_settings(session_settings.clone())
            .register(&observer)
            .build()?;

        MonitorController::new(inner_controller, session_settings)
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

pub struct MonitorController {
    inner: InnerController,
    progress_bar: Arc<MultiProgress>,
    session_settings: SessionSettings,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,
}

impl MonitorController {
    pub fn new_with_progress_bar(
        controller: InnerController,
        session_settings: SessionSettings,
        progress_bar: Arc<MultiProgress>,
    ) -> Self {
        Self {
            inner: controller,
            session_settings,
            progress_bar,
            progress_bar_thread: None,
        }
    }

    pub fn new(
        controller: InnerController,
        session_settings: SessionSettings,
    ) -> Result<Self, Error> {
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Self::new_with_progress_bar(
            controller,
            session_settings,
            progress_bar,
        ))
    }

    pub fn stake_pool(&mut self, node_alias: &str) -> Result<StakePool, Error> {
        if let Some(stake_pool) = self.inner.settings().stake_pools.get(node_alias) {
            Ok(stake_pool.clone())
        } else {
            Err(Error::StakePoolNotFound(node_alias.to_owned()))
        }
    }

    pub fn working_directory(&self) -> &TestingDirectory {
        self.inner.working_directory()
    }

    pub fn block0_conf(&self) -> Block0Configuration {
        self.inner.settings().block0.clone()
    }

    pub fn defined_wallets(&self) -> impl Iterator<Item = (WalletAlias, &WalletSetting)> {
        self.inner.defined_wallets()
    }

    pub fn controlled_wallets(&mut self) -> Vec<Wallet> {
        self.inner
            .settings()
            .wallets
            .iter()
            .cloned()
            .filter(|x| x.template().is_generated())
            .map(|x| {
                x.try_into()
                    .expect("internal error.. generated wallet should have inner wallet")
            })
            .collect()
    }

    pub fn settings(&self) -> &Settings {
        self.inner.settings()
    }

    pub fn defined_vote_plans(&self) -> Vec<VotePlanDef> {
        self.inner.defined_vote_plans()
    }

    pub fn session_settings(&self) -> &SessionSettings {
        &self.session_settings
    }

    pub fn add_to_progress_bar(&mut self, pb: ProgressBar) -> ProgressBar {
        self.progress_bar.add(pb)
    }

    pub fn block0_file(&self) -> PathBuf {
        self.inner.block0_file()
    }

    pub fn controlled_wallet(&self, wallet: &str) -> Result<Wallet, Error> {
        self.settings()
            .wallets
            .iter()
            .cloned()
            .find(|w| w.template().alias() == Some(wallet.to_string()))
            .map(|w| w.into())
            .ok_or_else(|| Error::WalletNotFound(wallet.to_owned()))
    }

    pub fn new_spawn_params(&self, node_alias: &str) -> SpawnParams {
        SpawnParams::new(node_alias).node_key_file(self.node_dir(node_alias))
    }

    fn node_dir(&self, alias: &str) -> PathBuf {
        self.session_settings.root.path().join(alias)
    }

    fn build_progress_bar(&mut self, alias: &str, listen: SocketAddr) -> ProgressBarController {
        let pb = ProgressBar::new_spinner();
        let pb = self.add_to_progress_bar(pb);
        ProgressBarController::new(pb, format!("{}@{}", alias, listen))
    }

    pub fn spawn_node(
        &mut self,
        node_alias: &str,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<Node, Error> {
        self.spawn_node_custom(
            self.new_spawn_params(node_alias)
                .leadership_mode(leadership_mode)
                .persistence_mode(persistence_mode)
                .jormungandr(self.session_settings.jormungandr.to_path_buf()),
        )
    }

    pub fn spawn_node_custom(&mut self, input_params: SpawnParams) -> Result<Node, Error> {
        let jormungandr_process = self.inner.spawn(input_params.clone())?;

        let progress_bar =
            self.build_progress_bar(input_params.get_alias(), jormungandr_process.rest_address());

        Ok(Node::new(jormungandr_process, progress_bar))
    }

    pub fn spawn_legacy_node(
        &mut self,
        input_params: SpawnParams,
        version: &Version,
    ) -> Result<LegacyNode, Error> {
        let (jormungandr_process, legacy_node_config) =
            self.inner.spawn_legacy(input_params.clone(), version)?;
        let progress_bar =
            self.build_progress_bar(input_params.get_alias(), jormungandr_process.rest_address());
        Ok(LegacyNode::new(
            jormungandr_process,
            progress_bar,
            legacy_node_config,
        ))
    }

    pub fn monitor_nodes(&mut self) {
        let pb = Arc::clone(&self.progress_bar);
        self.progress_bar_thread = Some(std::thread::spawn(move || {
            pb.join().unwrap();
        }));
    }

    pub fn finalize(self) {
        if let Some(thread) = self.progress_bar_thread {
            thread.join().unwrap()
        }
    }
}

impl From<MonitorController> for InnerController {
    fn from(monitor: MonitorController) -> Self {
        monitor.inner
    }
}
