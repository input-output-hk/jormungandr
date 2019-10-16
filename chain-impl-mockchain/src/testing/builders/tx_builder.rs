use crate::{
    block::HeaderId,
    fee::LinearFee,
    fragment::Fragment,
    ledger::OutputAddress,
    testing::{data::AddressData, witness_builder},
    transaction::{
        AuthenticatedTransaction, Input, NoExtra, Transaction, TransactionSignDataHash, Witness,
    },
    txbuilder::{OutputPolicy, TransactionBuilder as Builder},
};
use chain_addr::Address;

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

    pub fn with_inputs(&mut self, inputs: Vec<Input>) -> &mut Self {
        self.inputs.extend(inputs.iter().cloned());
        self
    }

    pub fn with_input(&mut self, input: Input) -> &mut Self {
        self.inputs.push(input);
        self
    }

    pub fn with_output(&mut self, output: OutputAddress) -> &mut Self {
        self.outputs.push(output);
        self
    }

    pub fn with_outputs(&mut self, outputs: Vec<OutputAddress>) -> &mut Self {
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

    pub fn authenticate_with(
        &mut self,
        fee_algorithm: LinearFee,
        output_policy: OutputPolicy,
    ) -> TransactionAuthenticator {
        let transaction = Transaction {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            extra: NoExtra,
        };
        let tx_builder = Builder::from(transaction);
        let (_, tx) = tx_builder
            .seal_with_output_policy(fee_algorithm, output_policy)
            .unwrap();

        self.inputs = tx.inputs.iter().cloned().collect();

        self.outputs = tx.outputs.iter().cloned().collect();

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

    pub fn transaction_hash(&self) -> TransactionSignDataHash {
        self.transaction.hash()
    }

    pub fn with_witnesses(
        &mut self,
        block0: &HeaderId,
        addreses_data: &Vec<AddressData>,
    ) -> &mut Self {
        for address in addreses_data {
            self.with_witness(&block0, &address);
        }
        self
    }

    pub fn with_witness(&mut self, block0: &HeaderId, address_data: &AddressData) -> &mut Self {
        self.witnesses.push(witness_builder::make_witness(
            &block0,
            &address_data,
            self.transaction_hash(),
        ));
        self
    }

    pub fn with_witness_from(&mut self, witness: Witness) -> &mut Self {
        self.witnesses.push(witness);
        self
    }

    pub fn as_message(&self) -> Fragment {
        let signed_tx = self.seal();
        Fragment::Transaction(signed_tx)
    }

    pub fn seal(&self) -> AuthenticatedTransaction<Address, NoExtra> {
        AuthenticatedTransaction {
            transaction: self.transaction.clone(),
            witnesses: self.witnesses.clone(),
        }
    }
}
