use super::address::AddressData;
use chain_addr::{Address, Kind};
use chain_impl_mockchain::account::SpendingCounter;
use chain_impl_mockchain::block::HeaderHash;
use chain_impl_mockchain::key::EitherEd25519SecretKey;
use chain_impl_mockchain::message::Message;
use chain_impl_mockchain::transaction::Witness;
use chain_impl_mockchain::transaction::*;

pub struct TransactionBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output<Address>>,
    witnesses: Vec<Witness>,
    transaction_id: Option<TransactionId>,
    transaction: Option<Transaction<Address, NoExtra>>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        TransactionBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
            witnesses: Vec::new(),
            transaction_id: None,
            transaction: None,
        }
    }

    pub fn with_input<'a>(&'a mut self, input: Input) -> &'a mut Self {
        self.inputs.push(input);
        self
    }

    pub fn with_output<'a>(&'a mut self, output: Output<Address>) -> &'a mut Self {
        self.outputs.push(output);
        self
    }

    pub fn with_outputs<'a>(&'a mut self, outputs: Vec<Output<Address>>) -> &'a mut Self {
        self.outputs.extend(outputs.iter().cloned());
        self
    }

    pub fn finalize<'a>(&'a mut self) -> &'a mut Self {
        let transaction = Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: NoExtra,
        };
        self.transaction_id = Some(transaction.hash());
        self.transaction = Some(transaction);
        self
    }

    pub fn with_witness<'a>(
        &'a mut self,
        block0: &HeaderHash,
        addres_data: &AddressData,
    ) -> &'a mut Self {
        match addres_data.address.kind() {
            Kind::Account(_) => self.with_account_witness(
                block0,
                &addres_data.spending_counter.unwrap(),
                &addres_data.private_key,
            ),
            _ => self.with_utxo_witness(block0, &addres_data.private_key),
        }
    }

    pub fn with_utxo_witness<'a>(
        &'a mut self,
        block0: &HeaderHash,
        secret_key: &EitherEd25519SecretKey,
    ) -> &'a mut Self {
        self.witnesses.push(Witness::new_utxo(
            block0,
            &self.transaction_id.unwrap(),
            secret_key,
        ));
        self
    }

    pub fn with_account_witness<'a>(
        &'a mut self,
        block0: &HeaderHash,
        spending_counter: &SpendingCounter,
        secret_key: &EitherEd25519SecretKey,
    ) -> &'a mut TransactionBuilder {
        self.witnesses.push(Witness::new_account(
            block0,
            &self.transaction_id.unwrap(),
            spending_counter,
            secret_key,
        ));
        self
    }

    pub fn as_utxos(&self) -> Vec<UtxoPointer> {
        let mut utxos = Vec::new();
        for (i, output) in self.outputs.iter().enumerate() {
            utxos.push(UtxoPointer {
                transaction_id: self.transaction_id.unwrap().clone(),
                output_index: i as u8,
                value: output.value.clone(),
            });
        }
        utxos
    }

    pub fn as_message(&self) -> Message {
        let signed_tx = self.seal();
        Message::Transaction(signed_tx)
    }

    pub fn seal(&self) -> AuthenticatedTransaction<Address, NoExtra> {
        AuthenticatedTransaction {
            transaction: self.transaction.clone().unwrap(),
            witnesses: self.witnesses.clone(),
        }
    }
}
