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
pub(super) fn valid_block0_transaction_no_inputs<Extra>(
    tx: &AuthenticatedTransaction<Address, Extra>,
) -> LedgerCheck {
    if_cond_fail_with!(
        tx.transaction.inputs.len() != 0,
        Error::Block0 {
            source: Block0Error::TransactionHasInput
        }
    )?;
    if_cond_fail_with!(
        tx.witnesses.len() != 0,
        Error::Block0 {
            source: Block0Error::TransactionHasWitnesses
        }
    )
}

// Check that a specific block0 transaction has no outputs
pub(super) fn valid_block0_transaction_no_outputs<Extra>(
    tx: &AuthenticatedTransaction<Address, Extra>,
) -> LedgerCheck {
    if_cond_fail_with!(
        tx.transaction.outputs.len() != 0,
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
pub(super) fn valid_stake_owner_delegation_transaction(
    auth_cert: &AuthenticatedTransaction<Address, certificate::OwnerStakeDelegation>,
) -> LedgerCheck {
    if_cond_fail_with!(
        auth_cert.transaction.inputs.len() != 1
            || auth_cert.witnesses.len() != 1
            || auth_cert.transaction.outputs.len() != 0,
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

pub(super) fn valid_pool_retirement_certificate(
    cert: &certificate::PoolOwnersSigned<certificate::PoolRetirement>,
) -> LedgerCheck {
    if_cond_fail_with!(
        cert.signatures.len() == 0,
        Error::CertificateInvalidSignature
    )?;
    if_cond_fail_with!(
        cert.signatures.len() > 255,
        Error::CertificateInvalidSignature
    )?;
    Ok(())
}

pub(super) fn valid_pool_update_certificate(
    cert: &certificate::PoolOwnersSigned<certificate::PoolUpdate>,
) -> LedgerCheck {
    if_cond_fail_with!(
        cert.signatures.len() == 0,
        Error::CertificateInvalidSignature
    )?;
    if_cond_fail_with!(
        cert.signatures.len() > 255,
        Error::CertificateInvalidSignature
    )?;
    Ok(())
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub TxVerifyError
        TooManyInputs {expected: usize, actual: usize }
            = "too many inputs, expected maximum of {expected}, but received {actual}",
        TooManyOutputs {expected: usize, actual: usize }
            = "too many outputs, expected maximum of {expected}, but received {actual}",
        TooManyWitnesses {expected: usize, actual: usize }
            = "too many witnesses, expected maximum of {expected}, but received {actual}",
        NumberOfSignaturesInvalid { expected: usize, actual: usize }
            = "invalid number of signatures, expected {expected}, but received {actual}",
}

pub struct TxVerifyLimits {
    pub max_inputs_count: usize,
    pub max_outputs_count: usize,
    pub max_witnesses_count: usize,
}

impl<OutAddress, Extra> AuthenticatedTransaction<OutAddress, Extra> {
    pub fn verify_well_formed(&self, limits: &TxVerifyLimits) -> Result<(), TxVerifyError> {
        let inputs = &self.transaction.inputs;
        if inputs.len() > limits.max_inputs_count {
            return Err(TxVerifyError::TooManyInputs {
                expected: limits.max_inputs_count,
                actual: inputs.len(),
            });
        }

        let outputs = &self.transaction.outputs;
        if outputs.len() > limits.max_outputs_count {
            return Err(TxVerifyError::TooManyOutputs {
                expected: limits.max_outputs_count,
                actual: outputs.len(),
            });
        }

        let witnesses = &self.witnesses;
        if witnesses.len() > limits.max_witnesses_count {
            return Err(TxVerifyError::TooManyWitnesses {
                expected: limits.max_witnesses_count,
                actual: witnesses.len(),
            });
        }

        if inputs.len() != witnesses.len() {
            return Err(TxVerifyError::NumberOfSignaturesInvalid {
                expected: inputs.len(),
                actual: witnesses.len(),
            });
        }

        Ok(())
    }
}
