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

pub(super) fn valid_pool_retirement_certificate(
    _auth_cert: &certificate::PoolOwnersSigned<certificate::PoolRetirement>
) -> LedgerCheck {
    Ok(())
}

pub(super) fn valid_pool_update_certificate(
    _auth_cert: &certificate::PoolOwnersSigned<certificate::PoolUpdate>
) -> LedgerCheck {
    Ok(())
}
