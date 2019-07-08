use super::address::AddressData;
use crate::{
    account::SpendingCounter,
    block::HeaderHash,
    fee::LinearFee,
    key::EitherEd25519SecretKey,
    ledger::OutputAddress,
    message::Message,
    transaction::{AuthenticatedTransaction, Input, NoExtra, Output, Transaction, Witness},
    txbuilder::{OutputPolicy, TransactionBuilder as Builder},
};
use chain_addr::{Address, Kind};

pub struct TransactionBuilder {
    inputs: Vec<Input>,
    outputs: Vec<OutputAddress>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        TransactionBuilder {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn with_inputs<'a>(&'a mut self, inputs: Vec<Input>) -> &'a mut Self {
        self.inputs.extend(inputs.iter().cloned());
        self
    }

    pub fn with_input<'a>(&'a mut self, input: Input) -> &'a mut Self {
        self.inputs.push(input);
        self
    }

    pub fn with_output<'a>(&'a mut self, output: OutputAddress) -> &'a mut Self {
        self.outputs.push(output);
        self
    }

    pub fn with_outputs<'a>(&'a mut self, outputs: Vec<OutputAddress>) -> &'a mut Self {
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

    pub fn authenticate_with_policy(
        &mut self,
        output_policy: OutputPolicy,
    ) -> TransactionAuthenticator {
        let transaction = Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: NoExtra,
        };
        let tx_builder = Builder::from(transaction);
        let fee_algorithm = LinearFee::new(0, 0, 0);
        let (_, tx) = tx_builder.finalize(fee_algorithm, output_policy).unwrap();

        self.inputs = tx
            .inputs
            .clone()
            .into_iter()
            .map(|input| Input {
                index_or_account: input.index_or_account,
                value: input.value,
                input_ptr: input.input_ptr,
            })
            .collect();
        self.outputs = tx
            .outputs
            .clone()
            .into_iter()
            .map(|output| Output {
                address: output.address,
                value: output.value,
            })
            .collect();

        TransactionAuthenticator::new(tx)
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

    pub fn with_witnesses<'a>(
        &'a mut self,
        block0: &HeaderHash,
        addreses_data: &'a Vec<AddressData>,
    ) -> &'a mut Self {
        for address in addreses_data {
            self.with_witness(&block0, &address);
        }
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
