use crate::{
    account::Identifier,
    accounting::account::account_state::AccountState,
    ledger::ledger::Ledger,
    stake::{Stake, StakeDistribution},
    testing::data::AddressData,
    utxo,
    value::Value,
};
use chain_addr::Address;
pub struct LedgerStateVerifier {
    ledger: Ledger,
}

impl LedgerStateVerifier {
    pub fn new(ledger: Ledger) -> Self {
        LedgerStateVerifier { ledger: ledger }
    }

    pub fn utxo_contains(&self, entry: &utxo::Entry<Address>) -> &Self {
        assert_eq!(
            self.ledger.utxos.iter().find(|x| *x == entry.clone()),
            Some(entry.clone())
        );
        self
    }

    pub fn and(&self) -> &Self {
        self
    }

    pub fn accounts_contains(
        &self,
        id: Identifier,
        expected_account_state: AccountState<()>,
    ) -> &Self {
        let account_state = self.ledger.accounts.get_state(&id).unwrap();
        assert_eq!(account_state.clone(), expected_account_state);
        self
    }

    pub fn utxos_count_is(&self, count: usize) -> &Self {
        assert_eq!(
            self.ledger.utxos.iter().count(),
            count,
            "Utxo count should be equal to {:?}",
            count
        );
        self
    }

    pub fn accounts_count_is(&self, count: usize) -> &Self {
        assert_eq!(
            self.ledger.accounts.iter().count(),
            count,
            "Utxo count should be equal to {:?}",
            count
        );
        self
    }

    pub fn multisigs_count_is_zero(&self) -> &Self {
        assert_eq!(self.ledger.multisig.iter_accounts().count(), 0);
        assert_eq!(self.ledger.multisig.iter_declarations().count(), 0);
        self
    }

    pub fn distribution(&self) -> DistributionVerifier {
        DistributionVerifier::new(self.ledger.get_stake_distribution())
    }

    pub fn total_value_is(&self, value: Value) -> &Self {
        let account_total = self.ledger.accounts.get_total_value().unwrap();
        let multisig_total = self.ledger.multisig.get_total_value().unwrap();
        let utxos_total =
            Value::sum(self.ledger.utxos.iter().map(|entry| entry.output.value)).unwrap();
        let totals = vec![account_total, multisig_total, utxos_total];
        let actual_value =
            Value::sum(totals.iter().cloned()).expect("cannot sum up ledger total value");
        assert_eq!(
            value, actual_value,
            "Expected value {:?} vs {:?} of actual",
            value, actual_value
        );
        self
    }

    // Does not cover situation in which we have two identical utxos
    pub fn address_has_expected_balance(&self, address: AddressData, value: Value) -> &Self {
        match self.ledger.accounts.exists(&address.to_id()) {
            true => self.account_has_expected_balance(address, value),
            false => self.utxo_has_expected_balance(address, value),
        }
    }

    pub fn account_has_expected_balance(&self, address: AddressData, value: Value) -> &Self {
        let account_state = self
            .ledger
            .accounts
            .get_state(&address.to_id())
            .expect("account does not exists while it should");
        assert_eq!(account_state.value(), value);
        self
    }

    pub fn utxo_has_expected_balance(&self, address_data: AddressData, value: Value) -> &Self {
        let utxo = self
            .ledger
            .utxos
            .iter()
            .find(|x| *x.output == address_data.make_output(&value));
        match value == Value::zero() {
            true => {
                assert!(utxo.is_none());
                return self;
            }
            false => {
                let utxo = utxo.unwrap();
                assert_eq!(utxo.output.value, value);
                return self;
            }
        }
    }
}

pub struct DistributionVerifier {
    stake_distribution: StakeDistribution,
}

impl DistributionVerifier {
    pub fn new(stake_distribution: StakeDistribution) -> Self {
        DistributionVerifier {
            stake_distribution: stake_distribution,
        }
    }

    pub fn dangling_is(&self, dangling: Stake) -> &Self {
        assert_eq!(
            dangling, self.stake_distribution.dangling,
            "wrong unassigned distribution value"
        );
        self
    }

    pub fn and(&self) -> &Self {
        self
    }

    pub fn unassigned_is(&self, unassigned: Stake) -> &Self {
        assert_eq!(
            unassigned, self.stake_distribution.unassigned,
            "wrong unassigned distribution value"
        );
        self
    }

    pub fn pools_total_stake_is(&self, pools_total: Stake) -> &Self {
        assert_eq!(
            pools_total,
            self.stake_distribution.total_stake(),
            "wrong total stake"
        );
        self
    }
}
