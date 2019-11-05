use crate::certificate::PoolId;
use crate::value::*;
use imhamt::HamtIter;

use super::LedgerError;

/// Set the choice of delegation:
///
/// * No delegation
/// * Full delegation of this account to a specific pool
/// * Ratio of stake to multiple pools
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DelegationType {
    NonDelegated,
    Full(PoolId),
    Ratio(DelegationRatio),
}

/// Delegation Ratio type express a number of parts
/// and a list of pools and their
///
/// E.g. parts: 7, pools: [(A, 3), (B,1), (C,4)] means that
/// A is associated with 3/7 of the stake, B has 1/7 of stake and C
/// has 4/7 of the stake.
///
/// It's invalid to have less than 2 elements in the array,
/// and by extension parts need to be equal to the sum of individual
/// pools parts.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DelegationRatio {
    parts: u8,
    pools: Box<[(PoolId, u8)]>,
}

impl DelegationRatio {
    pub fn is_valid(&self) -> bool {
        // map to u32 before summing to make sure we don't overflow
        let total: u32 = self.pools.iter().map(|x| x.1 as u32).sum();
        let has_no_zero = self.pools.iter().find(|x| x.1 == 0).is_none();
        has_no_zero && self.parts > 1 && self.pools.len() > 1 && total == (self.parts as u32)
    }

    pub fn new(parts: u8, pools: Vec<(PoolId, u8)>) -> Option<DelegationRatio> {
        let total: u32 = pools.iter().map(|x| x.1 as u32).sum();
        let has_no_zero = pools.iter().find(|x| x.1 == 0).is_none();
        if has_no_zero && parts > 1 && pools.len() > 1 && total == (parts as u32) {
            Some(Self {
                parts,
                pools: pools.into(),
            })
        } else {
            None
        }
    }

    pub fn parts(&self) -> u8 {
        self.parts
    }

    pub fn pools(&self) -> &[(PoolId, u8)] {
        &self.pools
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AccountState<Extra> {
    pub counter: SpendingCounter,
    pub delegation: DelegationType,
    pub value: Value,
    pub extra: Extra,
}

impl<Extra> AccountState<Extra> {
    /// Create a new account state with a specific start value
    pub fn new(v: Value, e: Extra) -> Self {
        Self {
            counter: SpendingCounter(0),
            delegation: DelegationType::NonDelegated,
            value: v,
            extra: e,
        }
    }

    /// Get referencet to delegation setting
    pub fn delegation(&self) -> &DelegationType {
        &self.delegation
    }

    pub fn value(&self) -> Value {
        self.value
    }

    // deprecated use value()
    pub fn get_value(&self) -> Value {
        self.value
    }

    pub fn get_counter(&self) -> u32 {
        self.counter.into()
    }
}

impl<Extra: Clone> AccountState<Extra> {
    /// Add a value to an account state
    ///
    /// Only error if value is overflowing
    pub fn add(&self, v: Value) -> Result<Self, LedgerError> {
        let new_value = (self.value + v)?;
        let mut st = self.clone();
        st.value = new_value;
        Ok(st)
    }

    /// Subtract a value from an account state, and return the new state.
    ///
    /// Note that this *also* increment the counter, as this function would be usually call
    /// for spending.
    ///
    /// If the counter is also reaching the extremely rare of max, we only authorise
    /// a total withdrawal of fund otherwise the fund will be stuck forever in limbo.
    pub fn sub(&self, v: Value) -> Result<Option<Self>, LedgerError> {
        let new_value = (self.value - v)?;
        match self.counter.increment() {
            None => {
                if new_value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NeedTotalWithdrawal)
                }
            }
            Some(new_counter) => Ok(Some(Self {
                counter: new_counter,
                delegation: self.delegation.clone(),
                value: new_value,
                extra: self.extra.clone(),
            })),
        }
    }

    /// Set delegation
    pub fn set_delegation(&self, delegation: DelegationType) -> Self {
        let mut st = self.clone();
        st.delegation = delegation;
        st
    }
}

/// Spending counter associated to an account.
///
/// every time the owner is spending from an account,
/// the counter is incremented. A matching counter
/// needs to be used in the spending phase to make
/// sure we have non-replayability of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendingCounter(u32);

impl SpendingCounter {
    pub fn zero() -> Self {
        SpendingCounter(0)
    }

