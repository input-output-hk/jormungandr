use super::{
    chain_storable::{AccountId, Address, BlockId, FragmentId, PoolId, Stake},
    endian::L64,
    helpers::find_last_record_by,
    pair::Pair,
    SanakirjaMutTx, SeqNum,
};
use super::{
    chain_storable::{ChainLength, ProposalId},
    error::DbError,
};
use chain_impl_mockchain::{transaction, value::Value};
use sanakirja::{
    btree::{self, Db},
    direct_repr, Storable, UnsizedStorable,
};
use std::convert::TryInto;

pub type StakeControl = Db<AccountId, Stake>;
pub type BlocksInBranch = Db<ChainLength, BlockId>;

pub type AddressId = SeqNum;
pub type AddressIds = Db<Address, AddressId>;
pub type AddressTransactions = Db<AddressId, Pair<SeqNum, FragmentId>>;
pub type Votes = Db<ProposalId, Pair<SeqNum, FragmentId>>;

// a typed (and in-memory) version of SerializedStateRef
pub struct StateRef {
    pub stake_control: StakeControl,
    pub blocks: BlocksInBranch,
    pub address_id: AddressIds,
    pub address_transactions: AddressTransactions,
    pub votes: Votes,
    // cached field, this gets written back by `finish`
    next_address_id: Option<SeqNum>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct SerializedStateRef {
    pub stake_control: L64,
    pub blocks: L64,
    pub address_id: L64,
    pub addresses: L64,
    pub votes: L64,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PoolIdEntry {
    pool: PoolId,
    seq: SeqNum,
}

direct_repr!(PoolIdEntry);

impl From<SerializedStateRef> for StateRef {
    fn from(ser: SerializedStateRef) -> Self {
        StateRef {
            stake_control: Db::from_page(ser.stake_control.get()),
            blocks: Db::from_page(ser.blocks.get()),
            address_id: Db::from_page(ser.address_id.get()),
            address_transactions: Db::from_page(ser.addresses.get()),
            votes: Db::from_page(ser.votes.get()),
            next_address_id: None,
        }
    }
}

impl StateRef {
    pub fn new_empty<T>(txn: &mut T) -> Result<Self, DbError>
    where
        T: ::sanakirja::AllocPage
            + ::sanakirja::LoadPage<Error = ::sanakirja::Error>
            + ::sanakirja::RootPage,
    {
        let mut empty = Self {
            stake_control: btree::create_db_(txn).unwrap(),
            blocks: btree::create_db_(txn).unwrap(),
            address_id: btree::create_db_(txn).unwrap(),
            address_transactions: btree::create_db_(txn).unwrap(),
            votes: btree::create_db_(txn).unwrap(),

            next_address_id: None,
        };

        // TODO: extract [0u8; 65] to an Address::sigil function
        btree::put(
            txn,
            &mut empty.address_id,
            &Address([0u8; 65]),
            &SeqNum::new(0),
        )?;

        Ok(empty)
    }

    pub fn finish(mut self, txn: &mut SanakirjaMutTx) -> Result<SerializedStateRef, DbError> {
        // if the sequence counter for addresses was incremented previously, rewrite it
        if let Some(next_seq) = self.next_address_id {
            btree::del(txn, &mut self.address_id, &Address([0u8; 65]), None)?;

            debug_assert!(btree::put(
                txn,
                &mut self.address_id,
                &Address([0u8; 65]),
                &next_seq.next(),
            )?);
        }

        Ok(SerializedStateRef {
            stake_control: L64::new(self.stake_control.db),
            blocks: L64::new(self.blocks.db),
            address_id: L64::new(self.address_id.db),
            addresses: L64::new(self.address_transactions.db),
            votes: L64::new(self.votes.db),
        })
    }

    pub fn apply_vote(
        &mut self,
        txn: &mut SanakirjaMutTx,
        fragment_id: &FragmentId,
        proposal_id: &ProposalId,
    ) -> Result<(), DbError> {
        let max_possible_value = Pair {
            a: SeqNum::MAX,
            b: FragmentId::MAX,
        };

        let seq = find_last_record_by(txn, &self.votes, &proposal_id, &max_possible_value)
            .map(|last| last.a.next())
            .unwrap_or(SeqNum::MIN);

        btree::put(
            txn,
            &mut self.votes,
            &proposal_id,
            &Pair {
                a: seq,
                b: fragment_id.clone(),
            },
        )?;

        Ok(())
    }

    /// Add the given transaction to address at the end of the list
    /// This function *only* checks the last fragment to avoid repetition when a transaction has more
    /// than one (input|output) with the same address (eg: utxo input and change).
    pub fn add_transaction_to_address(
        &mut self,
        txn: &mut SanakirjaMutTx,
        fragment_id: &FragmentId,
        address: &Address,
    ) -> Result<(), DbError> {
        let address_id = self.get_or_insert_address_id(txn, address)?;

        let max_possible_value = Pair {
            a: SeqNum::MAX,
            b: FragmentId::MAX,
        };

        let seq = match find_last_record_by(
            &*txn,
            &self.address_transactions,
            &address_id,
            &max_possible_value,
        ) {
            Some(v) => {
                if &v.b == fragment_id {
                    return Ok(());
                } else {
                    v.a.next()
                }
            }
            None => SeqNum::MIN,
        };

        debug_assert!(btree::put(
            txn,
            &mut self.address_transactions,
            &address_id,
            &Pair {
                a: seq,
                b: fragment_id.clone(),
            },
        )?);

        Ok(())
    }

    pub fn add_block_to_blocks(
        &mut self,
        txn: &mut SanakirjaMutTx,
        chain_length: &ChainLength,
        block_id: &BlockId,
    ) -> Result<(), DbError> {
        btree::put(txn, &mut self.blocks, chain_length, block_id).unwrap();
        Ok(())
    }

    pub(crate) fn get_or_insert_address_id(
        &mut self,
        txn: &mut SanakirjaMutTx,
        address: &Address,
    ) -> Result<SeqNum, DbError> {
        let address_exists = btree::get(txn, &self.address_id, address, None)?
            .filter(|(id, _)| id == &address)
            .map(|(_, v)| v)
            .cloned();

        let address_id = if let Some(v) = address_exists {
            v
        } else {
            let next_seq = if let Some(next_seq) = self.next_address_id {
                next_seq
            } else {
                *btree::get(txn, &self.address_id, &Address([0u8; 65]), None)?
                    .unwrap()
                    .1
            };

            self.next_address_id = Some(next_seq.next());

            btree::put(txn, &mut self.address_id, address, &next_seq)?;

            next_seq
        };

        Ok(address_id)
    }

    pub fn apply_output_to_stake_control(
        &mut self,
        txn: &mut SanakirjaMutTx,
        output: &transaction::Output<chain_addr::Address>,
    ) -> Result<(), DbError> {
        match output.address.kind() {
            chain_addr::Kind::Group(_, account) => {
                self.add_stake_to_account(txn, account, output.value)?;
            }
            chain_addr::Kind::Account(account) => {
                self.add_stake_to_account(txn, account, output.value)?;
            }
            chain_addr::Kind::Single(_account) => {}
            chain_addr::Kind::Multisig(_) => {}
            chain_addr::Kind::Script(_) => {}
        }
        Ok(())
    }

    pub fn add_stake_to_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        value: Value,
    ) -> Result<(), DbError> {
        let op =
            |current_stake: u64, value: u64| -> u64 { current_stake.checked_add(value).unwrap() };

        self.update_stake_for_account(txn, account, op, value)
    }

    pub fn substract_stake_from_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        value: Value,
    ) -> Result<(), DbError> {
        let op =
            |current_stake: u64, value: u64| -> u64 { current_stake.checked_sub(value).unwrap() };

        self.update_stake_for_account(txn, account, op, value)
    }

