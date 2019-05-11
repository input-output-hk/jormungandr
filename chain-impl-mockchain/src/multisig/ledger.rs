use imhamt::{Hamt, InsertError, RemoveError};
use std::collections::hash_map::DefaultHasher;

use super::declaration::{Declaration, DeclarationError, Identifier};
use crate::accounting::account::{self, SpendingCounter};
use crate::value::{Value, ValueError};

#[derive(Clone)]
pub struct Ledger {
    // TODO : investigate about merging the declarations and the accounts in
    // one with an extension on the account::Ledger
    accounts: account::Ledger<Identifier>,
    declarations: Hamt<DefaultHasher, Identifier, Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    ParticipantOutOfBound,
    AlreadyExist,
    DoesntExist,
    DeclarationError(DeclarationError),
    AccountError(account::LedgerError),
    IdentifierMismatch,
    ThresholdNotMet,
}

impl From<account::LedgerError> for LedgerError {
    fn from(a: account::LedgerError) -> Self {
        LedgerError::AccountError(a)
    }
}

impl From<InsertError> for LedgerError {
    fn from(_: InsertError) -> Self {
        LedgerError::AlreadyExist
    }
}

impl From<DeclarationError> for LedgerError {
    fn from(e: DeclarationError) -> Self {
        LedgerError::DeclarationError(e)
    }
}

impl From<RemoveError> for LedgerError {
    fn from(_: RemoveError) -> Self {
        LedgerError::DoesntExist
    }
}

impl Ledger {
    /// Create a new empty account ledger
    pub fn new() -> Self {
        Ledger {
            accounts: account::Ledger::new(),
            declarations: Hamt::new(),
        }
    }

    /// Add a new multisig declaration into the ledger.
    ///
    /// If the identifier is already present, error out.
    pub fn add_account(&self, declaration: &Declaration) -> Result<Self, LedgerError> {
        // check if declaration is valid here
        declaration.is_valid()?;

        let identifier = declaration.to_identifier();
        let new_decls = self
            .declarations
            .insert(identifier.clone(), declaration.clone())?;
        let new_accts = self.accounts.add_account(&identifier, Value::zero())?;
        Ok(Self {
            accounts: new_accts,
            declarations: new_decls,
        })
    }

    /// Remove a declaration from this ledger
    pub fn remove_account(&self, ident: &Identifier) -> Result<Self, LedgerError> {
        let new_decls = self.declarations.remove(ident)?;
        let new_accts = self.accounts.remove_account(ident)?;
        Ok(Self {
            accounts: new_accts,
            declarations: new_decls,
        })
    }

    pub fn add_value(&self, identifier: &Identifier, value: Value) -> Result<Self, LedgerError> {
        let new_accounts = self.accounts.add_value(identifier, value)?;
        Ok(Self {
            accounts: new_accounts,
            declarations: self.declarations.clone(),
        })
    }

    /// If the account doesn't exist, or that the value would become negative, errors out.
    pub fn remove_value(
        &self,
        identifier: &Identifier,
        value: Value,
    ) -> Result<(Self, &Declaration, SpendingCounter), LedgerError> {
        let decl = self
            .declarations
            .lookup(identifier)
            .ok_or(LedgerError::DoesntExist)?;
        let (new_accts, spending_counter) = self.accounts.remove_value(identifier, value)?;
        Ok((
            Self {
                accounts: new_accts,
                declarations: self.declarations.clone(),
            },
            decl,
            spending_counter,
        ))
    }

    pub fn get_total_value(&self) -> Result<Value, ValueError> {
        self.accounts.get_total_value()
    }
}
