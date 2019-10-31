use super::{Block0Error, Error};
use crate::certificate;
use crate::transaction::*;
use crate::value::Value;
use chain_addr::Address;

macro_rules! if_cond_fail_with(
    ($cond: expr, $err: expr) => {
        if $cond {
            Err($err)
        } else {
            Ok(())
        }
    };
);

type LedgerCheck = Result<(), Error>;

// Check that a specific block0 transaction has no inputs and no witnesses
pub(super) fn valid_block0_transaction_no_inputs<'a, Extra>(
    tx: &TransactionSlice<'a, Extra>,
) -> LedgerCheck {
    if_cond_fail_with!(
        tx.nb_inputs() != 0,
        Error::Block0 {
            source: Block0Error::TransactionHasInput
        }
    )?;
    if_cond_fail_with!(
        tx.nb_inputs() != 0,
        Error::Block0 {
            source: Block0Error::TransactionHasWitnesses
        }
    )
}

// Check that a specific block0 transaction has no outputs
pub(super) fn valid_block0_transaction_no_outputs<'a, Extra>(
    tx: &TransactionSlice<'a, Extra>,
) -> LedgerCheck {
    if_cond_fail_with!(
        tx.nb_outputs() != 0,
        Error::Block0 {
            source: Block0Error::TransactionHasOutput
        }
    )
}

/// Check that the output value is valid
pub(super) fn valid_output_value(output: &Output<Address>) -> LedgerCheck {
    if_cond_fail_with!(
        output.value == Value::zero(),
        Error::ZeroOutput {
            output: output.clone()
        }
    )
}

/// check that the transaction input/outputs/witnesses is valid for stake_owner_delegation
pub(super) fn valid_stake_owner_delegation_transaction<'a>(
    auth_cert: &TransactionSlice<'a, certificate::OwnerStakeDelegation>,
) -> LedgerCheck {
    if_cond_fail_with!(
        auth_cert.inputs().nb_inputs() != 1
            || auth_cert.witnesses().nb_witnesses() != 1
            || auth_cert.outputs().nb_outputs() != 0,
        Error::OwnerStakeDelegationInvalidTransaction
    )
}

pub(super) fn valid_pool_registration_certificate(
    auth_cert: &certificate::PoolRegistration,
) -> LedgerCheck {
    if_cond_fail_with!(
        auth_cert.management_threshold == 0,
        Error::PoolRegistrationInvalid
    )?;
    if_cond_fail_with!(
        auth_cert.management_threshold as usize > auth_cert.owners.len(),
        Error::PoolRegistrationInvalid
    )?;
    if_cond_fail_with!(
        auth_cert.owners.len() >= 256,
        Error::PoolRegistrationInvalid
    )?;
    Ok(())
}

pub(super) fn valid_pool_owner_signature<T: ?Sized>(
    pos: &certificate::PoolOwnersSigned<T>,
) -> LedgerCheck {
    if_cond_fail_with!(
        pos.signatures.len() == 0,
        Error::CertificateInvalidSignature
    )?;
    if_cond_fail_with!(
        pos.signatures.len() > 255,
        Error::CertificateInvalidSignature
    )?;
    Ok(())
}

pub(super) fn valid_pool_retirement_certificate(_: &certificate::PoolRetirement) -> LedgerCheck {
    Ok(())
}

pub(super) fn valid_pool_update_certificate(_: &certificate::PoolUpdate) -> LedgerCheck {
    Ok(())
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub TxVerifyError
        TooManyOutputs {expected: u8, actual: u8 }
            = "too many outputs, expected maximum of {expected}, but received {actual}",
}

pub(super) fn valid_transaction_ios_number<'a, P>(
    tx: &TransactionSlice<'a, P>,
) -> Result<(), TxVerifyError> {
    if tx.nb_outputs() >= 255 {
        return Err(TxVerifyError::TooManyOutputs {
            expected: 254,
            actual: tx.nb_outputs(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    fn test_valid_block0_transaction_no_inputs_for<P: Payload>(tx: Transaction<P>) -> TestResult {
        let has_valid_inputs = tx.nb_inputs() == 0 && tx.nb_witnesses() == 0;
        let result = valid_block0_transaction_no_inputs(&tx.as_slice());
        to_quickchek_result(result, has_valid_inputs)
    }

    #[quickcheck]
    pub fn test_valid_block0_transaction_no_inputs(
        tx: Transaction<certificate::OwnerStakeDelegation>,
    ) -> TestResult {
        test_valid_block0_transaction_no_inputs_for(tx)
    }

    #[quickcheck]
    pub fn test_valid_block0_transaction_outputs(
        tx: Transaction<certificate::OwnerStakeDelegation>,
    ) -> TestResult {
        let has_valid_outputs = tx.nb_outputs() == 0;

        let result = valid_block0_transaction_no_outputs(&tx.as_slice());
        to_quickchek_result(result, has_valid_outputs)
    }

    #[quickcheck]
    pub fn test_valid_output_value(output: Output<Address>) -> TestResult {
        let is_valid_output = output.value != Value::zero();
        let result = valid_output_value(&output);
        to_quickchek_result(result, is_valid_output)
    }

    #[quickcheck]
    pub fn test_valid_pool_registration_certificate(
        pool_registration: certificate::PoolRegistration,
    ) -> TestResult {
        let is_valid = pool_registration.management_threshold != 0
            && (pool_registration.management_threshold as usize) <= pool_registration.owners.len()
            && pool_registration.owners.len() < 256;
        let result = valid_pool_registration_certificate(&pool_registration);
        to_quickchek_result(result, is_valid)
    }

    #[quickcheck]
    pub fn test_valid_stake_owner_delegation_transaction(
        tx: Transaction<certificate::OwnerStakeDelegation>,
    ) -> TestResult {
        let is_valid = tx.nb_witnesses() == 1 && tx.nb_inputs() == 1 && tx.nb_outputs() == 0;
        let result = valid_stake_owner_delegation_transaction(&tx.as_slice());
        to_quickchek_result(result, is_valid)
    }

    /*
    #[quickcheck]
    pub fn test_valid_pool_retirement_certificate(
        cert: certificate::PoolOwnersSigned<T>,
    ) -> TestResult {
        let is_valid = cert.signatures.len() > 0 && cert.signatures.len() < 256;
        let result = valid_pool_retirement_certificate(&cert);
        to_quickchek_result(result, is_valid)
    }
    #[quickcheck]
    pub fn test_valid_pool_update_certificate(
        cert: certificate::PoolOwnersSigned<certificate::PoolUpdate>,
    ) -> TestResult {
        let is_valid = cert.signatures.len() > 0 && cert.signatures.len() < 256;
        let result = valid_pool_update_certificate(&cert);
        to_quickchek_result(result, is_valid)
    }
    */

    fn to_quickchek_result(result: LedgerCheck, should_succeed: bool) -> TestResult {
        match (result, should_succeed) {
            (Ok(_), true) => TestResult::passed(),
            (Ok(_), false) => TestResult::failed(),
            (Err(_), true) => TestResult::failed(),
            (Err(_), false) => TestResult::passed(),
        }
    }
}
