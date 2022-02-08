use crate::jcli_lib::transaction::{common, Error};
use jormungandr_lib::interfaces::EvmTransaction;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub struct AddEvmTransaction {
    #[structopt(flatten)]
    pub common: common::CommonTransaction,

    /// hex encoded evm transaction
    pub evm_transaction: EvmTransaction,
}

impl AddEvmTransaction {
    pub fn exec(self) -> Result<(), Error> {
        let mut transaction = self.common.load()?;
        transaction.set_evm_transaction(self.evm_transaction)?;
        self.common.store(&transaction)
    }
}

#[cfg(all(test, feature = "evm"))]
mod test {
    use super::*;
    use chain_impl_mockchain::evm;
    use quickcheck::Arbitrary;

    impl Arbitrary for AddEvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                common: common::CommonTransaction { staging_file: None },
                evm_transaction: EvmTransaction(evm::EvmTransaction::arbitrary(g)),
            }
        }
    }

    impl Clone for AddEvmTransaction {
        fn clone(&self) -> Self {
            Self {
                common: common::CommonTransaction { staging_file: None },
                evm_transaction: self.evm_transaction.clone(),
            }
        }
    }

    impl PartialEq for AddEvmTransaction {
        fn eq(&self, other: &AddEvmTransaction) -> bool {
            self.evm_transaction == other.evm_transaction
        }
    }

    quickcheck! {
        fn evm_transaction_encode(add_evm_tx: AddEvmTransaction) -> bool {
            let hex_tx = format!("{}", add_evm_tx.evm_transaction);
            AddEvmTransaction::from_iter(&["", &hex_tx]) == add_evm_tx
        }
    }
}
