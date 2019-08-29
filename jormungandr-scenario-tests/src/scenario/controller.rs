use crate::{
    scenario::{settings::Settings, Blockchain, ContextChaCha, ErrorKind, Result, Topology},
    style, MemPoolCheck, Node, NodeBlock0, NodeController, Wallet,
};
use chain_impl_mockchain::block::HeaderHash;
use indicatif::{MultiProgress, ProgressBar};
use jormungandr_lib::interfaces::Value;
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
    block0_hash: HeaderHash,

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

            settings.dottify(file)?;

            for wallet in settings.wallets.values() {
                wallet.save_to(path)?;
            }
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
            Ok(wallet)
        } else {
            Err(ErrorKind::WalletNotFound(wallet.to_owned()).into())
        }
    }

    pub fn spawn_node(&mut self, node_alias: &str, with_block0: bool) -> Result<NodeController> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(node_alias) {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(node_alias.to_owned()))
        };

        let block0_setting = if with_block0 {
            NodeBlock0::File(self.block0_file.as_path().into())
        } else {
            NodeBlock0::Hash(self.block0_hash.clone())
        };

        let pb = ProgressBar::new_spinner();
        let pb = self.progress_bar.add(pb);

        let mut node = Node::spawn(
            &self.context,
            pb,
            node_alias,
            node_setting,
            block0_setting,
            &self.working_directory,
        )?;
        let controller = node.controller();

        self.runtime.executor().spawn(node.capture_logs());
        self.runtime.executor().spawn(node);

        Ok(controller)
    }

    pub fn monitor_nodes(&mut self) {
        let pb = Arc::clone(&self.progress_bar);
        self.progress_bar_thread = Some(std::thread::spawn(move || {
            pb.join().unwrap();
        }));
    }

    pub fn finalize(self) {
        self.runtime.shutdown_on_idle().wait().unwrap(); //.shutdown_now().wait().unwrap();
        if let Some(thread) = self.progress_bar_thread {
            thread.join().unwrap()
        }
    }

    pub fn wallet_send_to(
        &mut self,
        from: &mut Wallet,
        to: &Wallet,
        via: &NodeController,
        value: Value,
    ) -> Result<MemPoolCheck> {
        let block0_hash = &self.block0_hash;
        let fees = &self.settings.block0.blockchain_configuration.linear_fees;
        let address = to.address(chain_addr::Discrimination::Test);

        let fragment = from.transaction_to(&block0_hash.clone().into(), fees, address, value)?;

        Ok(via.send_fragment(fragment)?)
    }
}
