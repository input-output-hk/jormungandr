use crate::{
    builder::SpawnParams,
    controller::{interactive::ControllerError, Controller, Error},
};
use chain_impl_mockchain::{certificate::VotePlan, vote::Choice};
use jormungandr_automation::jormungandr::{JormungandrProcess, Version};
use jormungandr_lib::interfaces::Value;
use jortestkit::prelude::InteractiveCommandError;
use thor::{FragmentSender, Wallet};

pub struct UserInteractionController {
    controller: Controller,
    wallets: Vec<Wallet>,
    nodes: Vec<JormungandrProcess>,
    legacy_nodes: Vec<JormungandrProcess>,
}

impl UserInteractionController {
    pub fn new(inner: Controller) -> Self {
        let wallets = inner
            .defined_wallets()
            .map(|(_, wallet)| wallet.clone())
            .map(Into::into)
            .collect();

        Self {
            controller: inner,
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

    pub fn nodes(&self) -> &[JormungandrProcess] {
        &self.nodes
    }

    pub fn legacy_nodes(&self) -> &[JormungandrProcess] {
        &self.legacy_nodes
    }

    pub fn legacy_nodes_mut(&mut self) -> &mut Vec<JormungandrProcess> {
        &mut self.legacy_nodes
    }
    pub fn nodes_mut(&mut self) -> &mut Vec<JormungandrProcess> {
        &mut self.nodes
    }

    pub fn controller(&self) -> &Controller {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut Controller {
        &mut self.controller
    }

    pub fn wallet(&self, wallet: &str) -> Result<Wallet, Error> {
        if let Some(wallet) = self.controller.settings().wallets.get(wallet) {
            Ok(wallet.clone().into())
        } else {
            Err(ControllerError::WalletNotFound(wallet.to_owned())).map_err(Into::into)
        }
    }

    // It is easier to convert to test::Result with ?, or we would have to individually
    // map errors for each match arm with verbose Into syntax
    #[allow(clippy::try_err)]
    pub fn tally_vote(
        &mut self,
        committee_alias: &str,
        vote_plan_alias: &str,
        node_alias: &str,
    ) -> Result<jormungandr_automation::jormungandr::MemPoolCheck, Error> {
        let committee_address = self.controller.wallet(committee_alias)?.address();
        let vote_plan_def = self.controller.defined_vote_plan(vote_plan_alias)?;

        let mut temp_wallets = self.wallets_mut().clone();
        let committee = temp_wallets
            .iter_mut()
            .find(|x| x.address() == committee_address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", committee_alias));

        let node = self.nodes.iter().find(|x| x.alias() == node_alias);
        let legacy_node = self.legacy_nodes.iter().find(|x| x.alias() == node_alias);

        let fragment_sender = FragmentSender::from(&self.controller.settings().block0);

        let vote_plan: VotePlan = vote_plan_def.into();

        let check = match (node, legacy_node) {
            (Some(node), None) => {
                fragment_sender.send_public_vote_tally(committee, vote_plan.to_id(), node)?
            }
            (None, Some(node)) => {
                fragment_sender.send_public_vote_tally(committee, vote_plan.to_id(), node)?
            }
            _ => Err(InteractiveCommandError::UserError(format!(
                "alias not found {}",
                node_alias
            )))?,
        };

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    // It is easier to convert to test::Result with ?, or we would have to individually
    // map errors for each match arm with verbose Into syntax
    #[allow(clippy::try_err)]
    pub fn cast_vote(
        &mut self,
        wallet_alias: &str,
        vote_plan_alias: &str,
        node_alias: &str,
        proposal_index: usize,
        choice: u8,
    ) -> Result<jormungandr_automation::jormungandr::MemPoolCheck, Error> {
        let address = self.controller.wallet(wallet_alias)?.address();
        let vote_plan_def = self.controller.defined_vote_plan(vote_plan_alias)?;

        let mut temp_wallets = self.wallets_mut().clone();
        let wallet = temp_wallets
            .iter_mut()
            .find(|x| x.address() == address)
            .unwrap_or_else(|| panic!("cannot find wallet with alias: {}", wallet_alias));

        let node = self.nodes.iter().find(|x| x.alias() == node_alias);
        let legacy_node = self.legacy_nodes.iter().find(|x| x.alias() == node_alias);

        let fragment_sender = FragmentSender::from(&self.controller.settings().block0);
        let check = match (node, legacy_node) {
            (Some(node), None) => fragment_sender.send_vote_cast(
                wallet,
                &vote_plan_def.into(),
                proposal_index as u8,
                &Choice::new(choice),
                node,
            )?,
            (None, Some(node)) => fragment_sender.send_vote_cast(
                wallet,
                &vote_plan_def.into(),
                proposal_index as u8,
                &Choice::new(choice),
                node,
            )?,
            _ => Err(InteractiveCommandError::UserError(format!(
                "alias not found {}",
                node_alias
            )))?,
        };

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    // It is easier to convert to test::Result with ?, or we would have to individually
    // map errors for each match arm with verbose Into syntax
    #[allow(clippy::try_err)]
    pub fn send_transaction(
        &mut self,
        from_str: &str,
        to_str: &str,
        node_alias: &str,
        value: Value,
    ) -> Result<jormungandr_automation::jormungandr::MemPoolCheck, Error> {
        let from_address = self.controller.wallet(from_str)?.address();
        let to_address = self.controller.wallet(to_str)?.address();

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

        let node = self.nodes.iter().find(|x| x.alias() == node_alias);
        let legacy_node = self.legacy_nodes.iter().find(|x| x.alias() == node_alias);

        let fragment_sender = FragmentSender::from(&self.controller.settings().block0);

        let check = match (node, legacy_node) {
            (Some(node), None) => fragment_sender.send_transaction(from, &to, node, value)?,
            (None, Some(node)) => fragment_sender.send_transaction(from, &to, node, value)?,
            _ => Err(InteractiveCommandError::UserError(format!(
                "alias not found {}",
                node_alias
            )))?,
        };

        *self.wallets_mut() = temp_wallets;
        Ok(check)
    }

    pub fn spawn_node(&mut self, input_params: SpawnParams) -> Result<JormungandrProcess, Error> {
        self.controller.spawn(input_params).map_err(Into::into)
    }

    pub fn spawn_legacy_node(
        &mut self,
        input_params: SpawnParams,
        version: &Version,
    ) -> Result<JormungandrProcess, Error> {
        self.controller
            .spawn_legacy(input_params, version)
            .map(|(process, _settings)| process)
            .map_err(Into::into)
    }
}

pub fn do_for_all_alias<F: Fn(&JormungandrProcess), G: Fn(&JormungandrProcess)>(
    alias: &Option<String>,
    nodes: &[JormungandrProcess],
    legacy_nodes: &[JormungandrProcess],
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
