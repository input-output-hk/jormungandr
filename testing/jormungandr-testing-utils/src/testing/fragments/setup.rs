use crate::testing::SyncNode;

pub struct FragmentSenderSetup<'a> {
    pub resend_on_error: Option<u8>,
    pub sync_nodes: Vec<&'a dyn SyncNode>,
    pub ignore_any_errors: bool,
    pub no_verify: bool,
}

impl<'a> FragmentSenderSetup<'a> {
    pub const NO_VERIFY: FragmentSenderSetup<'a> = FragmentSenderSetup {
        resend_on_error: None,
        sync_nodes: Vec::new(),
        ignore_any_errors: false,
        no_verify: true,
    };

    pub const RESEND_3_TIMES: FragmentSenderSetup<'a> = FragmentSenderSetup {
        resend_on_error: Some(3),
        sync_nodes: Vec::new(),
        ignore_any_errors: false,
        no_verify: false,
    };

    pub fn resend_3_times_and_sync_with(sync_nodes: Vec<&'a dyn SyncNode>) -> Self {
        Self {
            resend_on_error: Some(3),
            sync_nodes: sync_nodes,
            ignore_any_errors: false,
            no_verify: false,
        }
    }

    pub fn new() -> Self {
        Self {
            resend_on_error: None,
            sync_nodes: Vec::new(),
            ignore_any_errors: false,
            no_verify: false,
        }
    }

    pub fn resend_on_error(&self) -> Option<u8> {
        self.resend_on_error.clone()
    }

    pub fn sync_nodes(&self) -> Vec<&'a dyn SyncNode> {
        self.sync_nodes.clone()
    }

    pub fn no_sync_nodes(&self) -> bool {
        self.sync_nodes().len() == 0
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

    pub fn no_verify(&self) -> bool {
        self.no_verify
    }
}

impl<'a> Default for FragmentSenderSetup<'a> {
    fn default() -> FragmentSenderSetup<'a> {
        FragmentSenderSetup::new()
    }
}
