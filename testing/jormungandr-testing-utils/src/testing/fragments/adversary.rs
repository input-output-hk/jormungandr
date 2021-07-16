use super::{FragmentBuilderError, FragmentExporter, FragmentExporterError};
use crate::{
    testing::{
        ensure_node_is_in_sync_with_others,
        fragments::node::{FragmentNode, MemPoolCheck},
        FragmentVerifier, SyncNode, SyncNodeError, SyncWaitParams,
    },
    wallet::Wallet,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{
    certificate::{Certificate, PoolId},
    fee::LinearFee,
    fragment::Fragment,
    testing::{build_owner_stake_full_delegation, FaultTolerantTxCertBuilder, TestGen},
    transaction::{Input, Output, TransactionSignDataHash, TxBuilder, Witness},
};
use chain_impl_mockchain::{fee::FeeAlgorithm, ledger::OutputAddress, value::Value};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentStatus},
};
use rand::{thread_rng, Rng};
use std::{path::PathBuf, time::Duration};

/// Send malformed transactions
/// Only supports account based wallets
#[derive(custom_debug::Debug, thiserror::Error)]
pub enum AdversaryFragmentSenderError {
    #[error("fragment sent to node: {alias} is not in rejected, date: '{date}', block: '{block}'")]
    FragmentNotRejected {
        alias: String,
        date: BlockDate,
        block: Hash,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error("cannot build fragment")]
    FragmentBuilderError(#[from] super::FragmentBuilderError),
    #[error("cannot send fragment")]
    SendFragmentError(#[from] super::node::FragmentNodeError),
    #[error("cannot send fragment")]
    FragmentVerifierError(#[from] super::FragmentVerifierError),
    #[error("fragment exporter error")]
    FragmentExporterError(#[from] FragmentExporterError),
    #[error("cannot sync node before sending fragment")]
    SyncNodeError(#[from] crate::testing::SyncNodeError),
}

impl AdversaryFragmentSenderError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::AdversaryFragmentSenderError::*;
        let maybe_logs = match self {
            FragmentNotRejected { logs, .. } => Some(logs),
            _ => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

#[derive(Clone)]
pub struct AdversaryFragmentSenderSetup<'a, A: SyncNode + Send> {
    pub verify: bool,
    pub sync_nodes: Vec<&'a A>,
    pub dump_fragments: Option<PathBuf>,
}

impl<'a, A: SyncNode + Send> AdversaryFragmentSenderSetup<'a, A> {
    pub fn sync_before(nodes: Vec<&'a A>) -> Self {
        Self {
            verify: true,
            sync_nodes: nodes,
            dump_fragments: None,
        }
    }

    pub fn verify(&self) -> bool {
        self.verify
    }

    pub fn no_sync_nodes(&self) -> bool {
        self.sync_nodes.is_empty()
    }

    pub fn sync_nodes(&self) -> Vec<&'a A> {
        self.sync_nodes.clone()
    }
}

impl<'a> AdversaryFragmentSenderSetup<'a, DummySyncNode> {
    pub fn no_verify() -> Self {
        Self {
            verify: false,
            sync_nodes: vec![],
            dump_fragments: None,
        }
    }

    pub fn with_verify() -> Self {
        Self {
            verify: true,
            sync_nodes: vec![],
            dump_fragments: None,
        }
    }

    pub fn dump_into(path: PathBuf, verify: bool) -> Self {
        Self {
            verify,
            sync_nodes: vec![],
            dump_fragments: Some(path),
        }
    }
}

use super::DummySyncNode;

#[derive(Clone)]
pub struct AdversaryFragmentSender<'a, S: SyncNode + Send> {
    block0_hash: Hash,
    fees: LinearFee,
    setup: AdversaryFragmentSenderSetup<'a, S>,
}

impl<'a, S: SyncNode + Send> AdversaryFragmentSender<'a, S> {
    pub fn new(
        block0_hash: Hash,
        fees: LinearFee,
        setup: AdversaryFragmentSenderSetup<'a, S>,
    ) -> Self {
        Self {
            block0_hash,
            fees,
            setup,
        }
    }

    pub fn block0_hash(&self) -> Hash {
        self.block0_hash
    }

    pub fn fees(&self) -> LinearFee {
        self.fees
    }

    pub fn send_random_faulty_transaction<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
    ) -> Result<MemPoolCheck, AdversaryFragmentSenderError> {
        let fragment = self.random_faulty_transaction(from, to);
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(fragment, via)
    }

    fn random_faulty_transaction(&self, from: &Wallet, to: &Wallet) -> Fragment {
        let mut rng = thread_rng();
        let option: u8 = rng.gen();
        let faulty_tx_builder = FaultyTransactionBuilder::new(self.block0_hash, self.fees);
        match option % 7 {
            0 => faulty_tx_builder.wrong_block0_hash(from, to),
            1 => faulty_tx_builder.no_input(to),
            2 => faulty_tx_builder.no_output(from),
            3 => faulty_tx_builder.unbalanced(from, to),
            4 => faulty_tx_builder.empty(),
            5 => faulty_tx_builder.wrong_counter(from, to),
            6 => faulty_tx_builder.no_witnesses(from, to),
            _ => unreachable!(),
        }
    }

    pub fn send_transactions_with_invalid_counter<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        n: usize,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
    ) -> Result<Vec<MemPoolCheck>, AdversaryFragmentSenderError> {
        let mut mem_checks = Vec::new();
        let faulty_tx_builder = FaultyTransactionBuilder::new(self.block0_hash, self.fees);

        for _ in 0..n {
            let fragment = faulty_tx_builder.wrong_counter(from, to);
            self.dump_fragment_if_enabled(from, &fragment, via)?;
            mem_checks.push(self.send_fragment(fragment, via)?);
            from.confirm_transaction();
        }
        Ok(mem_checks)
    }

    pub fn send_all_faulty_transactions<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
    ) -> Result<Vec<MemPoolCheck>, AdversaryFragmentSenderError> {
        let faulty_tx_builder = FaultyTransactionBuilder::new(self.block0_hash, self.fees);
        let mut mem_checks = Vec::new();

        for fragment in vec![
            faulty_tx_builder.wrong_block0_hash(from, to),
            faulty_tx_builder.no_input(to),
            faulty_tx_builder.no_output(from),
            faulty_tx_builder.unbalanced(from, to),
            faulty_tx_builder.empty(),
            faulty_tx_builder.wrong_counter(from, to),
            faulty_tx_builder.no_witnesses(from, to),
        ] {
            self.dump_fragment_if_enabled(from, &fragment, via)?;
            mem_checks.push(self.send_fragment(fragment, via)?);
        }
        Ok(mem_checks)
    }

