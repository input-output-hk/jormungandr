use crate::{
    scenario::{settings::Settings, ContextChaCha, ErrorKind, Result},
    Node, NodeBlock0, NodeController,
};
use chain_impl_mockchain::block::HeaderHash;
use indicatif::{MultiProgress, ProgressBar};
use mktemp::Temp;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::runtime;

pub struct Controller {
    settings: Arc<Settings>,

    context: ContextChaCha,

    block0_file: Temp,
    block0_hash: HeaderHash,

    progress_bar: Arc<MultiProgress>,
    progress_bar_thread: Option<std::thread::JoinHandle<()>>,

    runtime: runtime::Runtime,
}

impl Controller {
    pub fn new(settings: Settings, context: ContextChaCha) -> Result<Self> {
        use chain_core::property::Serialize as _;

        let block0 = settings.block0.to_block();
        let block0_hash = block0.header.hash();

        let block0_file = Temp::new_file()?;
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;
        let progress_bar = Arc::new(MultiProgress::new());

        Ok(Controller {
            settings: Arc::new(settings),
            context,
            block0_file,
            block0_hash,
            progress_bar,
            progress_bar_thread: None,
            runtime: runtime::Runtime::new()?,
        })
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

        let node = Node::spawn(&self.context, pb, node_alias, node_setting, block0_setting)?;
        let controller = node.controller();

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
}
