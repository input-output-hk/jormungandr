use crate::{
    block::HeaderHash,
    fee::LinearFee,
    fragment::Fragment,
    ledger::OutputAddress,
    testing::{address::AddressData, witness_builder},
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

    pub fn with_witnesses(
        &mut self,
        block0: &HeaderHash,
        addreses_data: &Vec<AddressData>,
    ) -> &mut Self {
        for address in addreses_data {
            self.with_witness(&block0, &address);
        }
        self
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
        Fragment::Transaction(signed_tx)
    }

    pub fn seal(&self) -> AuthenticatedTransaction<Address, NoExtra> {
        AuthenticatedTransaction {
            transaction: self.transaction.clone(),
            witnesses: self.witnesses.clone(),
        }
    }
}
