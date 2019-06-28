use super::address::AddressData;
use chain_addr::{Address, Kind};
use crate::account::SpendingCounter;
use crate::block::HeaderHash;
use crate::key::EitherEd25519SecretKey;
use crate::message::Message;
use crate::transaction::Witness;
use crate::transaction::*;

pub struct TransactionBuilder {
    inputs: Vec<Input>,
    outputs: Vec<Output<Address>>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        TransactionBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
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

    pub fn authenticate(&self) -> TransactionAuthenticator {
        let transaction = Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: NoExtra,
        };
        TransactionAuthenticator::new(transaction)
    }
}

pub struct TransactionAuthenticator {
    witnesses: Vec<Witness>,
    transaction: Transaction<Address, NoExtra>,
}

impl TransactionAuthenticator {
    pub fn new(transaction: Transaction<Address, NoExtra>) -> Self {
        TransactionAuthenticator {
            witnesses: Vec::new(),
            transaction: transaction,
        }
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
            &self.transaction.hash(),
            secret_key,
        ));
        self
    }

    pub fn with_account_witness<'a>(
        &'a mut self,
        block0: &HeaderHash,
        spending_counter: &SpendingCounter,
        secret_key: &EitherEd25519SecretKey,
    ) -> &'a mut Self {
        self.witnesses.push(Witness::new_account(
            block0,
            &self.transaction.hash(),
            spending_counter,
            secret_key,
        ));
        self
    }

    pub fn as_utxos(&self) -> Vec<UtxoPointer> {
        let mut utxos = Vec::new();
        for (i, output) in self.transaction.outputs.iter().enumerate() {
            utxos.push(UtxoPointer {
                transaction_id: self.transaction.hash().clone(),
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
            transaction: self.transaction.clone(),
            witnesses: self.witnesses.clone(),
        }
    }
}
