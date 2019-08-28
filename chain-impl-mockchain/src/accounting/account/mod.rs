//! Generic account like accounting
//!
//! This is effectively an immutable clonable-HAMT of bank style account,
//! which contains a non negative value representing your balance with the
//! identifier of this account as key.

use crate::certificate::PoolId;
use crate::value::*;
use imhamt::{Hamt, InsertError, UpdateError};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug};
use std::hash::Hash;
pub mod account_state;

pub use account_state::*;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub LedgerError
        NonExistent = "Account does not exist",
        AlreadyExists = "Account already exists",
        NeedTotalWithdrawal = "Operation counter reached its maximum and next operation must be full withdrawal",
        NonZero = "Removed account is not empty",
        ValueError{ source: ValueError } = "Value calculation failed",
}

impl From<UpdateError<LedgerError>> for LedgerError {
    fn from(e: UpdateError<LedgerError>) -> Self {
        match e {
            UpdateError::KeyNotFound => LedgerError::NonExistent,
            UpdateError::ValueCallbackError(v) => v,
        }
    }
}

impl From<InsertError> for LedgerError {
    fn from(e: InsertError) -> Self {
        match e {
            InsertError::EntryExists => LedgerError::AlreadyExists,
        }
    }
}

/// The public ledger of all accounts associated with their current state
#[derive(Clone, PartialEq, Eq)]
pub struct Ledger<ID: Hash + Eq, Extra>(Hamt<DefaultHasher, ID, AccountState<Extra>>);

impl<ID: Clone + Eq + Hash, Extra: Clone> Ledger<ID, Extra> {
    /// Create a new empty account ledger
    pub fn new() -> Self {
        Ledger(Hamt::new())
    }

    /// Add a new account into this ledger.
    ///
    /// If the identifier is already present, error out.
    pub fn add_account(
        &self,
        identifier: &ID,
        initial_value: Value,
        extra: Extra,
    ) -> Result<Self, LedgerError> {
        self.0
            .insert(identifier.clone(), AccountState::new(initial_value, extra))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Set the delegation of an account in this ledger
    pub fn set_delegation(
        &self,
        identifier: &ID,
        delegation: Option<PoolId>,
    ) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| Ok(Some(st.set_delegation(delegation))))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// check if an account already exist
    #[inline]
    pub fn exists(&self, identifier: &ID) -> bool {
        self.0.contains_key(identifier)
    }

    /// Get account state
    ///
    /// If the identifier does not match any account, error out
    pub fn get_state(&self, account: &ID) -> Result<&AccountState<Extra>, LedgerError> {
        self.0.lookup(account).ok_or(LedgerError::NonExistent)
    }

    /// Remove an account from this ledger
    ///
    /// If the account still have value > 0, then error
    pub fn remove_account(&self, identifier: &ID) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| {
                if st.value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NonZero)
                }
            })
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Add value to an existing account.
    ///
    /// If the account doesn't exist, error out.
    pub fn add_value(&self, identifier: &ID, value: Value) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| st.add(value).map(Some))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Subtract value to an existing account.
    ///
    /// If the account doesn't exist, or that the value would become negative, errors out.
    pub fn remove_value(
        &self,
        identifier: &ID,
        value: Value,
    ) -> Result<(Self, SpendingCounter), LedgerError> {
        // ideally we don't need 2 calls to do this
        let counter = self
            .0
            .lookup(identifier)
            .map_or(Err(LedgerError::NonExistent), |st| Ok(st.counter))?;
        self.0
            .update(identifier, |st| st.sub(value))
            .map(|ledger| (Ledger(ledger), counter))
            .map_err(|e| e.into())
    }

    pub fn get_total_value(&self) -> Result<Value, ValueError> {
        let values = self
            .0
            .iter()
            .map(|(_, account_state)| account_state.get_value());
        Value::sum(values)
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, ID, Extra> {
        Iter(self.0.iter())
    }
}

impl<ID: Clone + Eq + Hash + Debug, Extra: Clone + Debug> Debug for Ledger<ID, Extra> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.0
                .iter()
                .map(|(id, account)| (id.clone(), account.clone()))
                .collect::<Vec<(ID, AccountState<Extra>)>>()
        )
    }
}

