use crate::{
    fragment::{FragmentId, Fragment},
    header::HeaderId,
    testing::ledger::Faucet,
    transaction::{Transaction, NoExtra, TxBuilder, Output},
    value::Value,
};
use chain_addr::Address;

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

    pub fn move_from_faucet(self, faucet: &mut Faucet, destination: &Address, value: Value) -> TestTx {
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
}
