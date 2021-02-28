use super::FragmentNode;
use crate::testing::SyncNode;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone)]
pub enum VerifyStrategy<'a> {
    AnyOf(Vec<&'a (dyn FragmentNode + Send + Sync)>),
    AllOf(Vec<&'a (dyn FragmentNode + Send + Sync)>),
    Single(&'a (dyn FragmentNode + Send + Sync)),
}

impl<'a> fmt::Debug for VerifyStrategy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyStrategy::AnyOf(nodes) => {
                let aliases: Vec<String> = nodes.iter().map(|x| x.alias().to_string()).collect();
                write!(f, "Any of {:?}", aliases)
            }
            VerifyStrategy::AllOf(nodes) => {
                let aliases: Vec<String> = nodes.iter().map(|x| x.alias().to_string()).collect();
                write!(f, "All of {:?}", aliases)
            }
            VerifyStrategy::Single(node) => write!(f, "{}", node.alias()),
        }
    }
}

#[derive(Clone)]
pub struct FragmentSenderSetup<'a> {
    pub resend_on_error: Option<u8>,
    pub sync_nodes: Vec<&'a (dyn SyncNode)>,
    pub ignore_any_errors: bool,
    pub dump_fragments: Option<PathBuf>,
    /// Sender will confirm transaction (increment account counter)
    ///
    pub auto_confirm: bool,
    /// Sender verifies transaction strategy. By default is disabled,
    /// so sender will verify fragment against node to which recieved transaction
    pub verify_strategy: Option<VerifyStrategy<'a>>,

    /// Just send fragment without any verifications
    pub fire_and_forget: bool,
}

impl<'a> FragmentSenderSetup<'a> {
    pub fn ignore_errors() -> Self {
        let mut builder = FragmentSenderSetupBuilder::new();
        builder.ignore_any_errors();
        builder.into()
    }

    pub fn resend_3_times() -> Self {
        let mut builder = FragmentSenderSetupBuilder::new();
        builder.resend_on_error(3);
        builder.into()
    }

    pub fn resend_3_times_and_sync_with(sync_nodes: Vec<&'a (dyn SyncNode)>) -> Self {
        let mut builder = FragmentSenderSetupBuilder::new();
        builder.resend_on_error(3).sync_nodes(sync_nodes);
        builder.into()
    }

    pub fn no_verify() -> Self {
        let mut builder = FragmentSenderSetupBuilder::new();
        builder.fire_and_forget();
        builder.into()
    }

    pub fn new() -> Self {
        Self {
            resend_on_error: None,
            sync_nodes: Vec::new(),
            ignore_any_errors: false,
            dump_fragments: None,
            auto_confirm: true,
            verify_strategy: None,
            fire_and_forget: false,
        }
    }

    pub fn resend_on_error(&self) -> Option<u8> {
        self.resend_on_error
    }

    pub fn sync_nodes(&self) -> Vec<&'a (dyn SyncNode)> {
        self.sync_nodes.clone()
    }

    pub fn no_sync_nodes(&self) -> bool {
        self.sync_nodes().is_empty()
    }

    pub fn ignore_any_errors(&self) -> bool {
        self.ignore_any_errors
    }

    pub fn attempts_count(&self) -> u8 {
        match self.resend_on_error {
            Some(resend_counter) => resend_counter + 1,
            None => 1,
        }
    }

    pub fn auto_confirm(&self) -> bool {
        self.auto_confirm
    }

    pub fn fire_and_forget(&self) -> bool {
        self.fire_and_forget
    }
}

impl<'a> Default for FragmentSenderSetup<'a> {
    fn default() -> FragmentSenderSetup<'a> {
        FragmentSenderSetup::new()
    }
}

pub struct FragmentSenderSetupBuilder<'a> {
    setup: FragmentSenderSetup<'a>,
}

impl<'a> Default for FragmentSenderSetupBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> FragmentSenderSetupBuilder<'a> {
    pub fn from(setup: FragmentSenderSetup<'a>) -> Self {
        Self { setup }
    }

    pub fn new() -> Self {
        Self {
            setup: FragmentSenderSetup::new(),
        }
    }

    pub fn resend_on_error(&mut self, count: u8) -> &mut Self {
        self.setup.resend_on_error = Some(count);
        self
    }

    pub fn sync_nodes(&mut self, sync_nodes: Vec<&'a (dyn SyncNode)>) -> &mut Self {
        self.setup.sync_nodes = sync_nodes;
        self
    }

    pub fn ignore_any_errors(&mut self) -> &mut Self {
        self.setup.ignore_any_errors = true;
        self
    }

    pub fn dump_fragments_into(&mut self, path: PathBuf) -> &mut Self {
        self.setup.dump_fragments = Some(path);
        self
    }

    pub fn fire_and_forget(&mut self) -> &mut Self {
        self.setup.fire_and_forget = true;
        self
    }

    pub fn build(self) -> FragmentSenderSetup<'a> {
        self.setup
    }
}

impl<'a> Into<FragmentSenderSetup<'a>> for FragmentSenderSetupBuilder<'a> {
    fn into(self) -> FragmentSenderSetup<'a> {
        self.setup
    }
}
