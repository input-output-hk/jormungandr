use crate::{
    account::SpendingCounter,
    fragment::{Fragment, FragmentId},
    header::HeaderId,
    testing::{ledger::TestLedger, KeysDb, data::AddressDataValue},
    transaction::{
        Input, InputEnum, NoExtra, Output, Transaction, TxBuilder, UnspecifiedAccountIdentifier,
        UtxoPointer, Witness,
    },
    value::Value,
    fee::FeeAlgorithm
};
use chain_addr::{Address, Kind};

pub struct TestTxBuilder {
    block0_hash: HeaderId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestTx {
    tx: Transaction<NoExtra>,
}

impl TestTx {

    pub fn new(tx: Transaction<NoExtra>) -> Self {
        TestTx { tx }
    }

    pub fn get_fragment_id(&self) -> FragmentId {
        self.clone().get_fragment().hash()
    }

    pub fn get_fragment(self) -> Fragment {
        Fragment::Transaction(self.tx)
    }

    pub fn get_tx(self) -> Transaction<NoExtra> {
        self.tx
    }
}

impl TestTxBuilder {
    pub fn new(block0_hash: &HeaderId) -> Self {
        Self {
            block0_hash: block0_hash.clone(),
        }
    }

    pub fn move_from_faucet(self, test_ledger: &mut TestLedger, destination: &Address, value: &Value) -> TestTx {
        assert_eq!(test_ledger.faucets.len(),1,"method can be used only for single faucet ledger");
        let mut faucet = test_ledger.faucets.iter().cloned().next().as_mut().expect("test ledger with no faucet configured").clone();
        let fee = test_ledger.fee().fees_for_inputs_outputs(1u8,1u8);
        let output_value = (*value - fee).expect("input value is smaller than fee");
        let inputs = vec![faucet.clone().make_input_with_value(test_ledger.find_utxo_for_address(&faucet.clone().into()),&value)];
        let outputs = vec![Output { address: destination.clone(), value: output_value }];
        let tx_builder = TxBuilder::new()
            .set_payload(&NoExtra)
            .set_ios(&inputs, &outputs);

        let witness = faucet.make_witness(&self.block0_hash,tx_builder.get_auth_data_for_witness());
        let witnesses = vec![witness];

        let tx = tx_builder
            .set_witnesses(&witnesses)
            .set_payload_auth(&());
        TestTx { tx }
    }

    pub fn move_to_outputs_from_faucet(self, test_ledger: &mut TestLedger, destination: &[Output<Address>]) -> TestTx {
        assert_eq!(test_ledger.faucets.len(),1,"method can be used only for single faucet ledger");
        let mut faucet = test_ledger.faucets.iter().next().as_mut().expect("test ledger with no faucet configured").clone();
        let input_val = Value::sum(destination.iter().map(|o| o.value)).unwrap();
        let inputs = vec![faucet.clone().make_input_with_value(test_ledger.find_utxo_for_address(&faucet.clone().into()),&input_val)];
        let tx_builder = TxBuilder::new()
            .set_payload(&NoExtra)
            .set_ios(&inputs, &destination);

        let witness = faucet.make_witness(&self.block0_hash,tx_builder.get_auth_data_for_witness());
        let witnesses = vec![witness];

        let tx = tx_builder
            .set_witnesses(&witnesses)
            .set_payload_auth(&());
        TestTx { tx }
    }

    pub fn move_all_funds(self, test_ledger: &mut TestLedger, source: &AddressDataValue, destination: &AddressDataValue) -> TestTx {
        let mut keys_db = KeysDb::empty();
        keys_db.add_key(source.private_key());
        keys_db.add_key(destination.private_key());
        self.move_funds(test_ledger,&source,&destination,&source.value)
    }