impl<ID: Clone + Eq + Hash, Extra: Clone> std::iter::FromIterator<(ID, AccountState<Extra>)>
    for Ledger<ID, Extra>
{
    fn from_iter<I: IntoIterator<Item = (ID, AccountState<Extra>)>>(iter: I) -> Self {
        Ledger(Hamt::from_iter(iter))
    }
}

#[cfg(test)]
pub mod tests {

    use crate::{
        account::{Identifier, Ledger},
        accounting::account::account_state::{AccountState, SpendingCounter},
        stake::{StakePoolId, StakePoolInfo},
        testing::{arbitrary::utils as arbitrary_utils, arbitrary::AverageValue},
        value::Value,
    };

    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::collections::HashSet;
    use std::iter;
    #[quickcheck]
    pub fn ledger_total_value_is_correct_after_remove_value(
        id: Identifier,
        account_state: AccountState<()>,
        value_to_remove: Value,
    ) -> TestResult {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(&id, account_state.get_value(), ())
            .unwrap();
        let result = ledger.remove_value(&id, value_to_remove);
        let expected_result = account_state.get_value() - value_to_remove;
        match (result, expected_result) {
            (Err(_), Err(_)) => verify_total_value(ledger, account_state.get_value()),
            (Ok(_), Err(_)) => TestResult::failed(),
            (Err(_), Ok(_)) => TestResult::failed(),
            (Ok((ledger, _)), Ok(value)) => verify_total_value(ledger, value),
        }
    }

    fn verify_total_value(ledger: Ledger, value: Value) -> TestResult {
        match ledger.get_total_value().unwrap() == value {
            true => TestResult::passed(),
            false => TestResult::error(format!(
                "Wrong total value got {:?}, while expecting {:?}",
                ledger.get_total_value(),
                value
            )),
        }
    }

    #[quickcheck]
    pub fn ledger_removes_account_only_if_zeroed(
        id: Identifier,
        account_state: AccountState<()>,
    ) -> TestResult {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(&id, account_state.get_value(), ())
            .unwrap();
        let result = ledger.remove_account(&id);
        let expected_result = account_state.get_value() == Value::zero();
        match (result, expected_result) {
            (Err(_), false) => verify_account_exists(&ledger, &id),
            (Ok(_), false) => TestResult::failed(),
            (Err(_), true) => TestResult::failed(),
            (Ok(ledger), true) => verify_account_does_not_exist(&ledger, &id),
        }
    }

    fn verify_account_exists(ledger: &Ledger, id: &Identifier) -> TestResult {
        match ledger.exists(&id) {
            true => TestResult::passed(),
            false => TestResult::error(format!("Account ({:?}) not exists , while it should", &id)),
        }
    }

    fn verify_account_does_not_exist(ledger: &Ledger, id: &Identifier) -> TestResult {
        match ledger.exists(&id) {
            true => TestResult::error(format!("Account ({:?}) exists , while it should not", &id)),
            false => TestResult::passed(),
        }
    }
    impl Arbitrary for Ledger {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let account_size = std::cmp::max(usize::arbitrary(gen), 1);
            let stake_pool_size =
                std::cmp::min(account_size, usize::arbitrary(gen) % account_size + 1);
            let arbitrary_accounts_ids = iter::from_fn(|| Some(Identifier::arbitrary(gen)))
                .take(account_size)
                .collect::<HashSet<Identifier>>();

            let arbitrary_stake_pools = iter::from_fn(|| Some(StakePoolInfo::arbitrary(gen)))
                .take(stake_pool_size)
                .collect::<Vec<StakePoolInfo>>();

            let mut ledger = Ledger::new();

            // Add all arbitrary accounts
            for account_id in arbitrary_accounts_ids.iter().cloned() {
                ledger = ledger
                    .add_account(&account_id, AverageValue::arbitrary(gen).into(), ())
                    .unwrap();
            }

