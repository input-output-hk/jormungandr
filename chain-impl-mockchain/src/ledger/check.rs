use super::Error;
use crate::certificate;
use crate::transaction::*;
use chain_addr::Address;

/// check that the transaction input/outputs/witnesses is valid for stake_owner_delegation
pub(super) fn valid_stake_owner_delegation_transaction(
    auth_cert: &AuthenticatedTransaction<Address, certificate::OwnerStakeDelegation>,
) -> Result<(), Error> {
    if auth_cert.transaction.inputs.len() != 1
        || auth_cert.witnesses.len() != 1
        || auth_cert.transaction.outputs.len() != 0
    {
        return Err(Error::OwnerStakeDelegationInvalidTransaction);
    }
    Ok(())
}