    pub fn move_funds(self, test_ledger: &mut TestLedger, source: &AddressDataValue, destination: &AddressDataValue, value: &Value) -> TestTx {
        let mut keys_db = KeysDb::empty();
        keys_db.add_key(source.private_key());
        keys_db.add_key(destination.private_key());

        let fee = test_ledger.fee().fees_for_inputs_outputs(1u8,1u8);
        let output_value = (*value - fee).expect("input value is smaller than fee");

        self.inputs_to_outputs(&keys_db,test_ledger,&[source.make_output_with_value(&value)],&[destination.make_output_with_value(&output_value)])
    }
    
    pub fn move_funds_multiple(self,test_ledger: &mut TestLedger, sources: &Vec<AddressDataValue>, destinations: &Vec<AddressDataValue>) -> TestTx {
        let mut keys_db = KeysDb::empty();

        for source in sources {
            keys_db.add_key(source.private_key())
        }
        for destination in destinations {
            keys_db.add_key(destination.private_key())
        }

        let source_outputs: Vec<Output<Address>>  = sources.iter().cloned().map(|x| x.make_output()).collect();
        let destination_outputs: Vec<Output<Address>> = destinations.iter().cloned().map(|x| x.make_output()).collect();

        self.inputs_to_outputs(&keys_db,test_ledger,&source_outputs,&destination_outputs)
    }

    pub fn inputs_to_outputs(
        self,
        kdb: &KeysDb,
        test_ledger: &mut TestLedger,
        sources: &[Output<Address>],
        destination: &[Output<Address>],
    ) -> TestTx {
        let inputs: Vec<_> = sources
            .iter()
            .map(|out| {
                match out.address.kind() {
                    Kind::Single(_) | Kind::Group(..) => {
                        let fragments = test_ledger.utxodb.find_fragments(&out);

                        if fragments.len() == 0 {
                            panic!("trying to do a inputs_to_outputs with unknown single utxo")
                        }

                        // Take the first one ..
                        let (fragment_id, idx) = fragments[0];

                        Input::from_utxo(UtxoPointer {
                            transaction_id: fragment_id,
                            output_index: idx,
                            value: out.value,
                        })
                    }
                    Kind::Account(pk) => {
                        let aid =
                            UnspecifiedAccountIdentifier::from_single_account(pk.clone().into());
                        Input::from_account(aid, out.value)
                    }
                    Kind::Multisig(pk) => {
                        let aid =
                            UnspecifiedAccountIdentifier::from_multi_account(pk.clone().into());
                        Input::from_account(aid, out.value)
                    }
                }
            })
            .collect();

        let tx_builder = TxBuilder::new()
            .set_payload(&NoExtra)
            .set_ios(&inputs, &destination);

        let auth_data_hash = tx_builder.get_auth_data_for_witness().hash();
        let mut witnesses = Vec::with_capacity(inputs.len());

        for (inp, _) in inputs.iter().zip(sources.iter()) {
            let witness = {
                match inp.to_enum() {
                    InputEnum::AccountInput(account_id, _) => {
                        let aid = account_id.to_single_account().unwrap();
                        let sk = kdb.find_ed25519_secret_key(&aid.into()).unwrap();
                        // FIXME - TODO need accountdb to get the latest state of account counter
                        let counter = SpendingCounter::zero();
                        Witness::new_account(&self.block0_hash, &auth_data_hash, &counter, sk)
                    }
                    InputEnum::UtxoInput(utxopointer) => {
                        match test_ledger.utxodb.get(&(utxopointer.transaction_id, utxopointer.output_index)) {
                            None => {
                                panic!("cannot find utxo input")
                            },
                            Some(output) => {
                                let sk = kdb.find_by_address(&output.address).unwrap();
                                Witness::new_utxo(&self.block0_hash, &auth_data_hash, sk)
                            }
                        }
                    }
                }
            };
            witnesses.push(witness)
        }

        let tx = tx_builder
            .set_witnesses(&witnesses)
            .set_payload_auth(&());
        TestTx { tx }
    }
}