    pub fn increment(&self) -> Option<Self> {
        self.0.checked_add(1).map(SpendingCounter)
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

impl From<u32> for SpendingCounter {
    fn from(v: u32) -> Self {
        SpendingCounter(v)
    }
}

impl From<SpendingCounter> for u32 {
    fn from(v: SpendingCounter) -> u32 {
        v.0
    }
}

pub struct Iter<'a, ID, Extra>(pub HamtIter<'a, ID, AccountState<Extra>>);

impl<'a, ID, Extra> Iterator for Iter<'a, ID, Extra> {
    type Item = (&'a ID, &'a AccountState<Extra>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountState, DelegationType, SpendingCounter};
    use crate::{certificate::PoolId, value::Value};
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::iter;

    impl Arbitrary for SpendingCounter {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            SpendingCounter(Arbitrary::arbitrary(gen))
        }
    }

    impl Arbitrary for AccountState<()> {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            AccountState {
                counter: Arbitrary::arbitrary(gen),
                delegation: DelegationType::Full(Arbitrary::arbitrary(gen)),
                value: Arbitrary::arbitrary(gen),
                extra: (),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum ArbitraryAccountStateOp {
        Add(Value),
        Sub(Value),
        Delegate(PoolId),
        RemoveDelegation,
    }

    impl Arbitrary for ArbitraryAccountStateOp {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let option = u8::arbitrary(gen) % 4;
            match option {
                0 => ArbitraryAccountStateOp::Add(Value::arbitrary(gen)),
                1 => ArbitraryAccountStateOp::Sub(Value::arbitrary(gen)),
                2 => ArbitraryAccountStateOp::Delegate(PoolId::arbitrary(gen)),
                3 => ArbitraryAccountStateOp::RemoveDelegation,
                _ => panic!("not implemented"),
            }
        }
    }

    #[quickcheck]
    pub fn account_sub_is_consistent(
        init_value: Value,
        sub_value: Value,
        counter: u32,
    ) -> TestResult {
        let mut account_state = AccountState::new(init_value, ());
        account_state.counter = counter.into();
        TestResult::from_bool(
            should_sub_fail(account_state.clone(), sub_value)
                == account_state.sub(sub_value).is_err(),
        )
    }

    #[derive(Clone, Debug)]
    pub struct ArbitraryOperationChain(pub Vec<ArbitraryAccountStateOp>);

    impl Arbitrary for ArbitraryOperationChain {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let len = usize::arbitrary(gen);
            let ops: Vec<ArbitraryAccountStateOp> =
                iter::from_fn(|| Some(ArbitraryAccountStateOp::arbitrary(gen)))
                    .take(len)
                    .collect();
            ArbitraryOperationChain(ops)
        }
    }

    impl ArbitraryOperationChain {
        pub fn get_account_state_after_n_ops(
            &self,
            initial_account_state: AccountState<()>,
            counter: usize,
            subs: u32,
        ) -> AccountState<()> {
            let n_ops: Vec<ArbitraryAccountStateOp> =
                self.0.iter().cloned().take(counter).collect();
            self.calculate_account_state(initial_account_state.clone(), n_ops.iter(), subs)
        }

        pub fn get_account_state(
            &self,
            initial_account_state: AccountState<()>,
            subs: u32,
        ) -> AccountState<()> {
            self.calculate_account_state(initial_account_state.clone(), self.0.iter(), subs)
        }

        fn calculate_account_state(
            &self,
            initial_account_state: AccountState<()>,
            operations: std::slice::Iter<ArbitraryAccountStateOp>,
            subs: u32,
        ) -> AccountState<()> {
            let result_spending_counter = initial_account_state.counter.0 + subs;
            let mut delegation = initial_account_state.delegation().clone();
            let mut result_value = initial_account_state.get_value();

            for operation in operations {
                match operation {
                    ArbitraryAccountStateOp::Add(value) => {
                        result_value = match result_value + *value {
                            Ok(new_value) => new_value,
                            Err(_) => result_value,
                        }
                    }
                    ArbitraryAccountStateOp::Sub(value) => {
                        result_value = match result_value - *value {
                            Ok(new_value) => new_value,
                            Err(_) => result_value,
                        }
                    }
                    ArbitraryAccountStateOp::Delegate(new_delegation) => {
                        delegation = DelegationType::Full(new_delegation.clone());
                    }
                    ArbitraryAccountStateOp::RemoveDelegation => {
                        delegation = DelegationType::NonDelegated;
                    }
                }
            }
            AccountState {
                counter: SpendingCounter(result_spending_counter),
                delegation: delegation,
                value: result_value,
                extra: (),
            }
        }
    }

