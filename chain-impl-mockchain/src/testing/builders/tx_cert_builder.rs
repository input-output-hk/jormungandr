use crate::{
    block::HeaderHash,
    certificate::Certificate,
    fee::FeeAlgorithm,
    fragment::Fragment,
    testing::data::AddressData,
    testing::witness_builder,
    transaction::{AuthenticatedTransaction, Input, Output, Transaction, Witness},
    txbuilder::{self, GeneratedTransaction, OutputPolicy, TransactionBuilder},
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

    pub fn finalize<FA>(&mut self, fee_algorithm: FA, output_policy: OutputPolicy) -> &mut Self
    where
        FA: FeeAlgorithm<Transaction<Address, Certificate>>,
    {
        let transaction = self.build_transaction();
        let builder = TransactionBuilder::from(transaction);
        let (_balance, tx) = builder.finalize(fee_algorithm, output_policy).unwrap();
        self.update_tx(tx);
        self
    }

    pub fn authenticate(&mut self) -> TransactionCertAuthenticator {
        let transaction = self.build_transaction();
        TransactionCertAuthenticator::new(transaction)
    }

    fn build_transaction(&self) -> Transaction<Address, Certificate> {
        Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: self
                .certificate
                .clone()
                .expect("Cannot build transaction: Certificate in None"),
        }
    }
}

pub struct TransactionCertAuthenticator {
    witnesses: Vec<Witness>,
    transaction: Transaction<Address, Certificate>,
}

impl TransactionCertAuthenticator {
    pub fn new(transaction: Transaction<Address, Certificate>) -> Self {
        TransactionCertAuthenticator {
            witnesses: Vec::new(),
            transaction: transaction,
        }
    }

    pub fn with_witness(&mut self, block0: &HeaderHash, address_data: &AddressData) -> &mut Self {
        self.witnesses.push(witness_builder::make_witness(
            &block0,
            &address_data,
            self.transaction.hash(),
        ));
        self
    }

    pub fn as_message(&self) -> Fragment {
        let signed_tx = self.seal();
        Fragment::Certificate(signed_tx)
    }

    pub fn seal(&self) -> AuthenticatedTransaction<Address, Certificate> {
        let cert_finalizer = txbuilder::TransactionFinalizer::new_cert(self.transaction.clone());
        match cert_finalizer.build() {
            Ok(GeneratedTransaction::Type2(auth)) => auth,
            _ => panic!("internal error: this should be a certificate not transaction"),
        }
    }
}
