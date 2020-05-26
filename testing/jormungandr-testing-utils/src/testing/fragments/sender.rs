use crate::{
    testing::{
        assure_node_in_sync,
        fragments::node::{FragmentNode, MemPoolCheck},
        FragmentSenderSetup, FragmentVerifier, SyncNode, SyncNodeError, SyncWaitParams,
    },
    wallet::Wallet,
};
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{FragmentStatus, Value},
};

use std::time::Duration;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FragmentSenderError {
    #[error("fragment sent to node: {alias} is not in block due to  '{reason}'. logs: {logs}")]
    FragmentNotInBlock {
        alias: String,
        reason: String,
        logs: String,
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
        self.send_fragment(fragment, via)
    }

    pub fn send_transactions<A: FragmentNode + SyncNode + Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        for _ in 0..n {
            self.send_transaction(&mut wallet1, &wallet2, node, value)?;
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
        for _ in 0..n {
            self.send_transaction(&mut wallet1, &wallet2, node, value.clone())?;
            wallet1.confirm_transaction();
            self.send_transaction(&mut wallet2, &wallet1, node, value.clone())?;
            wallet2.confirm_transaction();
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
                reason: reason,
                logs: FragmentNode::log_content(node),
            }),
            FragmentStatus::InABlock { .. } => Ok(()),
            _ => unimplemented!(),
        }
    }

    pub fn send_fragment<A: FragmentNode + SyncNode + Sized>(
        &self,
        fragment: Fragment,
        node: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        self.wait_for_node_sync_if_enabled(node)
            .map_err(|e| FragmentSenderError::SyncNodeError(e))?;
        for _ in 0..self.setup.attempts_count() {
            let check = node
                .send_fragment(fragment.clone())
                .map_err(|e| FragmentSenderError::SendFragmentError(e))?;
            if self.setup.no_verify() {
                return Ok(check);
            }

            if let Err(err) = self.verify(&check, node) {
                if self.setup.ignore_any_errors() {
                    println!("Ignoring error: {:?}", err);
                    return Ok(check);
                }
                println!(
                    "Error while sending fragment {:?}. Retrying if possible...",
                    err
                );
                continue;
            }
            return Ok(check);
        }

        Err(FragmentSenderError::TooManyAttemptsFailed {
            attempts: self.setup.attempts_count(),
            alias: FragmentNode::alias(node).to_string(),
        })
    }

    fn wait_for_node_sync_if_enabled<A: FragmentNode + SyncNode + Sized>(
        &self,
        node: &A,
    ) -> Result<(), SyncNodeError> {
        if self.setup.no_sync_nodes() {
            return Ok(());
        }

        let nodes_length = (self.setup.sync_nodes().len() + 1) as u64;
        assure_node_in_sync(
            node,
            self.setup.sync_nodes(),
            SyncWaitParams::network_size(nodes_length, 2).into(),
            "waiting for node to be in sync before sending transaction",
        )
    }
}
