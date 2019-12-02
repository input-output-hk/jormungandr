use crate::{
    account::Identifier,
    accounting::account::account_state::AccountState,
    ledger::{ledger::Ledger, Pots},
    stake::{Stake, StakeDistribution},
    testing::data::{AddressData,StakePool},
    utxo,
    value::Value,
    stake::PoolsState,
    certificate::PoolId,
};
use chain_addr::Address;
use std::fmt;

#[derive(Clone)]
pub struct Info{
    info: Option<String>,
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.info {
            Some(info) => write!(f, "{}", info),
            None => write!(f, ""),
        }
    }
}

impl Info {
    pub fn from_str<S: Into<String>>(info: S) -> Self {
        Info {
            info: Some(info.into())
        }
    }

    pub fn empty() -> Self {
        Info {
            info: None
        }
    }
}

pub struct LedgerStateVerifier {
    ledger: Ledger,
    info: Info,
}

impl LedgerStateVerifier {
    pub fn new(ledger: Ledger) -> Self {
        LedgerStateVerifier { ledger: ledger, info: Info::empty() }
    }

    pub fn info<S: Into<String>>(&mut self, info: S) -> &mut Self {
        self.info = Info::from_str(info);
        self
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
            "Utxo count should be equal to {:?} {}",
            count,self.info
        );
        self
    }

    pub fn accounts_count_is(&self, count: usize) -> &Self {
        assert_eq!(
            self.ledger.accounts.iter().count(),
            count,
            "Utxo count should be equal to {:?} {}",
            count,self.info
        );
        self
    }

    pub fn multisigs_count_is_zero(&self) -> &Self {
        assert_eq!(self.ledger.multisig.iter_accounts().count(), 0);
        assert_eq!(self.ledger.multisig.iter_declarations().count(), 0);
        self
    }

    pub fn distribution(&self) -> DistributionVerifier {
        DistributionVerifier::new(self.ledger.get_stake_distribution(),self.info.clone())
    }

    pub fn stake_pools(&self) -> StakePoolsVerifier {
        StakePoolsVerifier::new(self.ledger.delegation.clone(),self.info.clone())
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
            "Expected value {:?} vs {:?} of actual {}",
            value, actual_value, self.info
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

    pub fn pots(&self) -> PotsVerifier {
        PotsVerifier::new(self.ledger.pots.clone(),self.info.clone())
    }
}

pub struct PotsVerifier {
    pots: Pots,
    info: Info
}

impl PotsVerifier {
    pub fn new(pots: Pots, info: Info) -> Self {
        PotsVerifier { pots, info }
    }

    pub fn has_fee_equal_to(&self, value: &Value) {
        assert_eq!(self.pots.fees, *value, "incorrect pot fee value {}", self.info);
    }
}

pub struct StakePoolsVerifier {
    delegation: PoolsState,
    info: Info
}

impl StakePoolsVerifier {
    pub fn new(delegation: PoolsState, info: Info) -> Self {
        StakePoolsVerifier {
            delegation, info
        }
    }

    pub fn is_retired(&self, stake_pool: &StakePool) {
        assert!(!self.delegation.stake_pool_exists(&stake_pool.id()),"stake pool {} should be retired ({}), but it is not", stake_pool.alias(),self.info);
    }

    pub fn is_not_retired(&self, stake_pool: &StakePool) {
        assert!(self.delegation.stake_pool_exists(&stake_pool.id()),"stake pool {} should be active ({}), but it is retired", stake_pool.alias(),self.info);
    }
}

pub struct DistributionVerifier {
    stake_distribution: StakeDistribution,
    info: Info
}

impl DistributionVerifier {
    pub fn new(stake_distribution: StakeDistribution, info: Info) -> Self {
        DistributionVerifier {
            stake_distribution,info
        }
    }

    pub fn dangling_is(&self, dangling: Stake) -> &Self {
        assert_eq!(
            dangling, self.stake_distribution.dangling,
            "wrong unassigned distribution value {}", self.info
        );
        self
    }

    pub fn and(&self) -> &Self {
        self
    }

    pub fn unassigned_is(&self, unassigned: Stake) -> &Self {
        assert_eq!(
            unassigned, self.stake_distribution.unassigned,
            "wrong unassigned distribution value {}", self.info
        );
        self
    }

    pub fn pools_total_stake_is(&self, pools_total: Stake) -> &Self {
        assert_eq!(
            pools_total,
            self.stake_distribution.total_stake(),
            "wrong total stake {}",self.info
        );
        self
    }

    pub fn pools_distribution_is(&self, expected_distribution: Vec<(PoolId,Value)>) -> &Self {
        for (pool_id, value) in expected_distribution {
            let stake = self.stake_distribution.get_stake_for(&pool_id);
            assert!(stake.is_some(),"pool with id {:?} does not exist {}",pool_id, self.info);
            let stake = stake.unwrap();
            assert_eq!(stake,Stake::from_value(value),"wrong total stake for pool with id {} {}", pool_id,self.info);
        }
        self 
    }
}
