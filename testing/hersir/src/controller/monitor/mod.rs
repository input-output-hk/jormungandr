mod node;

use super::{spawn_legacy_node, spawn_node};
use crate::controller::{Context, Error};
use crate::style;
use crate::utils::Dotifier;
use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_testing_utils::testing::jormungandr::TestingDirectory;
use jormungandr_testing_utils::testing::network::Settings;
use jormungandr_testing_utils::testing::network::WalletAlias;
use jormungandr_testing_utils::testing::network::{
    builder::NetworkBuilder, controller::Controller as InnerController,
};
use jormungandr_testing_utils::testing::utils::{Event, Observable, Observer};
use jormungandr_testing_utils::{
    stake_pool::StakePool,
    testing::{
        benchmark_consumption,
        network::{
            Blockchain, LeadershipMode, PersistenceMode, SpawnParams, Topology,
            Wallet as WalletSetting,
        },
        ConsumptionBenchmarkRun,
    },
    wallet::Wallet,
    Version,
};
pub use node::{Error as NodeError, LegacyNode, Node, ProgressBarController};
use std::net::SocketAddr;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

pub struct MonitorControllerBuilder {
    title: String,
    network_builder: NetworkBuilder,
}

pub struct MonitorController {
    inner: InnerController,
    context: Context,
    progress_bar: Arc<MultiProgress>,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,
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

    pub fn build(self, context: Context) -> Result<MonitorController, Error> {
        let testing_directory = context.child_directory(&self.title);
        testing_directory.create_dir_all()?;
        let generate_documentation_path = testing_directory.path().to_path_buf();
        let observer: Rc<dyn Observer> = Rc::new(NetworkBuilderObserver::new(&self.title));
        let inner_controller = self
            .network_builder
            .testing_directory(testing_directory.into())
            .register(&observer)
            .build()?;
        if context.generate_documentation() {
            document(&generate_documentation_path, &inner_controller)?;
        }

        summary(&self.title);
        MonitorController::new(inner_controller, context)
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

fn document(path: &Path, inner: &InnerController) -> Result<(), Error> {
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

impl MonitorController {
    fn new(controller: InnerController, context: Context) -> Result<Self, Error> {
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Self {
            inner: controller,
            context,
            progress_bar,
            progress_bar_thread: None,
        })
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

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn add_to_progress_bar(&mut self, pb: ProgressBar) -> ProgressBar {
        self.progress_bar.add(pb)
    }

    pub fn block0_file(&self) -> PathBuf {
        self.inner.block0_file()
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

    pub fn wallet(&self, wallet: &str) -> Result<Wallet, Error> {
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
        self.context.testing_directory().child(alias)
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
                .jormungandr(self.context.jormungandr().to_path_buf()),
        )
    }

    pub fn spawn_node_custom(&mut self, input_params: SpawnParams) -> Result<Node, Error> {
        let jormungandr_process = spawn_node(&mut self.inner, input_params.clone())?;

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
            spawn_legacy_node(&mut self.inner, input_params.clone(), version)?;
        let progress_bar =
            self.build_progress_bar(input_params.get_alias(), jormungandr_process.rest_address());
        Ok(LegacyNode::new(
            jormungandr_process,
            progress_bar,
            legacy_node_config,
        ))
    }

    pub fn restart_node(
        &mut self,
        mut node: Node,
        leadership_mode: LeadershipMode,
        persistence_mode: PersistenceMode,
    ) -> Result<Node, Error> {
        node.shutdown()?;
        let new_node = self.spawn_node(&node.alias(), leadership_mode, persistence_mode)?;
        new_node.wait_for_bootstrap()?;
        Ok(new_node)
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
