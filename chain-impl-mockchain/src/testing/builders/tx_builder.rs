use crate::{
    account::SpendingCounter,
    fragment::{Fragment, FragmentId},
    header::HeaderId,
    testing::{ledger::TestLedger, KeysDb},
    transaction::{
        Input, InputEnum, NoExtra, Output, Transaction, TxBuilder, UnspecifiedAccountIdentifier,
        UtxoPointer, Witness,
    },
    value::Value,
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

    pub fn move_from_faucet(self, testledger: &mut TestLedger, destination: &Address, value: Value) -> TestTx {
        let faucet = testledger.faucet.as_mut().expect("test ledger with no faucet configured");
        assert_eq!(faucet.block0_hash, self.block0_hash);
        let inputs = vec![faucet.get_input_of(value)];
        let outputs = vec![Output { address: destination.clone(), value: value }];
        let tx_builder = TxBuilder::new()
            .set_payload(&NoExtra)
            .set_ios(&inputs, &outputs);

        let witness = faucet.make_witness(tx_builder.get_auth_data_for_witness());
        let witnesses = vec![witness];

        let tx = tx_builder
            .set_witnesses(&witnesses)
            .set_payload_auth(&());
        TestTx { tx }
    }

    pub fn move_to_outputs_from_faucet(self, testledger: &mut TestLedger, destination: &[Output<Address>]) -> TestTx {
        let faucet = testledger.faucet.as_mut().expect("test ledger with no faucet configured");
        assert_eq!(faucet.block0_hash, self.block0_hash);
        let input_val = Value::sum(destination.iter().map(|o| o.value)).unwrap();
        let inputs = vec![faucet.get_input_of(input_val)];
        let tx_builder = TxBuilder::new()
            .set_payload(&NoExtra)
            .set_ios(&inputs, &destination);

        let witness = faucet.make_witness(tx_builder.get_auth_data_for_witness());
        let witnesses = vec![witness];

        let tx = tx_builder
            .set_witnesses(&witnesses)
            .set_payload_auth(&());
        TestTx { tx }
    }

    pub fn inputs_to_outputs(
        self,
        kdb: &KeysDb,
        testledger: &mut TestLedger,
        sources: &[Output<Address>],
        destination: &[Output<Address>],
    ) -> TestTx {
        let inputs: Vec<_> = sources
            .iter()
            .map(|out| {
                match out.address.kind() {
                    Kind::Single(_) | Kind::Group(..) => {
                        let fragments = testledger.utxodb.find_fragments(&out);

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
                        match testledger.utxodb.get(&(utxopointer.transaction_id, utxopointer.output_index)) {
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
