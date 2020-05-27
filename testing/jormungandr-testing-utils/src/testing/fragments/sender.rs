use chain_impl_mockchain::{
    fee::LinearFee,
    fragment::{Fragment, FragmentId},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{FragmentStatus, Value},
};

use crate::{
    testing::{
        fragments::node::{FragmentNode, MemPoolCheck},
        FragmentVerifier, FragmentVerifierError,
    },
    wallet::Wallet,
};

use std::{thread, time::Duration};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FragmentSenderError {
    #[error("fragment sent to node: {alias} is not in block '{status:?}'. logs: {logs}")]
    FragmentNotInBlock {
        alias: String,
        status: FragmentStatus,
        logs: String,
    },
    #[error("transaction already balanced")]
    FragmentIsPendingForTooLong,
    #[error(
        "fragment sent to node: {alias} is not in in fragment pool :({fragment_id}). logs: {logs}"
    )]
    FragmentNoInMemPoolLogs {
        alias: String,
        fragment_id: FragmentId,
        logs: String,
    },
    #[error("fragment verifier error")]
    FragmentVerifierError(#[from] super::FragmentVerifierError),
    #[error("cannot send fragment")]
    SendFragmentError(#[from] super::node::FragmentNodeError),
    #[error("wallet error")]
    WalletError(#[from] crate::wallet::WalletError),
}

pub struct FragmentSender {
    block0_hash: Hash,
    fees: LinearFee,
}

impl FragmentSender {
    pub fn new(block0_hash: Hash, fees: LinearFee) -> Self {
        Self { block0_hash, fees }
    }

    pub fn send_transaction<A: FragmentNode + ?Sized>(
        &self,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
        value: Value,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let address = to.address();
        let fragment = from.transaction_to(&self.block0_hash, &self.fees, address, value)?;
        Ok(via.send_fragment(fragment)?)
    }

    pub fn send_transactions_ignore_errors<A: FragmentNode + ?Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        for _ in 0..n {
            let check = self.send_transaction(&mut wallet1, &wallet2, node, value);
            if let Err(err) = check {
                println!("ignoring error : {:?}", err);
            }
            thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }

    pub fn send_transactions_round_trip<A: FragmentNode + ?Sized>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        mut wallet2: &mut Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        let verifier = FragmentVerifier;

        for _ in 0..n {
            let check = self.send_transaction(&mut wallet1, &wallet2, node, value.clone())?;
            verifier.wait_and_verify_is_in_block(Duration::from_secs(2), check, node)?;
            wallet1.confirm_transaction();
            let check = self.send_transaction(&mut wallet2, &wallet1, node, value.clone())?;
            verifier.wait_and_verify_is_in_block(Duration::from_secs(2), check, node)?;
            wallet2.confirm_transaction();
        }
        Ok(())
    }

    pub fn send_fragment_and_verify_is_in_block<A: FragmentNode + ?Sized>(
        &self,
        fragment: Fragment,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let verifier = FragmentVerifier;
        let check = node.send_fragment(fragment)?;
        verifier.wait_and_verify_is_in_block(Duration::from_secs(2), check, node)
    }
}
