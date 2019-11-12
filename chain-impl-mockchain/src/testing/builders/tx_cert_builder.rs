use crate::{
    account::SpendingCounter,
    fragment::{FragmentId, Fragment},
    header::HeaderId,
    testing::{KeysDb, ledger::TestLedger},
    transaction::{AccountIdentifier, Transaction, TxBuilder, Input, InputEnum, Output, Witness, UtxoPointer},
    value::Value,
};
use chain_addr::{Address, Kind};

pub struct TestTxCertBuilder {
    block0_hash: HeaderId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestCertTx {
    tx: Transaction<Extra>,
}

impl TestCertTx {
    pub fn get_fragment_id(&self) -> FragmentId {
        self.clone().get_fragment().hash()
    }

    pub fn get_fragment(self) -> Fragment {
        Fragment::Transaction(self.tx)
    }

    pub fn get_tx(self) -> Transaction<Extra> {
        self.tx
    }
}

impl TestTxCertBuilder {
    pub fn new(block0_hash: &HeaderId) -> Self {
        Self {
            block0_hash: block0_hash.clone(),
        }
    }

    pub fn make_transaction_with_payload(self, testledger: &mut TestLedger, signers: &[Wallet], certificate: &Certificate) -> TestCertTx {
        let funder = &signers[0];
        let inputs = vec![funder.make_input()];
        let tx_builder = TxBuilder::new()
            .set_payload(&certificate.into())
            .set_ios(&inputs, &[]);

        let auth_data_hash = tx_builder.get_auth_data_for_witness().hash();
        let witness = witness_builder::make_witness(
            &testledger.block0_hash,
            &funder.address,
            auth_data_hash,
        );
        let tx = tx_builder.set_witnesses(&vec![witness]);
        let auth = set_auth(&funder.private_key(),certificate,&tx);

        let payload_auth = match certificate {
            certificate::OwnerStakeDelegation => (),
            _ => sign_certificate(certificate,keys)
        }
        tx.set_payload_auth(&payload_auth);
        TestCertTx { tx }
    }
}