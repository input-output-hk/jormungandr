use crate::{
    block::HeaderHash,
    certificate::Certificate,
    fee::FeeAlgorithm,
    fragment::Fragment,
    testing::data::AddressData,
    testing::witness_builder,
    transaction::{
        AuthenticatedTransaction, Input, Output, Transaction, TransactionSignDataHash, Witness,
    },
    txbuilder::{self, OutputPolicy, TransactionBuilder},
};
use chain_addr::Address;

#[derive(Debug)]
pub struct TransactionCertBuilder {
    certificate: Option<Certificate>,
    inputs: Vec<Input>,
    outputs: Vec<Output<Address>>,
}

impl TransactionCertBuilder {
    pub fn new() -> Self {
        TransactionCertBuilder {
            certificate: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn with_certificate(&mut self, cert: Certificate) -> &mut Self {
        self.certificate = Some(cert);
        self
    }

    fn update_tx<Extra>(&mut self, tx: Transaction<Address, Extra>) {
        self.inputs = tx.inputs.clone();
        self.outputs = tx.outputs.clone();
    }

    pub fn seal<FA>(&mut self, fee_algorithm: FA, output_policy: OutputPolicy) -> &mut Self
    where
        FA: FeeAlgorithm<Transaction<Address, Option<Certificate>>>,
    {
        let transaction = self.build_transaction();
        let builder = TransactionBuilder::from(transaction);
        let (_balance, tx) = builder
            .seal_with_output_policy(fee_algorithm, output_policy)
            .unwrap();
        self.update_tx(tx);
        self
    }

    pub fn authenticate(&mut self) -> TransactionCertAuthenticator {
        let transaction = self.build_transaction();
        TransactionCertAuthenticator::new(transaction)
    }

    fn build_transaction(&self) -> Transaction<Address, Option<Certificate>> {
        Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: self.certificate.clone(),
        }
    }
}

pub struct TransactionCertAuthenticator {
    witnesses: Vec<Witness>,
    transaction: Transaction<Address, Option<Certificate>>,
}

impl TransactionCertAuthenticator {
    pub fn new(transaction: Transaction<Address, Option<Certificate>>) -> Self {
        TransactionCertAuthenticator {
            witnesses: Vec::new(),
            transaction: transaction,
        }
    }

    pub fn with_witness(&mut self, block0: &HeaderHash, address_data: &AddressData) -> &mut Self {
        self.witnesses.push(witness_builder::make_witness(
            &block0,
            &address_data,
            self.hash(),
        ));
        self
    }

    pub fn hash(&self) -> TransactionSignDataHash {
        txbuilder::TransactionFinalizer::new(self.transaction.clone()).get_tx_sign_data_hash()
    }

    pub fn as_message(&self) -> Fragment {
        let cert_finalizer = self.build_finalizer();
        cert_finalizer.to_fragment().unwrap()
    }

    pub fn finalize(&self) -> AuthenticatedTransaction<Address, Option<Certificate>> {
        let cert_finalizer = self.build_finalizer();
        cert_finalizer.finalize().unwrap()
    }

    fn build_finalizer(&self) -> txbuilder::TransactionFinalizer {
        let mut cert_finalizer = txbuilder::TransactionFinalizer::new(self.transaction.clone());
        for (index, witness) in self.witnesses.iter().cloned().enumerate() {
            cert_finalizer
                .set_witness(index, witness)
                .expect("cannot set witness");
        }
        cert_finalizer
    }
}