            // Choose random subset of arbitraty accounts and delegate stake to random stake pools
            for account_id in
                arbitrary_utils::choose_random_set_subset(&arbitrary_accounts_ids, gen)
            {
                let random_stake_pool =
                    arbitrary_utils::choose_random_item(&arbitrary_stake_pools, gen);
                ledger = ledger
                    .set_delegation(&account_id, Some(random_stake_pool.to_id()))
                    .unwrap();
            }
            ledger
        }
    }

    #[quickcheck]
    pub fn account_ledger_test(
        mut ledger: Ledger,
        account_id: Identifier,
        value: Value,
        stake_pool_id: StakePoolId,
    ) -> TestResult {
        if value == Value::zero() || ledger.exists(&account_id) {
            return TestResult::discard();
        }

        let initial_total_value = ledger.get_total_value().unwrap();

        // add new account
        ledger = match ledger.add_account(&account_id, value, ()) {
            Ok(ledger) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Add account with id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // add account again should throw an error
        if ledger.add_account(&account_id, value, ()).is_ok() {
            return TestResult::error(format!(
                "Account with id {} again should should",
                account_id
            ));
        }

        // verify account exists
        if !ledger.exists(&account_id) {
            return TestResult::error(format!("Account with id {} should exist", account_id));
        }

        // verify account is listed amongst other
        if !ledger.iter().any(|(x, _)| *x == account_id) {
            return TestResult::error(format!(
                "Account with id {} should be lister amongst other",
                account_id
            ));
        }

        // verify total value was increased
        let test_result = test_total_value(
            (initial_total_value + value).unwrap(),
            ledger.get_total_value().unwrap(),
        );
        if test_result.is_error() {
            return test_result;
        }

        // set delegation to stake pool
        ledger = match ledger.set_delegation(&account_id, Some(stake_pool_id.clone())) {
            Ok(ledger) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Error for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // verify total value is still the same
        let test_result = test_total_value(
            (initial_total_value + value).unwrap(),
            ledger.get_total_value().unwrap(),
        );
        if test_result.is_error() {
            return test_result;
        }

        // add value to account
        ledger = match ledger.add_value(&account_id, value.clone()) {
            Ok(ledger) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Error for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // verify total value was increased
        let test_result = test_total_value(
            (initial_total_value + (value + value).unwrap()).unwrap(),
            ledger.get_total_value().unwrap(),
        );
        if test_result.is_error() {
            return test_result;
        }

        //verify account state
        match ledger.get_state(&account_id) {
            Ok(account_state) => {
                let expected_account_state = AccountState {
                    counter: SpendingCounter::zero(),
                    delegation: Some(stake_pool_id),
                    value: Value(value.0 * 2),
                    extra: (),
                };

                match *account_state == expected_account_state {
                    true => (),
                    false => {
                        return TestResult::error(format!(
                            "Account state is incorrect expected {:?} but got {:?}",
                            expected_account_state, account_state
                        ))
                    }
                }
            }
            Err(err) => {
                return TestResult::error(format!(
                    "Get state for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        }

        // remove value from account
        ledger = match ledger.remove_value(&account_id, value.clone()) {
            Ok((ledger, _spending_counter)) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Error for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // verify total value was decreased
        let test_result = test_total_value(
            (initial_total_value + value).unwrap(),
            ledger.get_total_value().unwrap(),
        );
        if test_result.is_error() {
            return test_result;
        }

        // verify remove account fails beause account still got some founds
        if ledger.remove_account(&account_id).is_ok() {
            return TestResult::error(format!(
                "Remove account should be unsuccesfull... account for id {} still got funds",
                account_id
            ));
        }

        // removes all funds from account
        ledger = match ledger.remove_value(&account_id, value.clone()) {
            Ok((ledger, _spending_counter)) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Error for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // removes account
        ledger = match ledger.remove_account(&account_id) {
            Ok(ledger) => ledger,
            Err(err) => {
                return TestResult::error(format!(
                    "Error for id {} should be successful: {:?}",
                    account_id, err
                ))
            }
        };

        // verify accounts does not exists
        if ledger.exists(&account_id) {
            return TestResult::error(format!("Account with id {} should not exist", account_id));
        }

        // verify account is not listed amongst other accounts
        if ledger.iter().any(|(x, _)| *x == account_id) {
            return TestResult::error(format!(
                "Account with id {} should not be listed amongst accounts",
                account_id
            ));
        }

        // verify total funds is equal to initial total_value
        let test_result = test_total_value(initial_total_value, ledger.get_total_value().unwrap());
        if test_result.is_error() {
            return test_result;
        }

        match ledger.get_state(&account_id) {
            Ok(_) => TestResult::error("Account state is should be none"),
            Err(_) => TestResult::passed(),
        }
    }

    fn test_total_value(expected: Value, actual: Value) -> TestResult {
        match actual == expected {
            true => TestResult::passed(),
            false => TestResult::error(format!(
                "Wrong total value expected {} but got {}",
                expected, actual
            )),
        }
    }
}
