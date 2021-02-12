use crate::{legacy::LegacyNodeController, test::Result};
use crate::{node::NodeController, scenario::Controller};
use chain_impl_mockchain::vote::Choice;
use jormungandr_lib::interfaces::Value;
use jormungandr_testing_utils::{
    testing::{FragmentNode, SyncNode},
    wallet::Wallet,
};
use structopt::{clap::AppSettings, StructOpt};

pub mod describe;
pub mod explorer;
pub mod send;
pub mod show;
pub mod spawn;

pub struct UserInteractionController {
    controller: Controller,
    wallets: Vec<Wallet>,
    nodes: Vec<NodeController>,
    legacy_nodes: Vec<LegacyNodeController>,
}

impl UserInteractionController {
    pub fn new(mut controller: Controller) -> Self {
        let wallets = controller.get_all_wallets();
        Self {
            controller,
            wallets,
            nodes: Vec::new(),
            legacy_nodes: Vec::new(),
        }
    }

    pub fn wallets(&self) -> &[Wallet] {
        &self.wallets
    }

    pub fn wallets_mut(&mut self) -> &mut Vec<Wallet> {
        &mut self.wallets
    }

    pub fn nodes(&self) -> &[NodeController] {
        &self.nodes
    }

    pub fn legacy_nodes(&self) -> &[LegacyNodeController] {
        &self.legacy_nodes
    }

    pub fn legacy_nodes_mut(&mut self) -> &mut Vec<LegacyNodeController> {
        &mut self.legacy_nodes
    }
    pub fn nodes_mut(&mut self) -> &mut Vec<NodeController> {
        &mut self.nodes
    }

    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut Controller {
        &mut self.controller
    }

    pub fn tally_vote<A: FragmentNode + SyncNode + Sized + Sync + Send>(
        &mut self,
        committee_alias: &str,
        vote_plan_alias: &str,
        via: &A,
    ) -> Result<jormungandr_testing_utils::testing::MemPoolCheck> {
        let committee_address = self.controller.wallet(&committee_alias)?.address();
        let vote_plan_def = self.controller.vote_plan(vote_plan_alias)?;

        let mut temp_wallets = self.wallets_mut().clone();
        let committee = temp_wallets
            .iter_mut()
            .find(|x| x.address() == committee_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", committee_alias));

        let check = self.controller.fragment_sender().send_public_vote_tally(
            committee,
            &vote_plan_def.into(),
            via,
        )?;

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    pub fn cast_vote<A: FragmentNode + SyncNode + Sized + Sync + Send>(
        &mut self,
        wallet_alias: &str,
        vote_plan_alias: &str,
        via: &A,
        proposal_index: usize,
        choice: u8,
    ) -> Result<jormungandr_testing_utils::testing::MemPoolCheck> {
        let address = self.controller.wallet(&wallet_alias)?.address();
        let vote_plan_def = self.controller.vote_plan(vote_plan_alias)?;

        let mut temp_wallets = self.wallets_mut().clone();
        let wallet = temp_wallets
            .iter_mut()
            .find(|x| x.address() == address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", wallet_alias));

        let check = self.controller.fragment_sender().send_vote_cast(
            wallet,
            &vote_plan_def.into(),
            proposal_index as u8,
            &Choice::new(choice),
            via,
        )?;

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    pub fn send_transaction<A: FragmentNode + SyncNode + Sized + Sync + Send>(
        &mut self,
        from_str: &str,
        to_str: &str,
        via: &A,
        value: Value,
    ) -> Result<jormungandr_testing_utils::testing::MemPoolCheck> {
        let from_address = self.controller.wallet(&from_str)?.address();
        let to_address = self.controller.wallet(&to_str)?.address();

        let to = self
            .wallets()
            .iter()
            .cloned()
            .find(|x| x.address() == to_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", to_str));

        let mut temp_wallets = self.wallets_mut().clone();
        let from = temp_wallets
            .iter_mut()
            .find(|x| x.address() == from_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", from_str));

        let check = self
            .controller
            .fragment_sender()
            .send_transaction(from, &to, via, value)?;
        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    pub fn finalize(self) {
        self.controller.finalize();
    }
}

#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::NoBinaryName)]
pub enum InteractiveCommand {
    /// Prints nodes related data, like stats,fragments etc.
    Show(show::Show),
    /// Spawn leader or passive node (also legacy)
    Spawn(spawn::Spawn),
    /// Sends Explorer queries
    Explorer(explorer::Explorer),
    /// Exit interactive mode
    Exit,
    /// Prints wallets, nodes which can be used. Draw topology
    Describe(describe::Describe),
    /// send fragments
    Send(send::Send),
}

fn do_for_all_alias<F: Fn(&NodeController), G: Fn(&LegacyNodeController)>(
    alias: &Option<String>,
    nodes: &[NodeController],
    legacy_nodes: &[LegacyNodeController],
    f: F,
    g: G,
) {
    if let Some(alias) = alias {
        if let Some(node) = nodes.iter().find(|x| *x.alias() == *alias) {
            f(node);
        }
        if let Some(node) = legacy_nodes.iter().find(|x| *x.alias() == *alias) {
            g(node)
        }
        return;
    }

    for node in nodes.iter() {
        f(node);
    }
    for node in legacy_nodes.iter() {
        g(node);
    }
}