    pub fn send_faulty_full_delegation<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: PoolId,
        via: &A,
    ) -> Result<MemPoolCheck, AdversaryFragmentSenderError> {
        let cert = build_owner_stake_full_delegation(to);
        let fragment = self.random_faulty_cert(from, cert)?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(fragment, via)
    }

    fn random_faulty_cert(
        &self,
        from: &Wallet,
        cert: Certificate,
    ) -> Result<Fragment, FragmentBuilderError> {
        let mut rng = thread_rng();
        let option: u8 = rng.gen();
        let faulty_tx_cert_builder = FaultTolerantTxCertBuilder::new(
            self.block0_hash.into_hash(),
            self.fees,
            cert,
            from.clone().into(),
        );
        match option % 3 {
            0 => Ok(faulty_tx_cert_builder.transaction_no_witness()),
            1 => Ok(faulty_tx_cert_builder.transaction_input_to_low()),
            2 => Ok(faulty_tx_cert_builder.transaction_with_output_only()),
            _ => unreachable!(),
        }
    }

    pub fn send_faulty_transactions<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
    ) -> Result<Vec<MemPoolCheck>, AdversaryFragmentSenderError> {
        self.send_faulty_transactions_with_iteration_delay(
            n,
            &mut wallet1,
            &wallet2,
            node,
            std::time::Duration::from_secs(0),
        )
    }

    pub fn send_faulty_transactions_with_iteration_delay<
        A: FragmentNode + SyncNode + Sized + Send,
    >(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
        duration: Duration,
    ) -> Result<Vec<MemPoolCheck>, AdversaryFragmentSenderError> {
        let mut mem_checks = Vec::new();
        for _ in 0..n {
            mem_checks.push(self.send_random_faulty_transaction(&mut wallet1, &wallet2, node)?);
            std::thread::sleep(duration);
        }
        Ok(mem_checks)
    }

    fn verify<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        check: &MemPoolCheck,
        node: &A,
    ) -> Result<(), AdversaryFragmentSenderError> {
        let verifier = FragmentVerifier;
        match verifier.wait_fragment(
            Duration::from_secs(2),
            check.clone(),
            Default::default(),
            node,
        )? {
            FragmentStatus::Rejected { .. } => Ok(()),
            FragmentStatus::InABlock { date, block } => {
                Err(AdversaryFragmentSenderError::FragmentNotRejected {
                    alias: FragmentNode::alias(node).to_string(),
                    date,
                    block,
                    logs: FragmentNode::log_content(node),
                })
            }
            _ => unimplemented!(),
        }
    }

    fn dump_fragment_if_enabled(
        &self,
        sender: &Wallet,
        fragment: &Fragment,
        via: &dyn FragmentNode,
    ) -> Result<(), AdversaryFragmentSenderError> {
        if let Some(dump_folder) = &self.setup.dump_fragments {
            FragmentExporter::new(dump_folder.to_path_buf())?
                .dump_to_file(fragment, sender, via)?;
        }
        Ok(())
    }

    pub fn send_fragment<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        fragment: Fragment,
        node: &A,
    ) -> Result<MemPoolCheck, AdversaryFragmentSenderError> {
        self.wait_for_node_sync_if_enabled(node)
            .map_err(AdversaryFragmentSenderError::SyncNodeError)?;

        let check = node.send_fragment(fragment.clone());

        if !self.setup.verify() {
            return Ok(MemPoolCheck::new(fragment.id()));
        }
        self.verify(&check?, node)?;
        Ok(MemPoolCheck::new(fragment.id()))
    }

    fn wait_for_node_sync_if_enabled<A: FragmentNode + SyncNode + Sized + Send>(
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

pub struct FaultyTransactionBuilder {
    block0_hash: Hash,
    fees: LinearFee,
}

impl FaultyTransactionBuilder {
    pub fn new(block0_hash: Hash, fees: LinearFee) -> Self {
        Self { block0_hash, fees }
    }

    pub fn wrong_block0_hash(&self, from: &Wallet, to: &Wallet) -> Fragment {
        let input_value = self.fees.calculate(None, 1, 1).saturating_add(Value(1u64));
        let input = from.add_input_with_value(input_value.into());
        let output = OutputAddress::from_address(to.address().into(), Value(1u64));
        self.transaction_to(&[input], &[output], |sign_data| {
            vec![from.mk_witness(&Hash::from_hash(TestGen::hash()), sign_data)]
        })
    }

    pub fn no_witnesses(&self, from: &Wallet, to: &Wallet) -> Fragment {
        let input_value = self.fees.calculate(None, 1, 1).saturating_add(Value(1u64));
        let input = from.add_input_with_value(input_value.into());
        let output = OutputAddress::from_address(to.address().into(), Value(1u64));
        self.transaction_to(&[input], &[output], |_sign_data| Vec::new())
    }

    pub fn no_input(&self, to: &Wallet) -> Fragment {
        let output = Output::from_address(to.address().into(), Value(1u64));
        self.transaction_to(&[], &[output], |_sign_data| Vec::new())
    }

    pub fn no_output(&self, from: &Wallet) -> Fragment {
        let input_value = self.fees.calculate(None, 1, 1).saturating_add(Value(1u64));
        let input = from.add_input_with_value(input_value.into());
        self.transaction_to(&[input], &[], |sign_data| {
            vec![from.mk_witness(&self.block0_hash, sign_data)]
        })
    }

    pub fn unbalanced(&self, from: &Wallet, to: &Wallet) -> Fragment {
        let input = from.add_input_with_value(1u64.into());
        let output = Output::from_address(to.address().into(), Value(2u64));
        self.transaction_to(&[input], &[output], |sign_data| {
            vec![from.mk_witness(&self.block0_hash, sign_data)]
        })
    }

    pub fn empty(&self) -> Fragment {
        self.transaction_to(&[], &[], |_sign_data| Vec::new())
    }

    pub fn wrong_counter(&self, from: &Wallet, to: &Wallet) -> Fragment {
        let input_value: Value = (self.fees.calculate(None, 1, 1) + Value(1u64)).unwrap();
        let input = from.add_input_with_value(input_value.into());
        let output = OutputAddress::from_address(to.address().into(), Value(1u64));
        self.transaction_to(&[input], &[output], |sign_data| {
            let mut false_from = from.clone();
            false_from.confirm_transaction();
            vec![false_from.mk_witness(&self.block0_hash, sign_data)]
        })
    }

    fn transaction_to<F: Fn(&TransactionSignDataHash) -> Vec<Witness>>(
        &self,
        inputs: &[Input],
        outputs: &[OutputAddress],
        make_witnesses: F,
    ) -> Fragment {
        let builder = TxBuilder::new().set_nopayload();
        let builder = builder.set_ios(inputs, outputs);
        let witnesses = make_witnesses(&builder.get_auth_data_for_witness().hash());
        let builder = builder.set_witnesses_unchecked(&witnesses);
        let tx = builder.set_payload_auth(&());
        Fragment::Transaction(tx)
    }
}
