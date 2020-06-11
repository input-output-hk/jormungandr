use super::{FragmentExporter, FragmentExporterError};
use crate::{
    testing::{
        ensure_node_is_in_sync_with_others,
        fragments::node::{FragmentNode, MemPoolCheck},
        FragmentSenderSetup, FragmentVerifier, SyncNode, SyncNodeError, SyncWaitParams,
    },
    wallet::Wallet,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{FragmentStatus, Value},
};
use std::time::Duration;

use custom_debug::CustomDebug;
use thiserror::Error;

#[derive(Error, CustomDebug)]
pub enum FragmentSenderError {
    #[error("fragment sent to node: {alias} is not in block due to '{reason}'")]
    FragmentNotInBlock {
        alias: String,
        reason: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error(
        "Too many attempts failed ({attempts}) while trying to send fragment to node: {alias}"
    )]
    TooManyAttemptsFailed { attempts: u8, alias: String },
    #[error("fragment verifier error")]
    FragmentVerifierError(#[from] super::FragmentVerifierError),
    #[error("cannot send fragment")]
    SendFragmentError(#[from] super::node::FragmentNodeError),
    #[error("cannot sync node before sending fragment")]
    SyncNodeError(#[from] crate::testing::SyncNodeError),
    #[error("wallet error")]
    WalletError(#[from] crate::wallet::WalletError),
    #[error("wrong sender configuration: cannot use disable transaction auto confirm when sending more than one transaction")]
    TransactionAutoConfirmDisabledError,
    #[error("fragment exporter error")]
    FragmentExporterError(#[from] FragmentExporterError),
}

impl FragmentSenderError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::FragmentSenderError::*;
        let maybe_logs = match self {
            FragmentNotInBlock { logs, .. } => Some(logs),
            _ => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

pub struct FragmentSender<'a> {
    block0_hash: Hash,
    fees: LinearFee,
    setup: FragmentSenderSetup<'a>,
}

impl<'a> FragmentSender<'a> {
    pub fn new(block0_hash: Hash, fees: LinearFee, setup: FragmentSenderSetup<'a>) -> Self {
        Self {
            block0_hash,
            fees,
            setup,
        }
    }

    pub fn send_transaction<A: FragmentNode + SyncNode + Sized>(
        &self,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
        value: Value,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let address = to.address();
        let fragment = from.transaction_to(&self.block0_hash, &self.fees, address, value)?;
        self.dump_fragment_if_enabled(from, to, &value, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_transactions<A: FragmentNode + SyncNode + Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        if self.setup.auto_confirm() == false {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, &wallet2, node, value)?;
        }
        Ok(())
    }

    pub fn send_transactions_with_iteration_delay<A: FragmentNode + SyncNode + Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &mut Wallet,
        node: &A,
        value: Value,
        duration: Duration,
    ) -> Result<(), FragmentSenderError> {
        if self.setup.auto_confirm() == false {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, &wallet2, node, value.clone())?;
            std::thread::sleep(duration);
        }
        Ok(())
    }

    pub fn send_transactions_round_trip<A: FragmentNode + SyncNode + Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        mut wallet2: &mut Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        if self.setup.auto_confirm() == false {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, &wallet2, node, value.clone())?;
            self.send_transaction(&mut wallet2, &wallet1, node, value.clone())?;
        }
        Ok(())
    }

    fn verify<A: FragmentNode + SyncNode + Sized>(
        &self,
        check: &MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentSenderError> {
        let verifier = FragmentVerifier;
        match verifier.wait_fragment(Duration::from_secs(2), check.clone(), node)? {
            FragmentStatus::Rejected { reason } => Err(FragmentSenderError::FragmentNotInBlock {
                alias: FragmentNode::alias(node).to_string(),
                reason,
                logs: FragmentNode::log_content(node),
            }),
            FragmentStatus::InABlock { .. } => Ok(()),
            _ => unimplemented!(),
        }
    }

    fn dump_fragment_if_enabled(
        &self,
        sender: &Wallet,
        reciever: &Wallet,
        value: &Value,
        fragment: &Fragment,
        via: &dyn FragmentNode,
    ) -> Result<(), FragmentSenderError> {
        if let Some(dump_folder) = &self.setup.dump_fragments {
            FragmentExporter::new(dump_folder.to_path_buf())?
                .dump_to_file(fragment, value, sender, reciever, via)?;
        }
        Ok(())
    }

    pub fn send_fragment<A: FragmentNode + SyncNode + Sized>(
        &self,
        sender: &mut Wallet,
        fragment: Fragment,
        node: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        self.wait_for_node_sync_if_enabled(node)
            .map_err(|e| FragmentSenderError::SyncNodeError(e))?;
        for _ in 0..self.setup.attempts_count() {
            let check = node.send_fragment(fragment.clone());

            if self.setup.fire_and_forget() {
                self.confirm_transaction_if_enabled(sender);
                return Ok(MemPoolCheck::new(fragment.id()));
            }

            if let Err(send_fragment_error) = check {
                if self.setup.ignore_any_errors() {
                    return Ok(MemPoolCheck::new(fragment.id()));
                }
                return Err(FragmentSenderError::SendFragmentError(send_fragment_error));
            }

            if let Err(err) = self.verify(&check.unwrap(), node) {
                if self.setup.ignore_any_errors() {
                    println!("Ignoring error: {:?}", err);
                    return Ok(MemPoolCheck::new(fragment.id()));
                }
                println!(
                    "Error while sending fragment {:?}. Retrying if possible...",
                    err
                );
                continue;
            }
            self.confirm_transaction_if_enabled(sender);
            return Ok(MemPoolCheck::new(fragment.id()));
        }

        if self.setup.ignore_any_errors() {
            self.confirm_transaction_if_enabled(sender);
            return Ok(MemPoolCheck::new(fragment.id()));
        }

        Err(FragmentSenderError::TooManyAttemptsFailed {
            attempts: self.setup.attempts_count(),
            alias: FragmentNode::alias(node).to_string(),
        })
    }

    fn confirm_transaction_if_enabled(&self, sender: &mut Wallet) {
        if self.setup.auto_confirm() {
            sender.confirm_transaction();
        }
    }

    fn wait_for_node_sync_if_enabled<A: FragmentNode + SyncNode + Sized>(
        &self,
        node: &A,
    ) -> Result<(), SyncNodeError> {
        if self.setup.no_sync_nodes() {
            return Ok(());
        }

        let nodes_length = (self.setup.sync_nodes().len() + 1) as u64;
        ensure_node_is_in_sync_with_others(
            node,
            self.setup.sync_nodes(),
            SyncWaitParams::network_size(nodes_length, 2).into(),
            "waiting for node to be in sync before sending transaction",
        )
    }
}