    fn update_stake_for_account(
        &mut self,
        txn: &mut SanakirjaMutTx,
        account: &chain_crypto::PublicKey<chain_crypto::Ed25519>,
        op: impl Fn(u64, u64) -> u64,
        value: Value,
    ) -> Result<(), DbError> {
        let account_id = AccountId(account.as_ref().try_into().unwrap());

        let current_stake = btree::get(txn, &self.stake_control, &account_id, None)
            .unwrap()
            .and_then(|(k, stake)| {
                if k == &account_id {
                    Some(stake.get())
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let new_stake = op(current_stake, value.0);

        btree::del(txn, &mut self.stake_control, &account_id, None).unwrap();
        btree::put(
            txn,
            &mut self.stake_control,
            &account_id,
            &L64::new(new_stake),
        )?;

        Ok(())
    }

    /// gc this fork so the allocated pages can be re-used
    ///
    /// # Safety
    ///
    /// It's important that any references to this particular state are not used anymore. For the
    /// current use-case, callers need to ensure that this snapshot is not referenced anymore in
    /// the `States` btree.
    pub unsafe fn drop(self, txn: &mut SanakirjaMutTx) -> Result<(), DbError> {
        let StateRef {
            stake_control,
            blocks,
            address_transactions,
            address_id,
            votes,
            next_address_id: _,
        } = self;

        btree::drop(txn, stake_control)?;
        btree::drop(txn, blocks)?;
        btree::drop(txn, address_id)?;
        btree::drop(txn, address_transactions)?;
        btree::drop(txn, votes)?;

        Ok(())
    }
}

impl SerializedStateRef {
    pub fn fork(&self, txn: &mut SanakirjaMutTx) -> Result<StateRef, DbError> {
        Ok(StateRef {
            stake_control: btree::fork_db(txn, &Db::from_page(self.stake_control.get()))?,
            blocks: btree::fork_db(txn, &Db::from_page(self.blocks.get()))?,
            address_id: btree::fork_db(txn, &Db::from_page(self.address_id.get()))?,
            address_transactions: btree::fork_db(txn, &Db::from_page(self.addresses.get()))?,
            votes: btree::fork_db(txn, &Db::from_page(self.votes.get()))?,
            next_address_id: None,
        })
    }
}

direct_repr!(SerializedStateRef);