    impl IntoIterator for ArbitraryOperationChain {
        type Item = ArbitraryAccountStateOp;
        type IntoIter = ::std::vec::IntoIter<Self::Item>;

        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    #[quickcheck]
    pub fn account_state_is_consistent(
        mut account_state: AccountState<()>,
        operations: ArbitraryOperationChain,
    ) -> TestResult {
        // for reporting which operation failed
        let mut counter = 0;

        // to count spending counter
        let mut successful_subs = 0;

        let initial_account_state = account_state.clone();
        for operation in operations.clone() {
            counter = counter + 1;
            account_state = match operation {
                ArbitraryAccountStateOp::Add(value) => {
                    let should_fail = should_add_fail(account_state.clone(), value);
                    let new_account_state = match (should_fail, account_state.add(value)) {
                        (false, Ok(account_state)) => account_state,
                        (true, Err(_)) => account_state,
                        (false,  Err(err)) => return TestResult::error(format!("Operation {}: unexpected add operation failure. Expected success but got: {:?}",counter,err)),
                        (true, Ok(account_state)) => return TestResult::error(format!("Operation {}: unexpected add operation success. Expected failure but got: success. AccountState: {:?}",counter, &account_state)),
                    };
                    new_account_state
                }
                ArbitraryAccountStateOp::Sub(value) => {
                    let should_fail = should_sub_fail(account_state.clone(), value);
                    let new_account_state = match (should_fail, account_state.sub(value)) {
                        (false, Ok(account_state)) => {
                            successful_subs = successful_subs + 1;
                            // check if account has any funds left
                            match account_state {
                                Some(account_state) => account_state,
                                None => return verify_account_lost_all_funds(initial_account_state.clone(),operations.clone(),counter,successful_subs,account_state.unwrap())
                            }
                        }
                        (true, Err(_)) => account_state,
                        (false,  Err(err)) => return TestResult::error(format!("Operation {}: unexpected sub operation failure. Expected success but got: {:?}",counter,err)),
                        (true, Ok(account_state)) => return TestResult::error(format!("Operation {}: unexpected sub operation success. Expected failure but got: success. AccountState: {:?}",counter, &account_state)),
                    };
                    new_account_state
                }
                ArbitraryAccountStateOp::Delegate(stake_pool_id) => {
                    account_state.set_delegation(DelegationType::Full(stake_pool_id))
                }
                ArbitraryAccountStateOp::RemoveDelegation => {
                    account_state.set_delegation(DelegationType::NonDelegated)
                }
            };
        }
        let expected_account_state =
            operations.get_account_state(initial_account_state.clone(), successful_subs);
        match expected_account_state == account_state {
            true => TestResult::passed(),
            false => TestResult::error(format!(
                "Actual AccountState is not equal to expected one. Expected {:?}, but got {:?}",
                expected_account_state, account_state
            )),
        }
    }

    fn verify_account_lost_all_funds(
        initial_account_state: AccountState<()>,
        operations: ArbitraryOperationChain,
        counter: usize,
        subs: u32,
        actual_account_state: AccountState<()>,
    ) -> TestResult {
        let expected_account =
            operations.get_account_state_after_n_ops(initial_account_state, counter, subs);
        match expected_account.value == Value::zero() {
              true => TestResult::passed(),
              false => TestResult::error(format!("Account is dry out from funds after {} operations, while expectation was different. Expected: {:?}, Actual {:?}",counter,expected_account,actual_account_state))
          }
    }

    fn should_add_fail(account_state: AccountState<()>, value: Value) -> bool {
        (value + account_state.get_value()).is_err()
    }

    fn should_sub_fail(account_state: AccountState<()>, value: Value) -> bool {
        // should fail if we recieve negative result
        // or if we reached counter limit and it's now full withdrawal
        (account_state.get_value() - value).is_err()
            || (account_state.counter.0.checked_add(1).is_none()
                && account_state.get_value() != value)
    }
}
